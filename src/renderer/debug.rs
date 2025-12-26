//! Debug visualization.
//!
//! Displays all audio analysis inputs in a retro CRT terminal style with
//! RGB chromatic aberration, scanlines, phosphor glow, vignette effects,
//! and calibration threshold indicators.

use super::Visualization;
use nannou::prelude::*;
use std::time::Instant;

use crate::audio::AudioAnalysis;

/// Base font size for numbers
const FONT_SIZE: u32 = 24;
/// Green CRT phosphor (classic terminal)
const PHOSPHOR_HUE: f32 = 120.0;
/// Update numbers every N frames (reduces flicker)
const UPDATE_INTERVAL: u32 = 2;

pub struct DebugViz {
    /// Frame counter
    frame_count: u32,
    /// Cached display values (updated every UPDATE_INTERVAL frames)
    display_bands: [f32; 8],
    /// Normalized bands relative to tracked min/max (can be outside 0-1)
    display_bands_normalized: [f32; 8],
    display_bass: f32,
    display_mids: f32,
    display_treble: f32,
    display_energy: f32,
    display_energy_diff: f32,
    display_transition: bool,
    display_zoom_shift: bool,
    display_bpm: f32,
    display_dominant_band: usize,
    display_last_mark: u32,
    display_viz_change: bool,
    /// Tracked min values for each band (for visualization)
    /// Tracked min values for each band (from analyzer)
    display_band_mins: [f32; 8],
    /// Tracked max values for each band (from analyzer)
    display_band_maxs: [f32; 8],
    /// Phosphor glow intensity
    glow_intensity: f32,
    /// Last frame time for FPS calculation
    last_frame_time: Instant,
    /// Smoothed FPS display value
    display_fps: f32,
}

impl DebugViz {
    /// Create a new debug visualization
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            display_bands: [0.0; 8],
            display_bands_normalized: [0.0; 8],
            display_bass: 0.0,
            display_mids: 0.0,
            display_treble: 0.0,
            display_energy: 0.0,
            display_energy_diff: 0.0,
            display_transition: false,
            display_zoom_shift: false,
            display_bpm: 0.0,
            display_dominant_band: 0,
            display_last_mark: 600,
            display_viz_change: false,
            display_band_mins: [0.0; 8],
            display_band_maxs: [0.0; 8],
            glow_intensity: 0.5,
            last_frame_time: Instant::now(),
            display_fps: 0.0,
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

