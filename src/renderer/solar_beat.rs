use super::Visualization;
use nannou::prelude::*;
use num_complex::Complex;
use rustfft::FftPlanner;

use crate::audio::BUFFER_SIZE;

const FFT_SIZE: usize = BUFFER_SIZE;
const NUM_LINES: usize = if cfg!(debug_assertions) { 64 } else { 128 };
const OVAL_SCALE: f32 = 0.75;

pub struct Radial {
    magnitudes: Vec<f32>,
    fft_planner: FftPlanner<f32>,
    fft_window: Vec<f32>,
    smoothed_magnitudes: Vec<f32>,
}

impl Radial {
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
        }
    }

    fn magnitude_to_color(mag: f32) -> Srgba<u8> {
        let mag = mag.clamp(0.0, 1.0);

        // Cyan to magenta gradient through white for high values
        let (r, g, b) = if mag < 0.25 {
            let t = mag / 0.25;
            (0.0, t * 0.8, t)
        } else if mag < 0.5 {
            let t = (mag - 0.25) / 0.25;
            (t * 0.5, 0.8 + t * 0.2, 1.0)
        } else if mag < 0.75 {
            let t = (mag - 0.5) / 0.25;
            (0.5 + t * 0.5, 1.0, 1.0 - t * 0.5)
        } else {
            let t = (mag - 0.75) / 0.25;
            (1.0, 1.0 - t * 0.5, 0.5 + t * 0.5)
        };

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            255,
        )
    }

    /// Maps a line index (0 to NUM_LINES-1) to a frequency bin index
    /// Bottom of circle = low frequencies, top = high frequencies
    /// Each side (left/right) shows the full frequency range
    fn line_to_freq_bin(&self, line_idx: usize, side: Side) -> usize {
        let half_lines = NUM_LINES / 2;
        let num_bins = self.magnitudes.len();

        // Position within half (0 at bottom, half_lines-1 at top)
        let pos_in_half = match side {
            Side::Right => {
                // Right side: line 0 is at bottom, going up to top
                if line_idx < half_lines {
                    line_idx
                } else {
                    NUM_LINES - 1 - line_idx
                }
            }
            Side::Left => {
                // Left side: mirrored
                if line_idx < half_lines {
                    half_lines - 1 - line_idx
                } else {
                    line_idx - half_lines
                }
            }
        };

        // Map position to frequency bin (log scale for perceptual accuracy)
        let normalized = pos_in_half as f32 / (half_lines - 1) as f32;
        // Use power scale to emphasize bass frequencies visually
        let scaled = normalized.powf(2.0);
        ((scaled * (num_bins - 1) as f32) as usize).min(num_bins - 1)
    }

    /// Calculates the angle for a given line index
    /// Starting from bottom (270°), right side goes clockwise, left side counter-clockwise
    fn line_to_angle(&self, line_idx: usize) -> f32 {
        let half_lines = NUM_LINES / 2;

        if line_idx < half_lines {
            // Right side: from bottom (3π/2) to top (π/2) going through 0
            let t = line_idx as f32 / (half_lines - 1) as f32;
            // 3π/2 (bottom) -> 2π -> 0 -> π/2 (top)
            // Simplified: -π/2 + t * π = from -π/2 to π/2
            -std::f32::consts::FRAC_PI_2 + t * std::f32::consts::PI
        } else {
            // Left side: from bottom (3π/2) to top (π/2) going through π
            let t = (line_idx - half_lines) as f32 / (half_lines - 1) as f32;
            // 3π/2 (bottom) -> π -> π/2 (top)
            // = 3π/2 - t * π
            std::f32::consts::FRAC_PI_2 * 3.0 - t * std::f32::consts::PI
        }
    }
}

#[derive(Clone, Copy)]
enum Side {
    Right,
    Left,
}

impl Visualization for Radial {
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
                // Map -60dB to 0dB range to 0-1 (more sensitive than spectrogram)
                ((db + 60.0) / 60.0).clamp(0.0, 1.0)
            })
            .collect();

        // Calculate smoothed magnitudes for each line
        let half_lines = NUM_LINES / 2;
        for i in 0..NUM_LINES {
            let side = if i < half_lines { Side::Right } else { Side::Left };
            let bin_idx = self.line_to_freq_bin(i, side);

            // Average a few neighboring bins for smoother visualization
            let start = bin_idx.saturating_sub(2);
            let end = (bin_idx + 3).min(self.magnitudes.len());
            let avg: f32 = self.magnitudes[start..end].iter().sum::<f32>()
                / (end - start) as f32;

            // Smooth over time
            let smoothing = 0.3;
            self.smoothed_magnitudes[i] =
                self.smoothed_magnitudes[i] * smoothing + avg * (1.0 - smoothing);
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center_x = bounds.x();
        let center_y = bounds.y();

        // Calculate ellipse radii (0.75 of the way to edges)
        let max_rx = bounds.w() / 2.0 * OVAL_SCALE;
        let max_ry = bounds.h() / 2.0 * OVAL_SCALE;

        // Base radius for the inner circle (where lines start)
        let inner_scale = 0.2;
        let inner_rx = max_rx * inner_scale;
        let inner_ry = max_ry * inner_scale;

        // Draw each line
        for i in 0..NUM_LINES {
            let angle = self.line_to_angle(i);
            let magnitude = self.smoothed_magnitudes[i];

            // Calculate line length based on magnitude
            // Lines extend from inner ellipse to outer ellipse based on magnitude
            let outer_scale = inner_scale + (1.0 - inner_scale) * magnitude;

            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Start point (on inner ellipse)
            let start_x = center_x + inner_rx * cos_a;
            let start_y = center_y + inner_ry * sin_a;

            // End point (extends based on magnitude)
            let end_x = center_x + max_rx * outer_scale * cos_a;
            let end_y = center_y + max_ry * outer_scale * sin_a;

            let color = Self::magnitude_to_color(magnitude);

            // Draw the line with thickness based on magnitude
            let thickness = 1.0 + magnitude * 3.0;
            draw.line()
                .start(pt2(start_x, start_y))
                .end(pt2(end_x, end_y))
                .weight(thickness)
                .color(color);
        }

        // Draw a subtle inner ellipse outline
        let inner_color = srgba(50u8, 50u8, 80u8, 200u8);
        draw.ellipse()
            .x_y(center_x, center_y)
            .w_h(inner_rx * 2.0, inner_ry * 2.0)
            .no_fill()
            .stroke(inner_color)
            .stroke_weight(1.0);
    }
}
