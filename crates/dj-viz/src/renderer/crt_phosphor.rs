//! CRT monitor phosphor decay simulation.
//!
//! Emulates cathode ray tube display characteristics with phosphor persistence
//! and scan line flickering.

use super::Visualization;
use nannou::prelude::*;

use crate::audio::AudioAnalysis;

/// Number of points in the waveform
const WAVEFORM_POINTS: usize = if cfg!(debug_assertions) { 128 } else { 256 };
/// History length for phosphor trails
const TRAIL_LENGTH: usize = if cfg!(debug_assertions) { 8 } else { 16 };
/// Phosphor decay rate per frame
const PHOSPHOR_DECAY: f32 = 0.85;

#[derive(Clone)]
struct TrailPoint {
    x: f32,
    y: f32,
    brightness: f32,
}

pub struct CrtPhosphor {
    /// Waveform history for trails
    waveform_history: Vec<Vec<TrailPoint>>,
    /// Current waveform points
    current_waveform: Vec<TrailPoint>,
    /// Beam intensity (affected by audio)
    beam_intensity: f32,
    /// Horizontal sweep phase
    sweep_phase: f32,
    /// Vertical center offset (audio reactive)
    vertical_offset: f32,
    /// Phosphor color hue (cycles slowly)
    hue: f32,
    /// Bass for bloom effect
    bass: f32,
    /// Treble for beam focus
    treble: f32,
    /// Energy for overall brightness
    energy: f32,
    /// Frame counter
    frame_count: u32,
    /// Lissajous mode toggle based on transitions
    lissajous_mode: bool,
    /// Lissajous phase
    lissajous_phase: f32,
}

impl Default for CrtPhosphor {
    fn default() -> Self {
        Self {
            waveform_history: vec![Vec::new(); TRAIL_LENGTH],
            current_waveform: Vec::with_capacity(WAVEFORM_POINTS),
            beam_intensity: 0.5,
            sweep_phase: 0.0,
            vertical_offset: 0.0,
            hue: 120.0, // Start with classic green
            bass: 0.0,
            treble: 0.0,
            energy: 0.0,
            frame_count: 0,
            lissajous_mode: false,
            lissajous_phase: 0.0,
        }
    }
}

impl CrtPhosphor {
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

