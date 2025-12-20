//! Debug visualization.
//!
//! Displays all audio analysis inputs in a retro CRT terminal style with
//! RGB chromatic aberration, scanlines, phosphor glow, vignette effects,
//! and calibration threshold indicators.

use super::Visualization;
use nannou::prelude::*;

use crate::audio::AudioAnalysis;

/// Base font size for numbers
const FONT_SIZE: u32 = 24;
/// Pixel offset for RGB fringing
const RGB_OFFSET: f32 = 2.0;
/// Pixels between scanlines
const SCANLINE_SPACING: f32 = 3.0;
/// Darkness of scanlines (0-255)
const SCANLINE_ALPHA: u8 = 15;
/// Number of blur passes
const BLUR_LAYERS: usize = 3;
/// How much each blur layer fades
const BLUR_ALPHA_DECAY: f32 = 0.5;
/// Green CRT phosphor (classic terminal)
const PHOSPHOR_HUE: f32 = 120.0;
/// Update numbers every N frames (reduces flicker)
const UPDATE_INTERVAL: u32 = 3;

pub struct DebugViz {
    /// Frame counter
    frame_count: u32,
    /// Cached display values (updated every UPDATE_INTERVAL frames)
    display_bands: [f32; 8],
    display_bass: f32,
    display_mids: f32,
    display_treble: f32,
    display_energy: f32,
    display_energy_diff: f32,
    display_transition: bool,
    /// Scanline offset for subtle animation
    scanline_offset: f32,
    /// Phosphor glow intensity
    glow_intensity: f32,
}

impl DebugViz {
    /// Create a new debug visualization
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            display_bands: [0.0; 8],
            display_bass: 0.0,
            display_mids: 0.0,
            display_treble: 0.0,
            display_energy: 0.0,
            display_energy_diff: 0.0,
            display_transition: false,
            scanline_offset: 0.0,
            glow_intensity: 0.5,
        }
    }

    /// Convert HSV to RGB color space
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

    /// Get phosphor green color with specified alpha
    fn phosphor_color(&self, alpha: f32) -> Srgba<u8> {
        let (r, g, b) = Self::hsv_to_rgb(PHOSPHOR_HUE, 0.9, 0.9);

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            alpha.clamp(0.0, 255.0) as u8,
        )
    }

    /// Get color based on value: gray -> green -> yellow -> red (0.0 -> 1.0)
    fn value_color(&self, value: f32, alpha: f32) -> Srgba<u8> {
        let clamped = value.clamp(0.0, 1.0);

        let (r, g, b) = if clamped < 0.25 {
            // 0.0 - 0.25: gray to green
            let t = clamped / 0.25;
            let gray = 0.4;
            let r = gray * (1.0 - t);
            let g = gray * (1.0 - t) + 0.8 * t;
            let b = gray * (1.0 - t);
            (r, g, b)
        } else if clamped < 0.5 {
            // 0.25 - 0.5: green
            (0.0, 0.8, 0.0)
        } else if clamped < 0.75 {
            // 0.5 - 0.75: green to yellow
            let t = (clamped - 0.5) / 0.25;
            let r = 0.9 * t;
            let g = 0.8;
            let b = 0.0;
            (r, g, b)
        } else {
            // 0.75 - 1.0: yellow to red
            let t = (clamped - 0.75) / 0.25;
            let r = 0.9;
            let g = 0.8 * (1.0 - t);
            let b = 0.0;
            (r, g, b)
        };

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            alpha.clamp(0.0, 255.0) as u8,
        )
    }

    /// Format a float value for display
    fn format_value(value: f32) -> String {
        format!("{:.2}", value)
    }
}

impl Visualization for DebugViz {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count += 1;

        // Animate scanlines
        self.scanline_offset += 0.5;
        if self.scanline_offset >= SCANLINE_SPACING {
            self.scanline_offset -= SCANLINE_SPACING;
        }

        // Update display values every UPDATE_INTERVAL frames
        if self.frame_count % UPDATE_INTERVAL == 0 {
            self.display_bands = analysis.bands;
            self.display_bass = analysis.bass;
            self.display_mids = analysis.mids;
            self.display_treble = analysis.treble;
            self.display_energy = analysis.energy;
            self.display_energy_diff = analysis.energy_diff;
            self.display_transition = analysis.transition_detected;
        }

        // Smooth glow intensity
        let target_glow = 0.5 + analysis.energy * 0.5;
        self.glow_intensity = self.glow_intensity * 0.9 + target_glow * 0.1;
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let bounds_w = bounds.w();
        let bounds_h = bounds.h();

