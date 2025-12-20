mod audio;
mod renderer;
mod utils;

use audio::{AudioAnalyzer, OutputCapture, SourcePipe};
use nannou::prelude::*;
use renderer::{FeedbackRenderer, Renderer, Resolution};
use std::cell::RefCell;
use std::env;

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
}

fn model(app: &App) -> Model {
    let resolution = Resolution::current();
    app.set_exit_on_escape(false);

    let mut win = app
        .new_window()
        .view(view)
        .key_pressed(key_pressed)
        .resized(resized)
        .size(resolution.width, resolution.height);

    if resolution.fullscreen {
        win = win.fullscreen();
    }

    let window_id = win.build().unwrap();

    // Get window for wgpu resources
    let window = app.window(window_id).unwrap();
    let device = window.device();
    let queue = window.queue();
    let size = window.inner_size_pixels();
    let sample_count = window.msaa_samples();

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

    Model {
        source: SourcePipe::new(),
        analyzer: AudioAnalyzer::new(),
        renderer: Renderer::with_cycling(),
        output_capture: OutputCapture::new(),
        feedback: RefCell::new(feedback),
        screensaver_inhibitor,
        phase_offset: 0.0,
        prev_energy: 0.0,
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let samples = model.source.stream();

    // Analyze audio (single FFT for all visualizations)
    let analysis = model.analyzer.analyze(&samples);

    // Debug: print energy every second
    if app.elapsed_frames().is_multiple_of(60) {
        println!("Energy: {:.2} | Bass: {:.2} | Mids: {:.2} | Treble: {:.2}",
            analysis.energy, analysis.bass, analysis.mids, analysis.treble);
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
    if model.output_capture.search_active {
        let search_draw = app.draw();
        draw_search_overlay(&search_draw, bounds, &model.output_capture);
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

fn draw_search_overlay(draw: &Draw, bounds: Rect, capture: &OutputCapture) {
    let padding = 20.0;
    let line_height = 22.0;
    let font_size = 18;
    let max_visible = 15;

    // Calculate overlay dimensions - almost full screen width
    let overlay_width = bounds.w() - padding * 2.0;
    let visible_count = capture.filtered.len().min(max_visible);
    let overlay_height = line_height * (visible_count as f32 + 2.0) + padding * 2.0;

    // Position at top (centered horizontally)
    let overlay_x = 0.0;
    let overlay_y = bounds.top() - overlay_height / 2.0 - padding;

    // Semi-transparent background
    draw.rect()
        .x_y(overlay_x, overlay_y)
        .w_h(overlay_width, overlay_height)
        .color(rgba(0.0, 0.0, 0.0, 0.85));

    // Text box centered on screen, text left-justified within it
    let text_box_width = bounds.w() - padding * 2.0;
    let text_box_x = 0.0; // Center of screen

    // Search query line
    let query_y = overlay_y + overlay_height / 2.0 - padding - line_height / 2.0;
    let query_text = format!("Search: {}_", capture.query);
    draw.text(&query_text)
        .xy(pt2(text_box_x, query_y))
        .wh(pt2(text_box_width, line_height))
        .left_justify()
        .no_line_wrap()
        .color(rgb(1.0, 1.0, 1.0))
        .font_size(font_size);

    // Separator line
    let sep_y = query_y - line_height;
    draw.line()
        .start(pt2(bounds.left() + padding, sep_y))
        .end(pt2(bounds.right() - padding, sep_y))
        .color(rgba(1.0, 1.0, 1.0, 0.3))
        .weight(1.0);

    // List filtered results
    for (i, port) in capture.filtered.iter().take(max_visible).enumerate() {
        let item_y = sep_y - line_height * (i as f32 + 1.0);
        let is_selected = i == capture.selected_idx;

        let prefix = if is_selected { "> " } else { "  " };
        let text = format!("{}{}", prefix, port.display());

        let color = if is_selected {
            rgb(0.3, 0.8, 1.0) // Highlight color
        } else {
            rgb(1.0, 1.0, 1.0) // White
        };

        draw.text(&text)
            .xy(pt2(text_box_x, item_y))
            .wh(pt2(text_box_width, line_height))
            .left_justify()
            .no_line_wrap()
            .color(color)
            .font_size(font_size);
    }

    // Show "..." if there are more results
    if capture.filtered.len() > max_visible {
        let more_y = sep_y - line_height * (max_visible as f32 + 1.0);
        let more_text = format!("  ... and {} more", capture.filtered.len() - max_visible);
        draw.text(&more_text)
            .xy(pt2(text_box_x, more_y))
            .wh(pt2(text_box_width, line_height))
            .left_justify()
            .no_line_wrap()
            .color(rgba(1.0, 1.0, 1.0, 0.5))
            .font_size(font_size);
    }
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    // Exit app with Q only
    if key == Key::Q {
        app.quit();
        return;
    }

    // Search mode handling
    if model.output_capture.search_active {
        match key {
            Key::Escape => {
                model.output_capture.cancel();
            }
            Key::Up => {
                model.output_capture.move_up();
            }
            Key::Down => {
                model.output_capture.move_down();
            }
            Key::Back => {
                model.output_capture.backspace();
            }
            Key::Return => {
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
            // Alphanumeric keys for filtering
            _ => {
                if let Some(c) = key_to_char(key, app.keys.mods.shift()) {
                    model.output_capture.append_char(c);
                }
            }
        }
        return;
    }

    // Normal mode: start search with /
    if key == Key::Slash {
        model.output_capture.start_search();
        return;
    }

    // Toggle debug visualization with 'd'
    if key == Key::D {
        model.renderer.toggle_debug_viz();
        return;
    }

    // Space cycles visualizations and unlocks auto-cycling
    if key == Key::Space {
        model.renderer.cycle_next();
        return;
    }

    // Number keys for visualization selection (locks to that visualization)
    let shift_offset = if app.keys.mods.shift() { 10 } else { 0 };

    let index = match key {
        Key::Key0 => Some(shift_offset),
        Key::Key1 => Some(1 + shift_offset),
        Key::Key2 => Some(2 + shift_offset),
        Key::Key3 => Some(3 + shift_offset),
        Key::Key4 => Some(4 + shift_offset),
        Key::Key5 => Some(5 + shift_offset),
        Key::Key6 => Some(6 + shift_offset),
        Key::Key7 => Some(7 + shift_offset),
        Key::Key8 => Some(8 + shift_offset),
        Key::Key9 => Some(9 + shift_offset),
        _ => None,
    };

    if let Some(idx) = index {
        if let Some(name) = model.renderer.set_visualization(idx) {
            model.renderer.show_notification(format!("[{}] {}", idx, name));
        }
    }
}

/// Convert a Key to a character (alphanumeric only)
fn key_to_char(key: Key, shift: bool) -> Option<char> {
    let c = match key {
        Key::A => 'a',
        Key::B => 'b',
        Key::C => 'c',
        Key::D => 'd',
        Key::E => 'e',
        Key::F => 'f',
        Key::G => 'g',
        Key::H => 'h',
        Key::I => 'i',
        Key::J => 'j',
        Key::K => 'k',
        Key::L => 'l',
        Key::M => 'm',
        Key::N => 'n',
        Key::O => 'o',
        Key::P => 'p',
        Key::Q => 'q',
        Key::R => 'r',
        Key::S => 's',
        Key::T => 't',
        Key::U => 'u',
        Key::V => 'v',
        Key::W => 'w',
        Key::X => 'x',
        Key::Y => 'y',
        Key::Z => 'z',
        Key::Key0 => '0',
        Key::Key1 => '1',
        Key::Key2 => '2',
        Key::Key3 => '3',
        Key::Key4 => '4',
        Key::Key5 => '5',
        Key::Key6 => '6',
        Key::Key7 => '7',
        Key::Key8 => '8',
        Key::Key9 => '9',
        Key::Minus => '-',
        Key::Period => '.',
        Key::Underline => '_',
        _ => return None,
    };

    Some(if shift && c.is_alphabetic() {
        c.to_ascii_uppercase()
    } else {
        c
    })
}