    /// Get phosphor color with proper CRT phosphor characteristics
    fn phosphor_color(&self, brightness: f32, age: f32) -> Srgba<u8> {
        // Phosphor color shifts slightly as it decays (green -> yellow-green for P31)
        let age_hue_shift = age * 20.0;
        let hue = self.hue + age_hue_shift;

        // Saturation decreases as phosphor decays
        let saturation = 0.8 - age * 0.3;

        // Value based on brightness and decay
        let value = brightness * (1.0 - age * 0.5);

        let (r, g, b) = Self::hsv_to_rgb(hue, saturation, value);

        // Alpha for bloom blending
        let alpha = brightness * (1.0 - age * 0.7) * 0.8;

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (alpha.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    /// Draw bloom/glow effect around a point
    fn draw_bloom(&self, draw: &Draw, x: f32, y: f32, brightness: f32, age: f32) {
        let bloom_size = 3.0 + brightness * 8.0 * (1.0 + self.bass * 0.5);
        let num_rings = 4;

        for i in 0..num_rings {
            let t = i as f32 / num_rings as f32;
            let radius = bloom_size * (1.0 - t * 0.3);
            let ring_brightness = brightness * (1.0 - t) * 0.5;
            let color = self.phosphor_color(ring_brightness, age);

            draw.ellipse().x_y(x, y).radius(radius).color(color);
        }
    }
}

impl Visualization for CrtPhosphor {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Smooth audio tracking
        self.bass = self.bass * 0.7 + analysis.bass * 0.3;
        self.treble = self.treble * 0.8 + analysis.treble * 0.2;
        self.energy = self.energy * 0.9 + analysis.energy * 0.1;

        // Beam intensity follows energy
        self.beam_intensity = 0.3 + self.energy * 0.7;

        // Vertical offset wobbles with bass
        self.vertical_offset = self.vertical_offset * 0.9 + analysis.bass * 50.0 * 0.1;

        // Hue slowly cycles, faster with more energy
        self.hue += 0.1 + self.energy * 0.5;
        if self.hue > 360.0 {
            self.hue -= 360.0;
        }

        // Toggle lissajous mode on transitions
        if analysis.transition_detected {
            self.lissajous_mode = !self.lissajous_mode;
        }

        // Lissajous phase advances
        self.lissajous_phase += 0.02 + analysis.mids * 0.05;

        // Shift history (oldest at back)
        self.waveform_history.remove(0);

        // Decay existing trail brightnesses
        for trail in &mut self.waveform_history {
            for point in trail.iter_mut() {
                point.brightness *= PHOSPHOR_DECAY;
            }
        }

        // Add current waveform to history
        self.waveform_history.push(self.current_waveform.clone());

        // Generate new waveform
        self.current_waveform.clear();
        self.sweep_phase += 0.05 + self.treble * 0.1;

        for i in 0..WAVEFORM_POINTS {
            let t = i as f32 / WAVEFORM_POINTS as f32;

            if self.lissajous_mode {
                // Lissajous pattern
                let freq_x = 3.0 + self.bass * 2.0;
                let freq_y = 2.0 + self.treble * 2.0;
                let phase = self.lissajous_phase;

                let x = (t * std::f32::consts::TAU * freq_x + phase).sin();
                let y = (t * std::f32::consts::TAU * freq_y).sin();

                // Modulate with audio bands
                let band_idx = (t * 7.0) as usize;
                let modulation = analysis.bands_normalized[band_idx.min(7)];

                self.current_waveform.push(TrailPoint {
                    x: x * (0.7 + modulation * 0.3),
                    y: y * (0.7 + modulation * 0.3),
                    brightness: self.beam_intensity * (0.5 + modulation * 0.5),
                });
            } else {
                // Classic oscilloscope waveform
                // X sweeps across screen
                let x = t * 2.0 - 1.0;

                // Y is audio waveform (synthesized from bands)
                let mut y = 0.0;
                for (band_idx, &band) in analysis.bands_normalized.iter().enumerate() {
                    let freq = (band_idx + 1) as f32;
                    let phase = self.sweep_phase * freq;
                    y += band * (t * std::f32::consts::TAU * freq + phase).sin() * 0.3;
                }

                // Add some noise based on treble
                y += self.treble * ((t * 50.0 + self.sweep_phase * 10.0).sin() * 0.1);

                self.current_waveform.push(TrailPoint {
                    x,
                    y: y.clamp(-1.0, 1.0),
                    brightness: self.beam_intensity,
                });
            }
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let w = bounds.w();
        let h = bounds.h();
        let center = bounds.xy();

        // Scale factors
        let scale_x = w * 0.45;
        let scale_y = h * 0.4;

        // Draw phosphor trails (oldest first for proper layering)
        for (trail_idx, trail) in self.waveform_history.iter().enumerate() {
            let age = 1.0 - (trail_idx as f32 / TRAIL_LENGTH as f32);

            // Draw trail with lines connecting points
            if trail.len() > 1 {
                for i in 0..trail.len() - 1 {
                    let p1 = &trail[i];
                    let p2 = &trail[i + 1];

                    if p1.brightness < 0.05 && p2.brightness < 0.05 {
                        continue;
                    }

                    let x1 = center.x + p1.x * scale_x;
                    let y1 = center.y + p1.y * scale_y + self.vertical_offset;
                    let x2 = center.x + p2.x * scale_x;
                    let y2 = center.y + p2.y * scale_y + self.vertical_offset;

                    let avg_brightness = (p1.brightness + p2.brightness) / 2.0;
                    let color = self.phosphor_color(avg_brightness, age);

                    // Line thickness based on beam intensity
                    let thickness = 1.0 + avg_brightness * 2.0 * (1.0 - age * 0.5);

                    draw.line()
                        .start(pt2(x1, y1))
                        .end(pt2(x2, y2))
                        .weight(thickness)
                        .color(color);
                }
            }
        }

        // Draw current waveform with bloom
        if self.current_waveform.len() > 1 {
            for i in 0..self.current_waveform.len() - 1 {
                let p1 = &self.current_waveform[i];
                let p2 = &self.current_waveform[i + 1];

                let x1 = center.x + p1.x * scale_x;
                let y1 = center.y + p1.y * scale_y + self.vertical_offset;
                let x2 = center.x + p2.x * scale_x;
                let y2 = center.y + p2.y * scale_y + self.vertical_offset;

                let avg_brightness = (p1.brightness + p2.brightness) / 2.0;

                // Draw bloom at points
                if i % 4 == 0 {
                    self.draw_bloom(draw, x1, y1, avg_brightness, 0.0);
                }

                // Bright core line
                let color = self.phosphor_color(avg_brightness * 1.2, 0.0);
                let thickness = 2.0 + avg_brightness * 3.0;

                draw.line()
                    .start(pt2(x1, y1))
                    .end(pt2(x2, y2))
                    .weight(thickness)
                    .color(color);
            }
        }

        // Draw CRT bezel/frame effect
        let _bezel_color = srgba(20u8, 20, 25, 200);
        let _bezel_width = 10.0;

        // Corner radius effect (darker corners)
        let corner_size = 80.0;
        for corner in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            let cx = center.x + corner.0 * (w / 2.0 - corner_size / 2.0);
            let cy = center.y + corner.1 * (h / 2.0 - corner_size / 2.0);

            for i in 0..5 {
                let t = i as f32 / 5.0;
                let size = corner_size * (1.0 - t * 0.5);
                let alpha = (1.0 - t) * 0.3;

                draw.ellipse()
                    .x_y(cx, cy)
                    .w_h(size, size)
                    .color(srgba(0, 0, 0, (alpha * 255.0) as u8));
            }
        }

        // Subtle scanlines
        let num_scanlines = (h / 4.0) as usize;
        for i in 0..num_scanlines {
            let y = bounds.bottom() + i as f32 * 4.0;
            draw.rect()
                .x_y(center.x, y)
                .w_h(w, 1.0)
                .color(srgba(0u8, 0, 0, 20));
        }

        // Screen curvature vignette
        for i in 0..20 {
            let t = i as f32 / 20.0;
            let inset = t * 50.0;
            let alpha = t * 0.1;

            draw.rect()
                .xy(center)
                .w_h(w - inset * 2.0, h - inset * 2.0)
                .no_fill()
                .stroke_weight(2.0)
                .stroke(srgba(0, 0, 0, (alpha * 255.0) as u8));
        }
    }
}