        // Draw semi-transparent black background
        draw.rect()
            .x_y(center.x, center.y)
            .w_h(bounds_w, bounds_h)
            .color(srgba(0u8, 0u8, 0u8, 180u8));

        // Layout configuration
        let column_spacing = bounds_w / 4.0;
        let row_spacing = 30.0;
        let start_y = bounds.top() - 50.0;
        let indicator_width = 150.0; // Max width for value indicators

        // Prepare text data: (label, value_str, x, y, numeric_value)
        let mut text_data: Vec<(String, String, f32, f32, Option<f32>)> = Vec::new();

        // Column 1: Frequency bands
        let col1_x = center.x - column_spacing;
        for i in 0..8 {
            text_data.push((
                format!("Band {}", i),
                Self::format_value(self.display_bands[i]),
                col1_x,
                start_y - row_spacing * i as f32,
                Some(self.display_bands[i]),
            ));
        }

        // Column 2: Bass/Mids/Treble
        let col2_x = center.x;
        text_data.push((
            "Bass".to_string(),
            Self::format_value(self.display_bass),
            col2_x,
            start_y,
            Some(self.display_bass),
        ));
        text_data.push((
            "Mids".to_string(),
            Self::format_value(self.display_mids),
            col2_x,
            start_y - row_spacing,
            Some(self.display_mids),
        ));
        text_data.push((
            "Treble".to_string(),
            Self::format_value(self.display_treble),
            col2_x,
            start_y - row_spacing * 2.0,
            Some(self.display_treble),
        ));

        // Column 3: Energy/Energy Diff/Transition
        let col3_x = center.x + column_spacing;
        text_data.push((
            "Energy".to_string(),
            Self::format_value(self.display_energy),
            col3_x,
            start_y,
            Some(self.display_energy),
        ));
        text_data.push((
            "Energy Diff".to_string(),
            Self::format_value(self.display_energy_diff),
            col3_x,
            start_y - row_spacing,
            Some(self.display_energy_diff.abs()), // Use absolute value for visualization
        ));
        text_data.push((
            "Transition".to_string(),
            if self.display_transition {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            },
            col3_x,
            start_y - row_spacing * 2.0,
            None, // Boolean, no indicator
        ));

        // Draw blur layers (behind text)
        for blur_idx in (0..BLUR_LAYERS).rev() {
            let offset = (blur_idx + 1) as f32 * 1.5;
            let alpha_scale = BLUR_ALPHA_DECAY.powi(blur_idx as i32);
            let base_alpha = 100.0 * self.glow_intensity * alpha_scale;

            for (label, value, x, y, numeric_value) in &text_data {
                let text_str = format!("{}: {}", label, value);
                // Use value-based color if numeric, otherwise use phosphor green
                let color = if let Some(val) = numeric_value {
                    self.value_color(*val, base_alpha)
                } else {
                    self.phosphor_color(base_alpha)
                };

                // Draw offset copies for blur effect
                draw.text(&text_str)
                    .x_y(x + offset, *y)
                    .color(color)
                    .font_size(FONT_SIZE);
                draw.text(&text_str)
                    .x_y(x - offset, *y)
                    .color(color)
                    .font_size(FONT_SIZE);
                draw.text(&text_str)
                    .x_y(*x, y + offset)
                    .color(color)
                    .font_size(FONT_SIZE);
                draw.text(&text_str)
                    .x_y(*x, y - offset)
                    .color(color)
                    .font_size(FONT_SIZE);
            }
        }

        // Draw RGB fringing (chromatic aberration)
        for (label, value, x, y, numeric_value) in &text_data {
            let text_str = format!("{}: {}", label, value);

            // Get base color from value, or use phosphor green for non-numeric
            let base_color = if let Some(val) = numeric_value {
                self.value_color(*val, 180.0 * self.glow_intensity)
            } else {
                self.phosphor_color(180.0 * self.glow_intensity)
            };

            // Extract RGB and apply channel tinting for chromatic aberration
            // Red channel (shifted left) - enhance red
            let color_r = srgba(
                base_color.red.saturating_add(40),
                base_color.green / 2,
                base_color.blue / 2,
                base_color.alpha,
            );
            draw.text(&text_str)
                .x_y(x - RGB_OFFSET, *y)
                .color(color_r)
                .font_size(FONT_SIZE);

            // Green channel (no shift - base position) - enhance green
            let color_g = srgba(
                base_color.red / 2,
                base_color.green,
                base_color.blue / 2,
                (200.0 * self.glow_intensity) as u8,
            );
            draw.text(&text_str)
                .x_y(*x, *y)
                .color(color_g)
                .font_size(FONT_SIZE);

            // Blue channel (shifted right) - enhance blue
            let color_b = srgba(
                base_color.red / 2,
                base_color.green / 2,
                base_color.blue.saturating_add(40),
                base_color.alpha,
            );
            draw.text(&text_str)
                .x_y(x + RGB_OFFSET, *y)
                .color(color_b)
                .font_size(FONT_SIZE);
        }

