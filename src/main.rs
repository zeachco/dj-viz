mod audio;
mod renderer;
mod ui;
mod utils;

use audio::{AudioAnalysis, AudioAnalyzer, OutputCapture, SourcePipe};
use nannou::prelude::*;
use renderer::{FeedbackRenderer, Renderer, Resolution};
use utils::Config;
use std::cell::RefCell;
use std::env;
use nannou::winit::event::WindowEvent;
use ui::bindings::{parse_key, Action};
use ui::text_picker::{draw_text_picker, TextPickerState};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--audio-info".to_string()) {
        utils::log_audio_info();
        return;
    }

    // List all devices at startup
    SourcePipe::list_devices();

    nannou::app(model).update(update).run();
}

struct Model {
    source: SourcePipe,
    analyzer: AudioAnalyzer,
    renderer: Renderer,
    output_capture: OutputCapture,
    feedback: RefCell<FeedbackRenderer>,
    #[allow(dead_code)]
    screensaver_inhibitor: Option<utils::ScreensaverInhibitor>,
    phase_offset: f32,
    prev_energy: f32,
    last_analysis: AudioAnalysis,
    /// Track shift key state from raw events (more reliable than app.keys.mods)
    shift_held: bool,
}

fn model(app: &App) -> Model {
    let resolution = Resolution::current();
    app.set_exit_on_escape(false);

    let mut win = app
        .new_window()
        .view(view)
        .key_pressed(key_pressed)
        .raw_event(raw_event)
        .resized(resized)
        .size(resolution.width, resolution.height);

    if resolution.fullscreen {
        win = win.fullscreen();
    }

    let window_id = win.build().unwrap();

    // Get window for wgpu resources
    let window = app.window(window_id).unwrap();

    // Hide cursor in fullscreen mode
    if resolution.fullscreen {
        window.set_cursor_visible(false);
    }
    let device = window.device();
    let queue = window.queue();
    let size = window.inner_size_pixels();
    let sample_count = window.msaa_samples();

    // Log actual window size vs requested size
    println!(
        "Window size: {}x{} (requested: {}x{})",
        size.0, size.1, resolution.width, resolution.height
    );

    // Create feedback renderer
    let feedback = FeedbackRenderer::new(
        device,
        queue,
        [size.0, size.1],
        sample_count,
        Frame::TEXTURE_FORMAT,
    );

    // Inhibit screensaver in release mode
    let screensaver_inhibitor = if !cfg!(debug_assertions) {
        utils::ScreensaverInhibitor::new()
    } else {
        None
    };

    // Load config and extract values for audio analyzer and renderer
    let config = Config::load();
    let detection_config = config.detection();
    let viz_energy_ranges = config.viz_energy_ranges();

    let mut model = Model {
        source: SourcePipe::new(),
        analyzer: AudioAnalyzer::with_config(44100.0, detection_config.clone()),
        renderer: Renderer::with_cycling(detection_config, viz_energy_ranges),
        output_capture: OutputCapture::new(),
        feedback: RefCell::new(feedback),
        screensaver_inhibitor,
        phase_offset: 0.0,
        prev_energy: 0.0,
        last_analysis: AudioAnalysis::default(),
        shift_held: false,
    };

    // Enable debug visualization if --debug or -d flag was passed
    let args: Vec<String> = env::args().collect();
    let debug_enabled = args.contains(&"--debug".to_string()) || args.contains(&"-d".to_string());
    if debug_enabled {
        model.renderer.toggle_debug_viz();
    }

    // Ensure feedback renderer is properly sized to actual window dimensions
    // This handles cases where OS window manager resizes the window
    if size.0 != resolution.width || size.1 != resolution.height {
        resized(app, &mut model, vec2(size.0 as f32, size.1 as f32));
    }

    model
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let samples = model.source.stream();

    // Analyze audio (single FFT for all visualizations)
    let analysis = model.analyzer.analyze(&samples);

    // Store for use in key handlers
    model.last_analysis = analysis.clone();

    // Debug: print energy every second
    if app.elapsed_frames().is_multiple_of(60) {
        println!(
            "Energy: {:.2} | Bass: {:.2} | Mids: {:.2} | Treble: {:.2}",
            analysis.energy, analysis.bass, analysis.mids, analysis.treble
        );
    }

    model.renderer.update(&analysis);

    // Detect energy peak and flip zoom direction
    if analysis.energy >= 0.95 && model.prev_energy < 0.95 {
        model.phase_offset += std::f32::consts::PI; // Add 180 degrees to reverse direction
    }
    model.prev_energy = analysis.energy;

    // Update feedback zoom based on beat intensity (bass + energy peaks)
    {
        let mut feedback = model.feedback.borrow_mut();
        // Sine wave oscillation over 30 seconds: zooms in and out
        let phase = app.time * std::f32::consts::TAU / 30.0 + model.phase_offset;
        let direction = phase.sin(); // -1 to 1
                                     // Base zoom follows sine wave
        let base_offset = 0.006 * direction;
        // Bass amplifies the current direction (zoom in faster or out faster)
        let bass_boost = analysis.bass * 0.012 * direction;
        feedback.scale = 1.0 + base_offset + bass_boost;
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let window = app.main_window();
    let device = window.device();
    let queue = window.queue();
    let bounds = app.window_rect();

    // Create draw context for primary visualization
    let primary_draw = app.draw();
    model.renderer.draw_primary(&primary_draw, bounds);

    // Create draw contexts for overlay visualizations
    let overlay_count = model.renderer.overlay_count();
    let overlay_draws: Vec<nannou::Draw> = (0..overlay_count).map(|_| app.draw()).collect();
    let overlay_draw_refs: Vec<&nannou::Draw> = overlay_draws.iter().collect();
    model.renderer.draw_overlays(&overlay_draw_refs, bounds);

    // Render through feedback buffer with burn blending and output to frame
    {
        let mut feedback = model.feedback.borrow_mut();
        feedback.render_with_overlays(
            device,
            queue,
            &primary_draw,
            &overlay_draw_refs,
            frame.texture_view(),
            Frame::TEXTURE_FORMAT,
            window.msaa_samples(),
        );
    }

    // Draw debug visualization directly to frame (not through feedback)
    let debug_draw = app.draw();
    model.renderer.draw_debug_viz(&debug_draw, bounds);
    debug_draw.to_frame(app, &frame).unwrap();

    // Draw notification overlay directly to frame (not through feedback)
    let notification_draw = app.draw();
    model.renderer.draw_notification(&notification_draw, bounds);
    notification_draw.to_frame(app, &frame).unwrap();

    // Draw search overlay directly to frame (not through feedback)
    if model.output_capture.is_active() {
        let search_draw = app.draw();
        draw_text_picker(&search_draw, bounds, &model.output_capture);
        search_draw.to_frame(app, &frame).unwrap();
    }
}

fn resized(app: &App, model: &mut Model, size: Vec2) {
    let window = app.main_window();
    let device = window.device();
    let sample_count = window.msaa_samples();

    model.feedback.borrow_mut().resize(
        device,
        [size.x as u32, size.y as u32],
        sample_count,
        Frame::TEXTURE_FORMAT,
    );
}

fn raw_event(_app: &App, model: &mut Model, event: &WindowEvent) {
    // Track shift key state from raw events for reliable modifier detection
    if let WindowEvent::ModifiersChanged(mods) = event {
        model.shift_held = mods.shift();
    }
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    let action = parse_key(
        key,
        app.keys.mods.shift(),
        model.output_capture.search_active,
    );

    match action {
        Some(Action::Quit) => app.quit(),

        // Search mode actions
        Some(Action::SearchCancel) => model.output_capture.cancel(),
        Some(Action::SearchMoveUp) => model.output_capture.move_up(),
        Some(Action::SearchMoveDown) => model.output_capture.move_down(),
        Some(Action::SearchBackspace) => model.output_capture.backspace(),
        Some(Action::SearchInput(c)) => model.output_capture.append_char(c),
        Some(Action::SearchConfirm) => {
            if let Some((name, idx)) = model.output_capture.select() {
                let msg = if let Some((_, success)) = model.source.select_device(idx) {
                    if success {
                        format!("[{}] {}", idx, name)
                    } else {
                        format!("[{}] {} - FAILED", idx, name)
                    }
                } else {
                    format!("[{}] {} - INVALID", idx, name)
                };
                model.renderer.show_notification(msg);
            }
            model.output_capture.cancel();
        }

        // Normal mode actions
        Some(Action::StartSearch) => model.output_capture.start_search(),
        Some(Action::ToggleDebugViz) => model.renderer.toggle_debug_viz(),
        Some(Action::CycleNext) => model.renderer.cycle_next(&model.last_analysis),
        Some(Action::SelectVisualization(idx)) => {
            if let Some(name) = model.renderer.set_visualization(idx) {
                model
                    .renderer
                    .show_notification(format!("[{}] {}", idx, name));
            }
        }

        None => {} // Unhandled key
    }
}
