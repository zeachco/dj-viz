use super::Visualization;
use nannou::prelude::*;
use num_complex::Complex;
use rustfft::FftPlanner;

use crate::audio::BUFFER_SIZE;

const FFT_SIZE: usize = BUFFER_SIZE;
const HISTORY_SIZE: usize = if cfg!(debug_assertions) { 100 } else { 512 };
const DISPLAY_BINS: usize = if cfg!(debug_assertions) { 64 } else { 256 };

pub struct Spectrogram {
    history: Vec<Vec<f32>>,
    fft_planner: FftPlanner<f32>,
    fft_window: Vec<f32>,
}

impl Spectrogram {
    pub fn new() -> Self {
        // Create Hann window for FFT
        let fft_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()))
            .collect();

        Self {
            history: vec![vec![0.0; FFT_SIZE / 2]; HISTORY_SIZE],
            fft_planner: FftPlanner::new(),
            fft_window,
        }
    }

    fn magnitude_to_color(mag: f32) -> Srgba<u8> {
        let mag = mag.clamp(0.0, 1.0);

        // Heat map: black -> purple -> blue -> cyan -> green -> yellow -> red -> white
        let (r, g, b) = if mag < 0.125 {
            let t = mag / 0.125;
            (0.0, 0.0, t * 0.5)
        } else if mag < 0.25 {
            let t = (mag - 0.125) / 0.125;
            (t * 0.5, 0.0, 0.5 + t * 0.5)
        } else if mag < 0.375 {
            let t = (mag - 0.25) / 0.125;
            (0.5 - t * 0.5, t, 1.0)
        } else if mag < 0.5 {
            let t = (mag - 0.375) / 0.125;
            (0.0, 1.0, 1.0 - t)
        } else if mag < 0.625 {
            let t = (mag - 0.5) / 0.125;
            (t, 1.0, 0.0)
        } else if mag < 0.75 {
            let t = (mag - 0.625) / 0.125;
            (1.0, 1.0 - t * 0.5, 0.0)
        } else if mag < 0.875 {
            let t = (mag - 0.75) / 0.125;
            (1.0, 0.5 - t * 0.5, 0.0)
        } else {
            let t = (mag - 0.875) / 0.125;
            (1.0, t, t)
        };

        // Alpha scales with magnitude for transparency blending
        let alpha = (mag * 200.0 + 55.0) as u8;

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            alpha,
        )
    }
}

impl Visualization for Spectrogram {
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
        let magnitudes: Vec<f32> = buffer[..FFT_SIZE / 2]
            .iter()
            .map(|c| {
                let mag = c.norm() / FFT_SIZE as f32;
                // Convert to dB scale, normalize to 0-1
                let db = 20.0 * (mag + 1e-10).log10();
                // Map -80dB to 0dB range to 0-1
                ((db + 80.0) / 80.0).clamp(0.0, 1.0)
            })
            .collect();

        // Add to history (scroll left)
        self.history.remove(0);
        self.history.push(magnitudes);
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let w = bounds.w();
        let h = bounds.h();

        // Calculate perimeter and edge lengths
        let perimeter = 2.0 * w + 2.0 * h;
        let bottom_ratio = w / perimeter;
        let left_ratio = h / perimeter;
        let top_ratio = w / perimeter;

        // Distribute history columns across edges proportionally
        let bottom_cols = (HISTORY_SIZE as f32 * bottom_ratio).round() as usize;
        let left_cols = (HISTORY_SIZE as f32 * left_ratio).round() as usize;
        let top_cols = (HISTORY_SIZE as f32 * top_ratio).round() as usize;
        let right_cols = HISTORY_SIZE.saturating_sub(bottom_cols + left_cols + top_cols);

        let bin_size = w.min(h) / (2.0 * DISPLAY_BINS as f32); // Bins extend inward

        let mut col_offset = 0;

        // Bottom edge: right to left, bins scale up
        if bottom_cols > 0 {
            let col_width = w / bottom_cols as f32;
            for i in 0..bottom_cols {
                let col_idx = col_offset + i;
                if col_idx >= self.history.len() {
                    break;
                }
                let column = &self.history[col_idx];
                let x = bounds.right() - (i as f32 + 0.5) * col_width;

                for y_idx in 0..DISPLAY_BINS {
                    let bin_idx = y_idx * (FFT_SIZE / 2) / DISPLAY_BINS;
                    let magnitude = column.get(bin_idx).copied().unwrap_or(0.0);
                    let y = bounds.bottom() + (y_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(col_width + 1.0, bin_size + 1.0)
                        .color(color);
                }
            }
            col_offset += bottom_cols;
        }

        // Left edge: bottom to top, bins scale right
        if left_cols > 0 {
            let col_height = h / left_cols as f32;
            for i in 0..left_cols {
                let col_idx = col_offset + i;
                if col_idx >= self.history.len() {
                    break;
                }
                let column = &self.history[col_idx];
                let y = bounds.bottom() + (i as f32 + 0.5) * col_height;

                for x_idx in 0..DISPLAY_BINS {
                    let bin_idx = x_idx * (FFT_SIZE / 2) / DISPLAY_BINS;
                    let magnitude = column.get(bin_idx).copied().unwrap_or(0.0);
                    let x = bounds.left() + (x_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(bin_size + 1.0, col_height + 1.0)
                        .color(color);
                }
            }
            col_offset += left_cols;
        }

        // Top edge: left to right, bins scale down
        if top_cols > 0 {
            let col_width = w / top_cols as f32;
            for i in 0..top_cols {
                let col_idx = col_offset + i;
                if col_idx >= self.history.len() {
                    break;
                }
                let column = &self.history[col_idx];
                let x = bounds.left() + (i as f32 + 0.5) * col_width;

                for y_idx in 0..DISPLAY_BINS {
                    let bin_idx = y_idx * (FFT_SIZE / 2) / DISPLAY_BINS;
                    let magnitude = column.get(bin_idx).copied().unwrap_or(0.0);
                    let y = bounds.top() - (y_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(col_width + 1.0, bin_size + 1.0)
                        .color(color);
                }
            }
            col_offset += top_cols;
        }

        // Right edge: top to bottom, bins scale left
        if right_cols > 0 {
            let col_height = h / right_cols as f32;
            for i in 0..right_cols {
                let col_idx = col_offset + i;
                if col_idx >= self.history.len() {
                    break;
                }
                let column = &self.history[col_idx];
                let y = bounds.top() - (i as f32 + 0.5) * col_height;

                for x_idx in 0..DISPLAY_BINS {
                    let bin_idx = x_idx * (FFT_SIZE / 2) / DISPLAY_BINS;
                    let magnitude = column.get(bin_idx).copied().unwrap_or(0.0);
                    let x = bounds.right() - (x_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(bin_size + 1.0, col_height + 1.0)
                        .color(color);
                }
            }
        }
    }
}
