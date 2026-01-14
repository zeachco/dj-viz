//! Debug visualization.
//!
//! Renders the debug.rhai script as an overlay on top of the primary visualization.

use super::scripted::ScriptedVisualization;
use super::VizInfo;
use nannou::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Instant;

use crate::audio::AudioAnalysis;

pub struct DebugViz {
    /// Last frame time for FPS calculation
    last_frame_time: Instant,
    /// Smoothed FPS display value
    display_fps: f32,
    /// The debug.rhai script visualization (RefCell for interior mutability in draw)
    debug_script: RefCell<Option<ScriptedVisualization>>,
}

impl DebugViz {
    /// Create a new debug visualization
    pub fn new() -> Self {
        // Try to load the debug.rhai script
        let script_path = PathBuf::from("scripts/debug.rhai");
        let debug_script = match ScriptedVisualization::new(script_path) {
            Ok(viz) => {
                println!("Debug script loaded: scripts/debug.rhai");
                Some(viz)
            }
            Err(e) => {
                eprintln!("Failed to load debug script: {}", e);
                None
            }
        };

        Self {
            last_frame_time: Instant::now(),
            display_fps: 0.0,
            debug_script: RefCell::new(debug_script),
        }
    }

    /// Update the debug visualization with audio analysis and bounds
    pub fn update(&mut self, analysis: &AudioAnalysis, bounds: Rect, viz_info: &VizInfo) {
        // Calculate FPS
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame_time).as_secs_f32();
        let current_fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };
        // Smooth FPS with exponential moving average
        self.display_fps = self.display_fps * 0.9 + current_fps * 0.1;
        self.last_frame_time = now;

        // Update the debug script
        if let Some(ref mut script) = *self.debug_script.borrow_mut() {
            // Pass FPS to the script
            script.set_var("fps", self.display_fps as f64);
            script.update(analysis, bounds, viz_info);
        }
    }

    /// Draw the debug visualization
    pub fn draw(&self, draw: &Draw, _bounds: Rect) {
        // Draw the debug script (background and FPS are handled in the script)
        if let Some(ref script) = *self.debug_script.borrow() {
            script.draw_overlay(draw);
        }
    }
}