        // Draw main text with value-based colors
        for (label, value, x, y, numeric_value) in &text_data {
            let text_str = format!("{}: {}", label, value);
            // Use value-based color if numeric, otherwise use phosphor green
            let color = if let Some(val) = numeric_value {
                self.value_color(*val, 255.0 * self.glow_intensity)
            } else {
                self.phosphor_color(255.0 * self.glow_intensity)
            };
            draw.text(&text_str)
                .x_y(*x, *y)
                .color(color)
                .font_size(FONT_SIZE);
        }

        // Draw debug indicator lines below numeric values
        for (_, _, x, y, numeric_value) in &text_data {
            if let Some(value) = numeric_value {
                let clamped_value = value.clamp(0.0, 1.0);
                let line_length = clamped_value * indicator_width;
                let line_y = y - 19.0; // A few pixels below text to avoid overlap

                // Draw background track (dim gray line showing full range)
                draw.line()
                    .start(pt2(*x - indicator_width / 2.0, line_y))
                    .end(pt2(*x + indicator_width / 2.0, line_y))
                    .weight(1.0)
                    .color(srgba(80u8, 80u8, 80u8, 100u8));

                // Draw 2.5px line with value-based color
                let line_color = self.value_color(*value, 220.0 * self.glow_intensity);
                draw.line()
                    .start(pt2(*x - indicator_width / 2.0, line_y))
                    .end(pt2(*x - indicator_width / 2.0 + line_length, line_y))
                    .weight(2.5)
                    .color(line_color);

                // Draw calibration threshold markers
                // 0.8 threshold (yellow/red transition zone)
                let marker_0_8_x = *x - indicator_width / 2.0 + 0.8 * indicator_width;
                draw.line()
                    .start(pt2(marker_0_8_x, line_y - 4.0))
                    .end(pt2(marker_0_8_x, line_y + 4.0))
                    .weight(1.5)
                    .color(srgba(200u8, 180u8, 0u8, 180u8)); // Yellow marker

                // 0.98 threshold (saturation warning zone)
                let marker_0_98_x = *x - indicator_width / 2.0 + 0.98 * indicator_width;
                draw.line()
                    .start(pt2(marker_0_98_x, line_y - 4.0))
                    .end(pt2(marker_0_98_x, line_y + 4.0))
                    .weight(1.5)
                    .color(srgba(220u8, 50u8, 50u8, 200u8)); // Red marker
            }
        }

        // Draw scanlines (horizontal lines across entire screen)
        let num_scanlines = (bounds_h / SCANLINE_SPACING) as usize + 1;
        for i in 0..num_scanlines {
            let y = bounds.bottom() + i as f32 * SCANLINE_SPACING + self.scanline_offset;
            if y > bounds.top() {
                break;
            }

            draw.line()
                .start(pt2(bounds.left(), y))
                .end(pt2(bounds.right(), y))
                .weight(1.0)
                .color(srgba(0, 0, 0, SCANLINE_ALPHA));
        }

        // Draw CRT vignette (darker edges)
        for i in 0..15 {
            let t = i as f32 / 15.0;
            let inset = t * 60.0;
            let alpha = (t * 0.15 * 255.0) as u8;

            draw.rect()
                .x_y(center.x, center.y)
                .w_h(bounds_w - inset * 2.0, bounds_h - inset * 2.0)
                .no_fill()
                .stroke_weight(2.0)
                .stroke(srgba(0, 0, 0, alpha));
        }

        // Draw corner glow (CRT curvature effect)
        let corner_size = 100.0;
        let corners = [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)];

        for (corner_x, corner_y) in corners {
            let cx = center.x + corner_x * (bounds_w / 2.0 - corner_size / 2.0);
            let cy = center.y + corner_y * (bounds_h / 2.0 - corner_size / 2.0);

            for i in 0..8 {
                let t = i as f32 / 8.0;
                let size = corner_size * (1.0 - t * 0.6);
                let alpha = ((1.0 - t) * 0.4 * 255.0) as u8;

                draw.ellipse()
                    .x_y(cx, cy)
                    .w_h(size, size)
                    .color(srgba(0, 0, 0, alpha));
            }
        }
    }
}
