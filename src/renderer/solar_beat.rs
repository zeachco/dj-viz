use super::Visualization;
use nannou::prelude::*;
use num_complex::Complex;
use rustfft::FftPlanner;

use crate::audio::BUFFER_SIZE;

const FFT_SIZE: usize = BUFFER_SIZE;
const NUM_LINES: usize = if cfg!(debug_assertions) { 64 } else { 128 };

pub struct SolarBeat {
    magnitudes: Vec<f32>,
    fft_planner: FftPlanner<f32>,
    fft_window: Vec<f32>,
    smoothed_magnitudes: Vec<f32>,
    // Rotation offset for psychedelic effect
    rotation_offset: f32,
}

impl SolarBeat {
    pub fn new() -> Self {
        // Create Hann window for FFT
        let fft_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()))
            .collect();

        Self {
            magnitudes: vec![0.0; FFT_SIZE / 2],
            fft_planner: FftPlanner::new(),
            fft_window,
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
            255,
        )
    }

    /// Maps a line index to a frequency bin with logarithmic scaling
    fn line_to_freq_bin(&self, line_idx: usize) -> usize {
        let num_bins = self.magnitudes.len();
        let normalized = line_idx as f32 / (NUM_LINES - 1) as f32;
        // Power scale emphasizes bass frequencies
        let scaled = normalized.powf(2.0);
        ((scaled * (num_bins - 1) as f32) as usize).min(num_bins - 1)
    }

    /// Calculates the angle for a given line index (evenly distributed around the circle)
    fn line_to_angle(&self, line_idx: usize, rotation: f32) -> f32 {
        let base_angle = (line_idx as f32 / NUM_LINES as f32) * std::f32::consts::TAU;
        base_angle + rotation
    }
}

impl Visualization for SolarBeat {
    fn update(&mut self, samples: &[f32]) {
        // Apply window function
        let windowed: Vec<f32> = samples
            .iter()
            .zip(self.fft_window.iter())
            .map(|(s, w)| s * w)
            .collect();

        // Perform FFT
        let fft = self.fft_planner.plan_fft_forward(FFT_SIZE);
        let mut buffer: Vec<Complex<f32>> = windowed
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .collect();

        fft.process(&mut buffer);

        // Calculate magnitude spectrum (only first half - positive frequencies)
        self.magnitudes = buffer[..FFT_SIZE / 2]
            .iter()
            .map(|c| {
                let mag = c.norm() / FFT_SIZE as f32;
                // Convert to dB scale, normalize to 0-1
                let db = 20.0 * (mag + 1e-10).log10();
                // Map -60dB to 0dB range to 0-1
                ((db + 60.0) / 60.0).clamp(0.0, 1.0)
            })
            .collect();

        // Calculate smoothed magnitudes for each line
        for i in 0..NUM_LINES {
            let bin_idx = self.line_to_freq_bin(i);

            // Average neighboring bins for smoother visualization
            let start = bin_idx.saturating_sub(2);
            let end = (bin_idx + 3).min(self.magnitudes.len());
            let avg: f32 = self.magnitudes[start..end].iter().sum::<f32>()
                / (end - start) as f32;

            // Light smoothing over time
            let smoothing = 0.2;
            self.smoothed_magnitudes[i] =
                self.smoothed_magnitudes[i] * smoothing + avg * (1.0 - smoothing);
        }

        // Update rotation based on bass energy for psychedelic movement
        let bass_energy: f32 = self.smoothed_magnitudes[..NUM_LINES / 8]
            .iter()
            .sum::<f32>() / (NUM_LINES / 8) as f32;
        self.rotation_offset += 0.005 + bass_energy * 0.02;
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

            // Line thickness based on magnitude
            let thickness = 1.0 + magnitude * 3.0;

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