        // Calculate FPS
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame_time).as_secs_f32();
        let current_fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };
        // Smooth FPS with exponential moving average
        self.display_fps = self.display_fps * 0.9 + current_fps * 0.1;
        self.last_frame_time = now;

        // Update display values every UPDATE_INTERVAL frames
        if self.frame_count % UPDATE_INTERVAL == 0 {
            self.display_bands = analysis.bands;
            self.display_bands_normalized = analysis.bands_normalized;
            self.display_band_mins = analysis.band_mins;
            self.display_band_maxs = analysis.band_maxs;
            self.display_bass = analysis.bass;
            self.display_mids = analysis.mids;
            self.display_treble = analysis.treble;
            self.display_energy = analysis.energy;
            self.display_energy_diff = analysis.energy_diff;
            self.display_transition = analysis.transition_detected;
            self.display_zoom_shift = analysis.zoom_direction_shift;
            self.display_bpm = analysis.bpm;
            self.display_dominant_band = analysis.dominant_band;
            self.display_last_mark = analysis.last_mark;
            self.display_viz_change = analysis.viz_change_triggered;
        }

        // Update tracked min/max for each band with slow decay towards current value
        const MIN_TRACK_RATE: f32 = 0.995; // Very slow tracking
        const MAX_TRACK_RATE: f32 = 0.995;

        for i in 0..8 {
            let current = analysis.bands[i];

            // Track minimum - if current is lower, use it; otherwise slowly drift towards current
            if current < self.display_band_mins[i] || self.display_band_mins[i] == 0.0 {
                self.display_band_mins[i] = current;
            } else {
                self.display_band_mins[i] =
                    self.display_band_mins[i] * MIN_TRACK_RATE + current * (1.0 - MIN_TRACK_RATE);
            }

            // Track maximum - if current is higher, use it; otherwise slowly drift towards current
            if current > self.display_band_maxs[i] {
                self.display_band_maxs[i] = current;
            } else {
                self.display_band_maxs[i] =
                    self.display_band_maxs[i] * MAX_TRACK_RATE + current * (1.0 - MAX_TRACK_RATE);
            }
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

        // Draw FPS at top left
        let fps_text = format!("FPS: {:.0}", self.display_fps);
        let fps_x = bounds.left() + 60.0;
        let fps_y = bounds.top() - 30.0;
        let fps_color = self.phosphor_color(255.0 * self.glow_intensity);
        draw.text(&fps_text)
            .x_y(fps_x, fps_y)
            .color(fps_color)
            .font_size(FONT_SIZE);

        // Layout configuration
        let column_spacing = bounds_w / 5.0;
        let row_spacing = 30.0;
        let start_y = bounds.top() - 50.0;
        let indicator_width = 120.0; // Max width for value indicators

        // Prepare text data: (label, value_str, x, y, numeric_value, is_normalized)
        // is_normalized: true means value can be outside 0-1, indicator should show differently
        let mut text_data: Vec<(String, String, f32, f32, Option<f32>, bool)> = Vec::new();

        // Column 1: Frequency bands (raw values)
        let col1_x = center.x - column_spacing * 1.5;
        for i in 0..8 {
            text_data.push((
                format!("Band {}", i),
                Self::format_value(self.display_bands[i]),
                col1_x,
                start_y - row_spacing * i as f32,
                Some(self.display_bands[i]),
                false,
            ));
        }

        // Column 2: Normalized bands (relative to min/max)
        let col2_x = center.x - column_spacing * 0.5;
        for i in 0..8 {
            text_data.push((
                format!("Norm {}", i),
                Self::format_value(self.display_bands_normalized[i]),
                col2_x,
                start_y - row_spacing * i as f32,
                Some(self.display_bands_normalized[i]),
                true,
            ));
        }

        // Column 3: Bass/Mids/Treble
        let col3_x = center.x + column_spacing * 0.5;
        text_data.push((
            "Bass".to_string(),
            Self::format_value(self.display_bass),
            col3_x,
            start_y,
            Some(self.display_bass),
            false,
        ));
        text_data.push((
            "Mids".to_string(),
            Self::format_value(self.display_mids),
            col3_x,
            start_y - row_spacing,
            Some(self.display_mids),
            false,
        ));
        text_data.push((
            "Treble".to_string(),
            Self::format_value(self.display_treble),
            col3_x,
            start_y - row_spacing * 2.0,
            Some(self.display_treble),
            false,
        ));

        // Column 4: Energy/Energy Diff/Transition
        let col4_x = center.x + column_spacing * 1.5;
        text_data.push((
            "Energy".to_string(),
            Self::format_value(self.display_energy),
            col4_x,
            start_y,
            Some(self.display_energy),
            false,
        ));
        text_data.push((
            "Energy Diff".to_string(),
            Self::format_value(self.display_energy_diff),
            col4_x,
            start_y - row_spacing,
            Some(self.display_energy_diff.abs()), // Use absolute value for visualization
            false,
        ));
        text_data.push((
            "Transition".to_string(),
            if self.display_transition {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            },
            col4_x,
            start_y - row_spacing * 2.0,
            None, // Boolean, no indicator
            false,
        ));
        text_data.push((
            "Zoom Shift".to_string(),
            if self.display_zoom_shift {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            },
            col4_x,
            start_y - row_spacing * 3.0,
            None, // Boolean, no indicator
            false,
        ));
        text_data.push((
            "BPM".to_string(),
            if self.display_bpm > 0.0 {
                format!("{:.0}", self.display_bpm)
            } else {
                "---".to_string()
            },
            col3_x,
            start_y - row_spacing * 4.0,
            None, // No indicator for BPM
            false,
        ));
        text_data.push((
            "Dominant Band".to_string(),
            format!("{}", self.display_dominant_band),
            col3_x,
            start_y - row_spacing * 5.0,
            None, // No indicator for dominant band
            false,
        ));
        text_data.push((
            "Last Mark".to_string(),
            format!("{}", self.display_last_mark),
            col3_x,
            start_y - row_spacing * 6.0,
            None, // No indicator for last mark
            false,
        ));
        text_data.push((
            "Viz Change".to_string(),
            if self.display_viz_change {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            },
            col3_x,
            start_y - row_spacing * 7.0,
            None, // Boolean, no indicator
            false,
        ));

        // Draw main text with value-based colors
        for (label, value, x, y, numeric_value, is_normalized) in &text_data {
            let text_str = format!("{}: {}", label, value);
            // Use value-based color if numeric, otherwise use phosphor green
            // For normalized values, clamp to 0-1 for color purposes
            let color = if let Some(val) = numeric_value {
                let display_val = if *is_normalized {
                    val.clamp(0.0, 1.0)
                } else {
                    *val
                };
                self.value_color(display_val, 255.0 * self.glow_intensity)
            } else {
                self.phosphor_color(255.0 * self.glow_intensity)
            };
            draw.text(&text_str)
                .x_y(*x, *y)
                .color(color)
                .font_size(FONT_SIZE);
        }

        // Draw debug indicator lines below numeric values
        for (idx, (_, _, x, y, numeric_value, is_normalized)) in text_data.iter().enumerate() {
            if let Some(value) = numeric_value {
                let line_y = y - 19.0; // A few pixels below text to avoid overlap

                if *is_normalized {
                    // For normalized values: draw a centered indicator that can extend beyond 0-1
                    // Center is at 0.5 of the indicator, representing normalized value of 0.5
                    let center_x = *x;

                    // Draw background track
                    draw.line()
                        .start(pt2(*x - indicator_width / 2.0, line_y))
                        .end(pt2(*x + indicator_width / 2.0, line_y))
                        .weight(1.0)
                        .color(srgba(80u8, 80u8, 80u8, 100u8));

                    // Draw center marker (where 0.5 is)
                    draw.line()
                        .start(pt2(center_x, line_y - 3.0))
                        .end(pt2(center_x, line_y + 3.0))
                        .weight(1.0)
                        .color(srgba(100u8, 100u8, 100u8, 150u8));

                    // Draw value indicator - clamped to fit within indicator width
                    // 0.0 normalized = left edge, 1.0 normalized = right edge
                    let clamped_norm = value.clamp(0.0, 1.0);
                    let value_x = *x - indicator_width / 2.0 + clamped_norm * indicator_width;
                    let line_color = self.value_color(clamped_norm, 220.0 * self.glow_intensity);
                    draw.line()
                        .start(pt2(*x - indicator_width / 2.0, line_y))
                        .end(pt2(value_x, line_y))
                        .weight(2.5)
                        .color(line_color);
                } else {
                    // Standard 0-1 indicator for non-normalized values
                    let clamped_value = value.clamp(0.0, 1.0);
                    let line_length = clamped_value * indicator_width;

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

                    // Draw tracked min/max markers for frequency bands (first 8 entries)
                    if idx < 8 {
                        let band_idx = idx;

                        // Yellow marker for tracked minimum
                        let min_val = self.display_band_mins[band_idx].clamp(0.0, 1.0);
                        let marker_min_x = *x - indicator_width / 2.0 + min_val * indicator_width;
                        draw.line()
                            .start(pt2(marker_min_x, line_y - 4.0))
                            .end(pt2(marker_min_x, line_y + 4.0))
                            .weight(1.5)
                            .color(srgba(200u8, 180u8, 0u8, 180u8)); // Yellow marker

                        // Red marker for tracked maximum
                        let max_val = self.display_band_maxs[band_idx].clamp(0.0, 1.0);
                        let marker_max_x = *x - indicator_width / 2.0 + max_val * indicator_width;
                        draw.line()
                            .start(pt2(marker_max_x, line_y - 4.0))
                            .end(pt2(marker_max_x, line_y + 4.0))
                            .weight(1.5)
                            .color(srgba(220u8, 50u8, 50u8, 200u8)); // Red marker
                    }
                }
            }
        }
    }
}
