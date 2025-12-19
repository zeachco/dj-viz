mod audio;
mod renderer;
mod utils;

use nannou::prelude::*;
use audio::{OutputCapture, SourcePipe};
use renderer::{Renderer, Resolution};
use utils::Config;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--audio-info".to_string()) {
        utils::log_audio_info();
        return;
    }

    // List all devices at startup
    SourcePipe::list_devices();

    nannou::app(model)
        .update(update)
        .run();
}

struct Model {
    source: SourcePipe,
    renderer: Renderer,
    output_capture: OutputCapture,
}

fn model(app: &App) -> Model {
    let resolution = Resolution::current();

    let mut win = app.new_window()
        .view(view)
        .key_pressed(key_pressed)
        .size(resolution.width, resolution.height);

    if resolution.fullscreen {
        win = win.fullscreen();
    }

    win.build().unwrap();

    Model {
        source: SourcePipe::new(),
        renderer: Renderer::with_cycling(),
        output_capture: OutputCapture::new(),
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let samples = model.source.stream();

    // Debug: print max sample value every second
    if app.elapsed_frames() % 60 == 0 {
        let max_sample = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        println!("Max sample: {:.6}", max_sample);
    }

    model.renderer.update(&samples);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let bounds = app.window_rect();

    model.renderer.draw(&draw, bounds);

    // Draw search overlay if active
    if model.output_capture.search_active {
        draw_search_overlay(&draw, bounds, &model.output_capture);
    }

    draw.to_frame(app, &frame).unwrap();
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

    // Text starts at left edge of overlay
    let text_left = bounds.left() + padding;

    // Search query line
    let query_y = overlay_y + overlay_height / 2.0 - padding - line_height / 2.0;
    let query_text = format!("Search: {}_", capture.query);
    draw.text(&query_text)
        .xy(pt2(text_left, query_y))
        .wh(pt2(bounds.w(), line_height).into())
        .left_justify()
        .no_line_wrap()
        .color(rgb(1.0, 1.0, 1.0))
        .font_size(font_size);

    // Separator line
    let sep_y = query_y - line_height;
    draw.line()
        .start(pt2(text_left, sep_y))
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
            .xy(pt2(text_left, item_y))
            .wh(pt2(bounds.w(), line_height).into())
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
            .xy(pt2(text_left, more_y))
            .wh(pt2(bounds.w(), line_height).into())
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
                if let Some((name, monitor, success)) = model.output_capture.select_and_connect() {
                    // Save to config
                    let mut config = Config::load();
                    config.set_pw_link_target(&name);

                    let msg = if success {
                        // Refresh devices and switch to the monitor source
                        if let Some(monitor_name) = monitor {
                            model.source.refresh_devices();
                            if let Some((dev_name, switched)) = model.source.select_device_by_name(&monitor_name) {
                                if switched {
                                    format!("Capturing: {}", name)
                                } else {
                                    format!("Connected {} (switch to {} failed)", name, dev_name)
                                }
                            } else {
                                format!("Connected {} (monitor not found)", name)
                            }
                        } else {
                            format!("Selected: {}", name)
                        }
                    } else {
                        format!("Failed: {}", name)
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

    // Space cycles visualizations
    if key == Key::Space {
        model.renderer.cycle_next();
        return;
    }

    // Number keys for device selection
    let num_devices = model.source.device_count();
    if num_devices == 0 {
        return;
    }

    let shift_offset = if app.keys.mods.shift() { 10 } else { 0 };

    let index = match key {
        Key::Key0 => Some(0 + shift_offset),
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
        if let Some((name, success)) = model.source.select_device(idx) {
            let msg = if success {
                format!("[{}] {}", idx, name)
            } else {
                format!("[{}] {} - FAILED", idx, name)
            };
            model.renderer.show_notification(msg);
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
