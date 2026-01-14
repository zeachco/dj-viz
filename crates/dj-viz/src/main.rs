mod audio;
mod plugin_loader;
mod renderer;
mod ui;
mod utils;

use audio::{AudioAnalysis, AudioAnalyzer, OutputCapture, SourcePipe};
use nannou::prelude::*;
use nannou::winit::event::WindowEvent;
use renderer::{FeedbackRenderer, Renderer, Resolution};
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use ui::bindings::{parse_key, Action};
use ui::help_overlay::HelpOverlay;
use ui::text_picker::{draw_text_picker, TextPickerState};
use ui::viz_picker::{draw_viz_picker, VizPicker};
use utils::Config;

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
    viz_picker: VizPicker,
    help_overlay: HelpOverlay,
    feedback: RefCell<FeedbackRenderer>,
    #[allow(dead_code)]
    screensaver_inhibitor: Option<utils::ScreensaverInhibitor>,
    phase_offset: f32,
    prev_energy: f32,
    last_analysis: AudioAnalysis,
    /// Track shift key state from raw events (more reliable than app.keys.mods)
    shift_held: bool,
    /// Manages Rhai scripted visualizations
    script_manager: ScriptManager,
}

fn model(app: &App) -> Model {
    let args: Vec<String> = env::args().collect();
    let windowed = args.contains(&"--windowed".to_string()) || args.contains(&"-w".to_string());
    let resolution = Resolution::current(windowed);
    app.set_exit_on_escape(false);

    let mut win = app
        .new_window()
        .view(view)
        .key_pressed(key_pressed)
        .mouse_pressed(mouse_pressed)
        .mouse_wheel(mouse_wheel)
        .raw_event(raw_event)
        .resized(resized)
        .size(resolution.width, resolution.height)
        .min_size(400, 400);

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

    // Initialize script manager with scripts directory
    let scripts_dir = PathBuf::from("scripts");
    let script_manager = ScriptManager::new(scripts_dir);

    let mut model = Model {
        source: SourcePipe::new(),
        analyzer: AudioAnalyzer::with_config(44100.0, detection_config.clone()),
        renderer: Renderer::with_cycling(detection_config, viz_energy_ranges),
        output_capture: OutputCapture::new(),
        viz_picker: VizPicker::new(),
        help_overlay: HelpOverlay::new(),
        feedback: RefCell::new(feedback),
        screensaver_inhibitor,
        phase_offset: 0.0,
        prev_energy: 0.0,
        last_analysis: AudioAnalysis::default(),
        shift_held: false,
        script_manager,
    };

    // Enable debug visualization if --debug or -d flag was passed
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

    // // Debug: print energy every second
    // if app.elapsed_frames().is_multiple_of(60) {
    //     println!(
    //         "Energy: {:.2} | Bass: {:.2} | Mids: {:.2} | Treble: {:.2}",
    //         analysis.energy, analysis.bass, analysis.mids, analysis.treble
    //     );
    // }

    // Update scripted visualization if active
    let bounds = app.window_rect();

    model.renderer.update(&analysis, bounds);
    let viz_info = model.renderer.viz_info();
    model.script_manager.update(&analysis, bounds, &viz_info);

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

    // If a script is active, render it directly (no feedback effects)
    if model.script_manager.is_active() {
        let script_draw = app.draw();
        model.script_manager.draw(&script_draw, bounds);
        script_draw.to_frame(app, &frame).unwrap();
    } else {
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

    // Draw viz picker overlay directly to frame
    if model.viz_picker.active {
        let picker_draw = app.draw();
        draw_viz_picker(&picker_draw, bounds, &model.viz_picker);
        picker_draw.to_frame(app, &frame).unwrap();
    }

    // Draw help overlay directly to frame
    if model.help_overlay.visible {
        let help_draw = app.draw();
        model
            .help_overlay
            .draw(&help_draw, bounds, model.renderer.is_locked());
        help_draw.to_frame(app, &frame).unwrap();
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
        model.viz_picker.active,
    );

    match action {
        Some(Action::Quit) => app.quit(),
        Some(Action::ShowHelp) => {
            model.help_overlay.toggle();
            model.viz_picker.hide(); // Close picker when showing help
        }

        // Search mode actions (audio device search)
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

        // Viz picker mode actions
        Some(Action::VizPickerShow) => {
            model.help_overlay.hide();
            model.viz_picker.update_active_states(
                model.renderer.current_idx(),
                model.renderer.overlay_indices(),
            );
            model.viz_picker.show();
        }
        Some(Action::VizPickerHide) => model.viz_picker.hide(),
        Some(Action::VizPickerMoveUp) => model.viz_picker.move_up(),
        Some(Action::VizPickerMoveDown) => model.viz_picker.move_down(),
        Some(Action::VizPickerSelect) => {
            if let Some(idx) = model.viz_picker.selected_viz_index() {
                model.script_manager.deactivate();
                if let Some(name) = model.renderer.set_visualization(idx) {
                    model
                        .renderer
                        .show_notification(format!("[{}] {}", idx, name));
                }
                model.viz_picker.update_active_states(
                    model.renderer.current_idx(),
                    model.renderer.overlay_indices(),
                );
            }
            model.viz_picker.hide();
        }
        Some(Action::VizPickerToggle) => {
            if let Some(idx) = model.viz_picker.selected_viz_index() {
                model.renderer.toggle_overlay(idx);
                model.viz_picker.update_active_states(
                    model.renderer.current_idx(),
                    model.renderer.overlay_indices(),
                );
            }
        }

        // Normal mode actions
        Some(Action::StartSearch) => model.output_capture.start_search(),
        Some(Action::ToggleDebugViz) => model.renderer.toggle_debug_viz(),
        Some(Action::ToggleLock) => {
            model.renderer.toggle_lock();
            let status = if model.renderer.is_locked() {
                "LOCKED"
            } else {
                "UNLOCKED"
            };
            model
                .renderer
                .show_notification(format!("Auto-cycling: {}", status));
        }
        Some(Action::CycleNext) => {
            model.script_manager.deactivate();
            model.renderer.cycle_next(&model.last_analysis);
        }
        Some(Action::CycleScript) => {
            if let Some(name) = model.script_manager.cycle_next() {
                model
                    .renderer
                    .show_notification(format!("Script: {}", name));
            } else {
                model
                    .renderer
                    .show_notification("No scripts found in scripts/".to_string());
            }
        }

        None => {} // Unhandled key
    }
}

fn mouse_pressed(_app: &App, model: &mut Model, button: MouseButton) {
    if !model.viz_picker.active {
        return;
    }

    match button {
        MouseButton::Left => {
            // Select the visualization
            if let Some(idx) = model.viz_picker.selected_viz_index() {
                model.script_manager.deactivate();
                if let Some(name) = model.renderer.set_visualization(idx) {
                    model
                        .renderer
                        .show_notification(format!("[{}] {}", idx, name));
                }
                model.viz_picker.update_active_states(
                    model.renderer.current_idx(),
                    model.renderer.overlay_indices(),
                );
            }
            model.viz_picker.hide();
        }
        MouseButton::Right => {
            // Toggle as overlay
            if let Some(idx) = model.viz_picker.selected_viz_index() {
                model.renderer.toggle_overlay(idx);
                model.viz_picker.update_active_states(
                    model.renderer.current_idx(),
                    model.renderer.overlay_indices(),
                );
            }
        }
        _ => {}
    }
}

fn mouse_wheel(_app: &App, model: &mut Model, delta: MouseScrollDelta, _phase: TouchPhase) {
    // Close help when scrolling
    model.help_overlay.hide();

    // If picker not active, show it first
    if !model.viz_picker.active {
        model.viz_picker.update_active_states(
            model.renderer.current_idx(),
            model.renderer.overlay_indices(),
        );
        model.viz_picker.show();
    }

    // Navigate based on scroll direction
    match delta {
        MouseScrollDelta::LineDelta(_, y) => {
            if y > 0.0 {
                model.viz_picker.move_up();
            } else if y < 0.0 {
                model.viz_picker.move_down();
            }
        }
        MouseScrollDelta::PixelDelta(pos) => {
            if pos.y > 10.0 {
                model.viz_picker.move_up();
            } else if pos.y < -10.0 {
                model.viz_picker.move_down();
            }
        }
    }
}
