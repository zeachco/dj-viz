//! Scrolling frequency-time heatmap visualization.
//!
//! Displays audio frequency content over time as a colorful 2D road-like spectrogram with
//! beat-reactive shake and rotation effects.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::{AudioAnalysis, NUM_BANDS};

const HISTORY_SIZE: usize = if cfg!(debug_assertions) { 50 } else { 200 };
/// Number of visual bins to display (interpolated from NUM_BANDS)
const DISPLAY_BINS: usize = if cfg!(debug_assertions) { 16 } else { 24 };

pub struct SpectroRoad {
    /// History of band values for scrolling display
    history: Vec<[f32; NUM_BANDS]>,
    /// Beat-reactive shake offset
    shake_x: f32,
    shake_y: f32,
    /// Beat-reactive rotation
    rotation: f32,
    /// Current music intensity (for alpha modulation)
    intensity: f32,
    /// Current bass level (for border effect)
    bass: f32,
    /// Frame counter for time-based effects
    frame_count: u32,
    /// Counter to slow down history scrolling
    shift_counter: u32,
}

impl Default for SpectroRoad {
    fn default() -> Self {
        Self {
            history: vec![[0.0; NUM_BANDS]; HISTORY_SIZE],
            shake_x: 0.0,
            shake_y: 0.0,
            rotation: 0.0,
            intensity: 0.0,
            bass: 0.0,
            frame_count: 0,
            shift_counter: 0,
        }
    }
}

impl SpectroRoad {
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

    /// Calculate border width based on sin wave (15s period)
    fn border_width(&self) -> f32 {
        // Sin wave over 15 seconds (assuming 60fps)
        let phase = self.frame_count as f32 * std::f32::consts::TAU / (15.0 * 60.0);
        let sin_factor = (phase.sin() + 1.0) / 2.0; // 0 to 1
        // Border width 1-5px based purely on sin wave
        1.0 + sin_factor * 4.0
    }

    /// Get border color (90 degrees rotated hue, saturation based on intensity)
    fn border_color(&self, main_color: Srgba<u8>) -> Srgba<u8> {
        // Convert RGB to HSV, rotate hue by 90 degrees
        let r = main_color.red as f32 / 255.0;
        let g = main_color.green as f32 / 255.0;
        let b = main_color.blue as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        // Calculate hue
        let hue = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };
        let hue = if hue < 0.0 { hue + 360.0 } else { hue };

        // Rotate by 90 degrees
        let new_hue = (hue + 90.0) % 360.0;

        // Saturation modulated by intensity (more intense = more saturated border)
        let base_saturation = if max == 0.0 { 0.0 } else { delta / max };
        let saturation = base_saturation * (0.3 + self.intensity * 0.7);
        let value = max;

        // Convert back to RGB
        let c = value * saturation;
        let x = c * (1.0 - ((new_hue / 60.0) % 2.0 - 1.0).abs());
        let m = value - c;

        let (r1, g1, b1) = if new_hue < 60.0 {
            (c, x, 0.0)
        } else if new_hue < 120.0 {
            (x, c, 0.0)
        } else if new_hue < 180.0 {
            (0.0, c, x)
        } else if new_hue < 240.0 {
            (0.0, x, c)
        } else if new_hue < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        srgba(
            ((r1 + m) * 255.0) as u8,
            ((g1 + m) * 255.0) as u8,
            ((b1 + m) * 255.0) as u8,
            main_color.alpha,
        )
    }

    /// Convert magnitude to color with intensity-based alpha
    fn magnitude_to_color(&self, mag: f32) -> Srgba<u8> {
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

        // Calculate saturation - colors near black/white are less saturated
        let max_rgb = r.max(g).max(b);
        let min_rgb = r.min(g).min(b);
        let saturation = if max_rgb > 0.001 { (max_rgb - min_rgb) / max_rgb } else { 0.0 };

        // Base alpha for desaturated colors scales with music intensity
        // Quiet: white/black very transparent (5%), Loud: more visible (30%)
        let base_alpha = 0.05 + self.intensity * 0.25;
        // Saturated colors always more visible, plus intensity boost
        let saturated_alpha = 0.35 + self.intensity * 0.15;

        let alpha = base_alpha + saturation * (saturated_alpha - base_alpha);

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (alpha * 255.0) as u8,
        )
    }
}

