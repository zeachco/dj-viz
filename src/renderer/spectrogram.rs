use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::{AudioAnalysis, NUM_BANDS};

const HISTORY_SIZE: usize = if cfg!(debug_assertions) { 100 } else { 512 };
/// Number of visual bins to display (interpolated from NUM_BANDS)
const DISPLAY_BINS: usize = if cfg!(debug_assertions) { 32 } else { 64 };

pub struct Spectrogram {
    /// History of band values for scrolling display
    history: Vec<[f32; NUM_BANDS]>,
    /// Beat-reactive shake offset
    shake_x: f32,
    shake_y: f32,
    /// Beat-reactive rotation
    rotation: f32,
}

impl Spectrogram {
    pub fn new() -> Self {
        Self {
            history: vec![[0.0; NUM_BANDS]; HISTORY_SIZE],
            shake_x: 0.0,
            shake_y: 0.0,
            rotation: 0.0,
        }
    }

    /// Interpolate from NUM_BANDS to a specific display bin
    fn interpolate_band(bands: &[f32; NUM_BANDS], bin_idx: usize) -> f32 {
        // Map display bin to band position with log-like scaling for bass emphasis
        let normalized = bin_idx as f32 / (DISPLAY_BINS - 1) as f32;
        let scaled = normalized.powf(1.5); // Slight emphasis on lower frequencies
        let band_pos = scaled * (NUM_BANDS - 1) as f32;

        let low_band = (band_pos as usize).min(NUM_BANDS - 1);
        let high_band = (low_band + 1).min(NUM_BANDS - 1);
        let t = band_pos - low_band as f32;

        bands[low_band] * (1.0 - t) + bands[high_band] * t
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
    fn update(&mut self, analysis: &AudioAnalysis) {
        // Add to history (scroll left)
        self.history.remove(0);
        self.history.push(analysis.bands);

        // Beat-reactive shake: trigger on bass hits
        if analysis.bass > 0.4 {
            let mut rng = rand::rng();
            let intensity = analysis.bass * 15.0;
            self.shake_x += rng.random_range(-1.0..1.0) * intensity;
            self.shake_y += rng.random_range(-1.0..1.0) * intensity;
            self.rotation += rng.random_range(-1.0..1.0) * analysis.bass * 0.03;
        }

        // Decay shake and rotation
        self.shake_x *= 0.85;
        self.shake_y *= 0.85;
        self.rotation *= 0.92;
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        // Apply beat-reactive transform (shake + rotation)
        let draw = draw
            .x_y(self.shake_x, self.shake_y)
            .rotate(self.rotation);

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

                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let y = bounds.bottom() + (bin_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(col_width - 2.0, bin_size - 2.0)
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

                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let x = bounds.left() + (bin_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(bin_size - 2.0, col_height - 2.0)
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

                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let y = bounds.top() - (bin_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(col_width - 2.0, bin_size - 2.0)
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

                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let x = bounds.right() - (bin_idx as f32 + 0.5) * bin_size;

                    let color = Self::magnitude_to_color(magnitude);
                    draw.rect()
                        .x_y(x, y)
                        .w_h(bin_size - 2.0, col_height - 2.0)
                        .color(color);
                }
            }
        }
    }
}
