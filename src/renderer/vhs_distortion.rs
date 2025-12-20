use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::{AudioAnalysis, NUM_BANDS};

/// Number of scanlines
const NUM_SCANLINES: usize = if cfg!(debug_assertions) { 30 } else { 60 };
/// Number of frequency bars
const NUM_BARS: usize = if cfg!(debug_assertions) { 16 } else { 32 };
/// Maximum tracking error offset
const MAX_TRACKING_OFFSET: f32 = 50.0;
/// Chromatic aberration max offset
const MAX_CHROMATIC_OFFSET: f32 = 15.0;

pub struct VhsDistortion {
    /// Tracking error offset (simulates VHS tracking issues)
    tracking_offset: f32,
    /// Target tracking offset
    target_tracking: f32,
    /// Horizontal noise bands (y positions of glitch bands)
    glitch_bands: Vec<f32>,
    /// Glitch band intensities
    glitch_intensities: Vec<f32>,
    /// Chromatic aberration offset
    chromatic_offset: f32,
    /// Scanline phase
    scanline_phase: f32,
    /// Bass level
    bass: f32,
    /// Treble level
    treble: f32,
    /// Energy level
    energy: f32,
    /// Frame counter
    frame_count: u32,
    /// Smoothed band values for bars
    smoothed_bands: [f32; NUM_BANDS],
    /// Head switch noise (bottom of frame distortion)
    head_switch_intensity: f32,
    /// Color shift amount
    color_shift: f32,
}

impl VhsDistortion {
    pub fn new() -> Self {
        Self {
            tracking_offset: 0.0,
            target_tracking: 0.0,
            glitch_bands: vec![0.0; 5],
            glitch_intensities: vec![0.0; 5],
            chromatic_offset: 0.0,
            scanline_phase: 0.0,
            bass: 0.0,
            treble: 0.0,
            energy: 0.0,
            frame_count: 0,
            smoothed_bands: [0.0; NUM_BANDS],
            head_switch_intensity: 0.0,
            color_shift: 0.0,
        }
    }

    /// Generate VHS-style color (slightly washed out, shifted)
    fn vhs_color(&self, base_hue: f32, saturation: f32, value: f32) -> (f32, f32, f32) {
        // VHS has limited color accuracy - reduce saturation, shift colors
        let hue = (base_hue + self.color_shift * 30.0) % 360.0;
        let sat = saturation * 0.7; // Washed out look
        let val = value * 0.9 + 0.1; // Slightly lifted blacks

        Self::hsv_to_rgb(hue, sat, val)
    }

    fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> (f32, f32, f32) {
        let hue = hue % 360.0;
        let c = value * saturation;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = value - c;

        let (r1, g1, b1) = if hue < 60.0 {
            (c, x, 0.0)
        } else if hue < 120.0 {
            (x, c, 0.0)
        } else if hue < 180.0 {
            (0.0, c, x)
        } else if hue < 240.0 {
            (0.0, x, c)
        } else if hue < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        (r1 + m, g1 + m, b1 + m)
    }
}