impl Visualization for SpectroRoad {
    fn update(&mut self, analysis: &AudioAnalysis) {
        // Slower scrolling: only shift every 3 frames
        self.shift_counter = self.shift_counter.wrapping_add(1);
        if self.shift_counter % 3 == 0 {
            self.history.remove(0);
            // Use analyzer's already-smoothed bands (0-1 range)
            self.history.push(analysis.bands);
        }

        // Track intensity with less smoothing for more reactive scaling
        self.intensity = self.intensity * 0.7 + analysis.energy * 0.3;

        // Track bass for border effect
        self.bass = self.bass * 0.7 + analysis.bass * 0.3;

        // Increment frame counter
        self.frame_count = self.frame_count.wrapping_add(1);

        // Beat-reactive shake: trigger on bass hits
        if analysis.bass > 0.4 {
            let mut rng = rand::rng();
            let shake_intensity = analysis.bass * 15.0;
            self.shake_x += rng.random_range(-1.0..1.0) * shake_intensity;
            self.shake_y += rng.random_range(-1.0..1.0) * shake_intensity;
            self.rotation += rng.random_range(-1.0..1.0) * analysis.bass * 0.008;
        }

        // Decay shake and rotation
        self.shake_x *= 0.85;
        self.shake_y *= 0.85;
        self.rotation *= 0.95;
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        // Apply beat-reactive transform (shake + rotation + slight zoom to hide edges)
        let draw = draw
            .x_y(self.shake_x, self.shake_y)
            .rotate(self.rotation)
            .scale(1.02);

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

                let border_w = self.border_width();
                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let y = bounds.bottom() + (bin_idx as f32 + 0.5) * bin_size;

                    let color = self.magnitude_to_color(magnitude);
                    let border_color = self.border_color(color);
                    let mut rng = rand::rng();
                    // Gutter can reach up to 70% of each dimension
                    let gutter_w = rng.random_range(1.0..(col_width * 0.7).max(2.0));
                    let gutter_h = rng.random_range(1.0..(bin_size * 0.7).max(2.0));
                    // Scale rects dramatically with energy (up to 150% larger at max energy)
                    let intensity_scale = 1.0 + self.intensity * 1.5;
                    let rect_w = (col_width - gutter_w) * intensity_scale;
                    let rect_h = (bin_size - gutter_h) * intensity_scale;
                    // Draw border (larger rect behind)
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w + border_w * 2.0, rect_h + border_w * 2.0)
                        .color(border_color);
                    // Draw main rect
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w, rect_h)
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

                let border_w = self.border_width();
                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let x = bounds.left() + (bin_idx as f32 + 0.5) * bin_size;

                    let color = self.magnitude_to_color(magnitude);
                    let border_color = self.border_color(color);
                    let mut rng = rand::rng();
                    let gutter_w = rng.random_range(1.0..(bin_size * 0.7).max(2.0));
                    let gutter_h = rng.random_range(1.0..(col_height * 0.7).max(2.0));
                    // Scale rects dramatically with energy (up to 150% larger at max energy)
                    let intensity_scale = 1.0 + self.intensity * 1.5;
                    let rect_w = (bin_size - gutter_w) * intensity_scale;
                    let rect_h = (col_height - gutter_h) * intensity_scale;
                    // Draw border
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w + border_w * 2.0, rect_h + border_w * 2.0)
                        .color(border_color);
                    // Draw main rect
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w, rect_h)
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

                let border_w = self.border_width();
                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let y = bounds.top() - (bin_idx as f32 + 0.5) * bin_size;

                    let color = self.magnitude_to_color(magnitude);
                    let border_color = self.border_color(color);
                    let mut rng = rand::rng();
                    let gutter_w = rng.random_range(1.0..(col_width * 0.7).max(2.0));
                    let gutter_h = rng.random_range(1.0..(bin_size * 0.7).max(2.0));
                    // Scale rects dramatically with energy (up to 150% larger at max energy)
                    let intensity_scale = 1.0 + self.intensity * 1.5;
                    let rect_w = (col_width - gutter_w) * intensity_scale;
                    let rect_h = (bin_size - gutter_h) * intensity_scale;
                    // Draw border
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w + border_w * 2.0, rect_h + border_w * 2.0)
                        .color(border_color);
                    // Draw main rect
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w, rect_h)
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

                let border_w = self.border_width();
                for bin_idx in 0..DISPLAY_BINS {
                    let magnitude = Self::interpolate_band(column, bin_idx);
                    let x = bounds.right() - (bin_idx as f32 + 0.5) * bin_size;

                    let color = self.magnitude_to_color(magnitude);
                    let border_color = self.border_color(color);
                    let mut rng = rand::rng();
                    let gutter_w = rng.random_range(1.0..(bin_size * 0.7).max(2.0));
                    let gutter_h = rng.random_range(1.0..(col_height * 0.7).max(2.0));
                    // Scale rects dramatically with energy (up to 150% larger at max energy)
                    let intensity_scale = 1.0 + self.intensity * 1.5;
                    let rect_w = (bin_size - gutter_w) * intensity_scale;
                    let rect_h = (col_height - gutter_h) * intensity_scale;
                    // Draw border
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w + border_w * 2.0, rect_h + border_w * 2.0)
                        .color(border_color);
                    // Draw main rect
                    draw.rect()
                        .x_y(x, y)
                        .w_h(rect_w, rect_h)
                        .color(color);
                }
            }
        }
    }
}
