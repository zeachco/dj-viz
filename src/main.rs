mod audio;
mod renderer;
mod utils;

use nannou::prelude::*;
use audio::SourcePipe;
use renderer::{Renderer, Resolution};
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

    draw.to_frame(app, &frame).unwrap();
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    // Space cycles visualizations
    if key == Key::Space {
        model.renderer.cycle_next();
        return;
    }

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