impl Visualization for VhsDistortion {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);
        let mut rng = rand::rng();

        // Smooth audio tracking
        self.bass = self.bass * 0.7 + analysis.bass * 0.3;
        self.treble = self.treble * 0.8 + analysis.treble * 0.2;
        self.energy = self.energy * 0.9 + analysis.energy * 0.1;

        // Smooth band values
        for i in 0..NUM_BANDS {
            self.smoothed_bands[i] = self.smoothed_bands[i] * 0.7 + analysis.bands[i] * 0.3;
        }

        // Tracking error triggered by bass hits
        if analysis.bass > 0.6 && rng.random::<f32>() < 0.3 {
            self.target_tracking = rng.random_range(-MAX_TRACKING_OFFSET..MAX_TRACKING_OFFSET)
                * analysis.bass;
        }
        self.tracking_offset = self.tracking_offset * 0.9 + self.target_tracking * 0.1;
        self.target_tracking *= 0.95;

        // Chromatic aberration tied to treble
        self.chromatic_offset = self.treble * MAX_CHROMATIC_OFFSET;

        // Scanline scrolling
        self.scanline_phase += 0.5 + self.energy * 2.0;

        // Update glitch bands
        for i in 0..self.glitch_bands.len() {
            // Random chance to spawn new glitch
            if rng.random::<f32>() < 0.02 * (1.0 + analysis.bass) {
                self.glitch_bands[i] = rng.random_range(-1.0..1.0);
                self.glitch_intensities[i] = rng.random_range(0.3..1.0) * analysis.energy;
            }
            // Decay glitch
            self.glitch_intensities[i] *= 0.92;
        }

        // Head switch noise at bottom on bass hits
        if analysis.bass > 0.5 {
            self.head_switch_intensity = analysis.bass * 0.8;
        } else {
            self.head_switch_intensity *= 0.9;
        }

        // Color shift drifts over time
        self.color_shift = (self.frame_count as f32 * 0.02).sin() * 0.5
            + analysis.treble * 0.3;
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let w = bounds.w();
        let h = bounds.h();
        let left = bounds.left();
        let bottom = bounds.bottom();
        let mut rng = rand::rng();

        // Draw frequency bars with VHS color palette
        let bar_width = w / NUM_BARS as f32;
        for i in 0..NUM_BARS {
            // Map bar index to band with interpolation
            let band_pos = (i as f32 / NUM_BARS as f32) * (NUM_BANDS - 1) as f32;
            let low_band = band_pos as usize;
            let high_band = (low_band + 1).min(NUM_BANDS - 1);
            let t = band_pos - low_band as f32;
            let magnitude = self.smoothed_bands[low_band] * (1.0 - t)
                + self.smoothed_bands[high_band] * t;

            let bar_height = magnitude * h * 0.8;
            let x = left + i as f32 * bar_width + bar_width / 2.0;

            // VHS color palette - cyans, magentas, yellows
            let hue = 180.0 + i as f32 * 5.0 + self.color_shift * 60.0;
            let (r, g, b) = self.vhs_color(hue, 0.8, 0.7 + magnitude * 0.3);

            // Apply tracking offset to horizontal position
            let x_offset = if (bottom + bar_height / 2.0) > 0.0 {
                self.tracking_offset * (1.0 + rng.random_range(-0.1..0.1))
            } else {
                0.0
            };

            // Draw with chromatic aberration (RGB split)
            // Red channel (shifted left)
            draw.rect()
                .x_y(x + x_offset - self.chromatic_offset, bottom + bar_height / 2.0)
                .w_h(bar_width - 2.0, bar_height)
                .color(srgba((r * 255.0) as u8, 0, 0, 80));

            // Green channel (center)
            draw.rect()
                .x_y(x + x_offset, bottom + bar_height / 2.0)
                .w_h(bar_width - 2.0, bar_height)
                .color(srgba(0, (g * 255.0) as u8, 0, 80));

            // Blue channel (shifted right)
            draw.rect()
                .x_y(x + x_offset + self.chromatic_offset, bottom + bar_height / 2.0)
                .w_h(bar_width - 2.0, bar_height)
                .color(srgba(0, 0, (b * 255.0) as u8, 80));

            // Combined color on top
            draw.rect()
                .x_y(x + x_offset, bottom + bar_height / 2.0)
                .w_h(bar_width - 2.0, bar_height)
                .color(srgba(
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    120,
                ));
        }

        // Draw scanlines
        let scanline_spacing = h / NUM_SCANLINES as f32;
        for i in 0..NUM_SCANLINES {
            let y = bottom + i as f32 * scanline_spacing
                + (self.scanline_phase % scanline_spacing);

            // Alternating dark lines
            let alpha = if i % 2 == 0 { 0.15 } else { 0.05 };

            draw.rect()
                .x_y(bounds.x(), y)
                .w_h(w, 1.0)
                .color(srgba(0, 0, 0, (alpha * 255.0) as u8));
        }

        // Draw glitch bands (horizontal noise)
        for i in 0..self.glitch_bands.len() {
            if self.glitch_intensities[i] > 0.05 {
                let y = bounds.y() + self.glitch_bands[i] * h / 2.0;
                let band_height = 5.0 + rng.random_range(0.0..20.0) * self.glitch_intensities[i];
                let x_shift = rng.random_range(-50.0..50.0) * self.glitch_intensities[i];

                // Noise pattern
                let num_segments = 20;
                let seg_width = w / num_segments as f32;

                for s in 0..num_segments {
                    if rng.random::<f32>() < 0.7 {
                        let seg_x = left + s as f32 * seg_width + seg_width / 2.0 + x_shift;
                        let brightness = rng.random_range(0.3..1.0);

                        draw.rect()
                            .x_y(seg_x, y)
                            .w_h(seg_width, band_height)
                            .color(srgba(
                                (brightness * 255.0) as u8,
                                (brightness * 255.0) as u8,
                                (brightness * 255.0) as u8,
                                (self.glitch_intensities[i] * 200.0) as u8,
                            ));
                    }
                }
            }
        }

        // Head switch noise at bottom
        if self.head_switch_intensity > 0.1 {
            let noise_height = 30.0 * self.head_switch_intensity;
            let num_noise_lines = 10;

            for i in 0..num_noise_lines {
                let y = bottom + i as f32 * (noise_height / num_noise_lines as f32);
                let x_offset = rng.random_range(-30.0..30.0) * self.head_switch_intensity;

                // Random noise segments
                let num_segs = rng.random_range(5..15);
                for _ in 0..num_segs {
                    let seg_x = left + rng.random_range(0.0..w);
                    let seg_w = rng.random_range(10.0..50.0);
                    let brightness = rng.random_range(0.0..1.0);

                    draw.rect()
                        .x_y(seg_x + x_offset, y)
                        .w_h(seg_w, 3.0)
                        .color(srgba(
                            (brightness * 255.0) as u8,
                            (brightness * 200.0) as u8,
                            (brightness * 180.0) as u8,
                            (self.head_switch_intensity * 180.0) as u8,
                        ));
                }
            }
        }

        // Occasional full-frame color flash on peaks
        if self.bass > 0.7 && rng.random::<f32>() < 0.1 {
            let flash_alpha = (self.bass - 0.7) * 0.3;
            let hue = rng.random_range(0.0..360.0);
            let (r, g, b) = Self::hsv_to_rgb(hue, 0.3, 0.9);

            draw.rect()
                .xy(bounds.xy())
                .wh(bounds.wh())
                .color(srgba(
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    (flash_alpha * 255.0) as u8,
                ));
        }

        // VHS timestamp overlay (bottom right)
        let timestamp_alpha = 0.4 + self.energy * 0.3;
        let minutes = (self.frame_count / 3600) % 60;
        let seconds = (self.frame_count / 60) % 60;
        let text = format!("{:02}:{:02}", minutes, seconds);

        draw.text(&text)
            .x_y(bounds.right() - 50.0, bounds.bottom() + 20.0)
            .color(srgba(255, 255, 255, (timestamp_alpha * 255.0) as u8))
            .font_size(14);
    }
}
