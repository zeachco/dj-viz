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

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            255,
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

        let column_width = w / HISTORY_SIZE as f32;
        let row_height = h / DISPLAY_BINS as f32;

        // Draw spectrogram
        for (x_idx, column) in self.history.iter().enumerate() {
            let x = bounds.left() + (x_idx as f32 + 0.5) * column_width;

            for y_idx in 0..DISPLAY_BINS {
                let bin_idx = y_idx * (FFT_SIZE / 2) / DISPLAY_BINS;
                let magnitude = column.get(bin_idx).copied().unwrap_or(0.0);
                let y = bounds.bottom() + (y_idx as f32 + 0.5) * row_height;

                let color = Self::magnitude_to_color(magnitude);

                draw.rect()
                    .x_y(x, y)
                    .w_h(column_width + 1.0, row_height + 1.0)
                    .color(color);
            }
        }
    }
}
