//! Radial frequency burst visualization.
//!
//! Renders frequency bands as colorful rays emanating from the center, creating
//! a sunburst effect with psychedelic rotation.

use super::Visualization;
use nannou::prelude::*;

use crate::audio::{AudioAnalysis, NUM_BANDS};

const NUM_LINES: usize = if cfg!(debug_assertions) { 64 } else { 128 };

pub struct SolarBeat {
    smoothed_magnitudes: Vec<f32>,
    // Rotation offset for psychedelic effect
    rotation_offset: f32,
}

impl SolarBeat {
    pub fn new() -> Self {
        Self {
            smoothed_magnitudes: vec![0.0; NUM_LINES],
            rotation_offset: 0.0,
        }
    }

    fn magnitude_to_color(mag: f32) -> Srgba<u8> {
        let mag = mag.clamp(0.0, 1.0);

        // Vibrant color palette inspired by WMP: cyan -> magenta -> yellow -> white
        let (r, g, b) = if mag < 0.2 {
            // Deep blue to cyan
            let t = mag / 0.2;
            (0.0, t * 0.6, 0.3 + t * 0.7)
        } else if mag < 0.4 {
            // Cyan to magenta
            let t = (mag - 0.2) / 0.2;
            (t * 0.8, 0.6 - t * 0.2, 1.0 - t * 0.3)
        } else if mag < 0.6 {
            // Magenta to pink/white
            let t = (mag - 0.4) / 0.2;
            (0.8 + t * 0.2, 0.4 + t * 0.4, 0.7 + t * 0.3)
        } else if mag < 0.8 {
            // Pink to yellow
            let t = (mag - 0.6) / 0.2;
            (1.0, 0.8 + t * 0.2, 1.0 - t * 0.6)
        } else {
            // Yellow to white hot
            let t = (mag - 0.8) / 0.2;
            (1.0, 1.0, 0.4 + t * 0.6)
        };

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            128, // 50% transparency for trail effect
        )
    }

    /// Maps a line index to a band with interpolation
    fn line_to_band_value(&self, line_idx: usize, bands: &[f32; NUM_BANDS]) -> f32 {
        let normalized = line_idx as f32 / (NUM_LINES - 1) as f32;
        // Power scale emphasizes bass frequencies
        let scaled = normalized.powf(2.0);
        let band_pos = scaled * (NUM_BANDS - 1) as f32;

        // Interpolate between bands
        let low_band = (band_pos as usize).min(NUM_BANDS - 1);
        let high_band = (low_band + 1).min(NUM_BANDS - 1);
        let t = band_pos - low_band as f32;

        bands[low_band] * (1.0 - t) + bands[high_band] * t
    }

    /// Calculates the angle for a given line index (evenly distributed around the circle)
    fn line_to_angle(&self, line_idx: usize, rotation: f32) -> f32 {
        let base_angle = (line_idx as f32 / NUM_LINES as f32) * std::f32::consts::TAU;
        base_angle + rotation
    }
}

impl Visualization for SolarBeat {
    fn update(&mut self, analysis: &AudioAnalysis) {
        // Calculate smoothed magnitudes for each line from band data
        for i in 0..NUM_LINES {
            let band_value = self.line_to_band_value(i, &analysis.bands);

            // Light smoothing over time
            let smoothing = 0.2;
            self.smoothed_magnitudes[i] =
                self.smoothed_magnitudes[i] * smoothing + band_value * (1.0 - smoothing);
        }

        // Update rotation based on bass energy for psychedelic movement
        self.rotation_offset += 0.005 + analysis.bass * 0.02;
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center_x = bounds.x();
        let center_y = bounds.y();

        // Max radius for lines (use full extent of smaller dimension)
        let max_radius = bounds.w().min(bounds.h()) / 2.0;

        // Draw current frame only - trail effect handled by feedback buffer
        self.draw_burst(
            draw,
            center_x,
            center_y,
            max_radius,
            &self.smoothed_magnitudes,
            self.rotation_offset,
        );
    }
}

impl SolarBeat {
    fn draw_burst(
        &self,
        draw: &Draw,
        center_x: f32,
        center_y: f32,
        radius: f32,
        magnitudes: &[f32],
        rotation: f32,
    ) {
        // Inner radius where lines converge (very small for sharp center)
        let inner_radius = radius * 0.05;

        for i in 0..NUM_LINES {
            let angle = self.line_to_angle(i, rotation);
            let magnitude = magnitudes.get(i).copied().unwrap_or(0.0);

            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Calculate outward line length based on magnitude
            let outward_length = inner_radius + (radius - inner_radius) * magnitude;

            // Calculate inward line length (opposite direction, slightly shorter)
            let inward_length = inner_radius * (0.5 + magnitude * 1.5);

            // Center point
            let center_pt = pt2(center_x, center_y);

            // Outward endpoint
            let outward_x = center_x + outward_length * cos_a;
            let outward_y = center_y + outward_length * sin_a;
            let outward_pt = pt2(outward_x, outward_y);

            // Inward endpoint (opposite direction from center)
            let inward_x = center_x - inward_length * cos_a;
            let inward_y = center_y - inward_length * sin_a;
            let inward_pt = pt2(inward_x, inward_y);

            let color = Self::magnitude_to_color(magnitude);

            // Line thickness based on magnitude (2x wider)
            let thickness = 2.0 + magnitude * 6.0;

            // Draw outward line
            draw.line()
                .start(center_pt)
                .end(outward_pt)
                .weight(thickness)
                .color(color);

            // Draw inward line (creates the "starburst" effect)
            draw.line()
                .start(center_pt)
                .end(inward_pt)
                .weight(thickness * 0.7)
                .color(color);
        }
    }
}
