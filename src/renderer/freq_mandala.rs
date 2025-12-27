//! Frequency mandala visualization.
//!
//! 8-fold symmetry where each segment represents a frequency band.
//! Rotates with energy, creating a meditative geometric pattern.

use super::Visualization;
use nannou::prelude::*;

use crate::audio::AudioAnalysis;

/// Number of symmetry segments (matches 8 frequency bands)
const NUM_SEGMENTS: usize = 8;
/// Number of concentric rings
const NUM_RINGS: usize = if cfg!(debug_assertions) { 12 } else { 20 };

pub struct FreqMandala {
    /// Current rotation
    rotation: f32,
    /// Per-band smoothed values
    bands: [f32; 8],
    /// Hue offset for color cycling
    hue_offset: f32,
    /// Smoothed bass
    bass: f32,
    /// Smoothed energy
    energy: f32,
    /// Bloom/glow intensity
    bloom: f32,
    /// Frame counter
    frame_count: u32,
}

impl Default for FreqMandala {
    fn default() -> Self {
        Self {
            rotation: 0.0,
            bands: [0.0; 8],
            hue_offset: 0.0,
            bass: 0.0,
            energy: 0.0,
            bloom: 0.0,
            frame_count: 0,
        }
    }
}

impl FreqMandala {
    fn hsv_to_rgba(hue: f32, saturation: f32, value: f32, alpha: f32) -> Srgba<u8> {
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

        srgba(
            ((r1 + m) * 255.0) as u8,
            ((g1 + m) * 255.0) as u8,
            ((b1 + m) * 255.0) as u8,
            (alpha * 255.0) as u8,
        )
    }
}

impl Visualization for FreqMandala {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Fast attack, slow decay for each band
        let attack = 0.6;
        let decay = 0.12;

        for i in 0..8 {
            let target = analysis.bands_normalized[i];
            if target > self.bands[i] {
                self.bands[i] = self.bands[i] * (1.0 - attack) + target * attack;
            } else {
                self.bands[i] = self.bands[i] * (1.0 - decay) + target * decay;
            }
        }

        // Smooth bass and energy
        if analysis.bass > self.bass {
            self.bass = self.bass * 0.3 + analysis.bass * 0.7;
        } else {
            self.bass = self.bass * 0.85 + analysis.bass * 0.15;
        }

        self.energy = self.energy * 0.9 + analysis.energy * 0.1;

        // Rotation speeds up with energy
        self.rotation += 0.003 + self.energy * 0.02;

        // Bloom pulses with bass
        let target_bloom = self.bass * 0.5;
        self.bloom = self.bloom * 0.9 + target_bloom * 0.1;

        // Color cycling
        self.hue_offset += 0.2 + self.energy * 1.0;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0;
        let segment_angle = std::f32::consts::TAU / NUM_SEGMENTS as f32;

        // Draw concentric rings
        for ring in 0..NUM_RINGS {
            let ring_t = ring as f32 / NUM_RINGS as f32;
            let base_radius = max_radius * (0.1 + ring_t * 0.85);

            // Draw each segment of the ring
            for seg in 0..NUM_SEGMENTS {
                let band_value = self.bands[seg];
                let seg_angle = seg as f32 * segment_angle + self.rotation;

                // Ring radius modulated by band value
                let radius_mod = 1.0 + band_value * 0.3 * (1.0 - ring_t);
                let inner_radius = base_radius * 0.9 * radius_mod;
                let outer_radius = base_radius * radius_mod;

                // Color based on segment (band) and ring depth
                let hue = (seg as f32 / NUM_SEGMENTS as f32 * 360.0 + self.hue_offset) % 360.0;
                let saturation = 0.6 + band_value * 0.4;
                let value = 0.3 + band_value * 0.5 + ring_t * 0.2;
                let alpha = 0.2 + band_value * 0.4;

                let color = Self::hsv_to_rgba(hue, saturation, value, alpha);

                // Draw segment as a curved quad (approximated with triangles)
                let num_arc_points = 8;
                let half_segment = segment_angle * 0.45;

                let mut points: Vec<Vec2> = Vec::with_capacity((num_arc_points + 1) * 2);

                // Outer arc
                for i in 0..=num_arc_points {
                    let t = i as f32 / num_arc_points as f32;
                    let angle = seg_angle - half_segment + t * half_segment * 2.0;
                    points.push(pt2(
                        center.x + outer_radius * angle.cos(),
                        center.y + outer_radius * angle.sin(),
                    ));
                }

                // Inner arc (reverse order)
                for i in (0..=num_arc_points).rev() {
                    let t = i as f32 / num_arc_points as f32;
                    let angle = seg_angle - half_segment + t * half_segment * 2.0;
                    points.push(pt2(
                        center.x + inner_radius * angle.cos(),
                        center.y + inner_radius * angle.sin(),
                    ));
                }

                draw.polygon().points(points).color(color);

                // Add highlight lines on active bands
                if band_value > 0.3 {
                    let line_alpha = (band_value - 0.3) * 0.8;
                    let line_color = Self::hsv_to_rgba(hue, 0.3, 0.95, line_alpha);

                    let angle = seg_angle;
                    let start = pt2(
                        center.x + inner_radius * angle.cos(),
                        center.y + inner_radius * angle.sin(),
                    );
                    let end = pt2(
                        center.x + outer_radius * angle.cos(),
                        center.y + outer_radius * angle.sin(),
                    );

                    draw.line()
                        .start(start)
                        .end(end)
                        .weight(1.0 + band_value * 2.0)
                        .color(line_color);
                }
            }
        }

        // Draw outer rim with pulsing glow
        let rim_radius = max_radius * 0.95;
        for i in 0..5 {
            let t = i as f32 / 5.0;
            let r = rim_radius + t * max_radius * 0.05 * (1.0 + self.bloom);
            let alpha = (1.0 - t) * 0.2 * (0.5 + self.energy);
            let hue = (self.hue_offset + t * 30.0) % 360.0;
            let color = Self::hsv_to_rgba(hue, 0.5, 0.8, alpha);

            draw.ellipse()
                .xy(center)
                .radius(r)
                .no_fill()
                .stroke_weight(2.0)
                .stroke(color);
        }

        // Center mandala eye
        let core_radius = max_radius * 0.1 * (1.0 + self.bass * 0.3);

        // Inner glow rings
        for i in 0..8 {
            let t = i as f32 / 8.0;
            let r = core_radius * (1.0 - t * 0.7);
            let hue = (self.hue_offset + t * 45.0 + 180.0) % 360.0;
            let alpha = (1.0 - t) * 0.5;
            let color = Self::hsv_to_rgba(hue, 0.7, 0.9, alpha);

            draw.ellipse().xy(center).radius(r).color(color);
        }

        // Petal pattern around center
        let petal_radius = core_radius * 1.5;
        for seg in 0..NUM_SEGMENTS {
            let band_value = self.bands[seg];
            let angle = seg as f32 * segment_angle + self.rotation * 2.0;

            let petal_length = petal_radius * (0.5 + band_value * 0.5);
            let petal_width = core_radius * 0.3 * (0.5 + band_value * 0.5);

            let tip_x = center.x + petal_length * angle.cos();
            let tip_y = center.y + petal_length * angle.sin();

            let perp_angle = angle + std::f32::consts::FRAC_PI_2;
            let base_offset_x = petal_width * perp_angle.cos();
            let base_offset_y = petal_width * perp_angle.sin();

            let hue = (seg as f32 / NUM_SEGMENTS as f32 * 360.0 + self.hue_offset + 60.0) % 360.0;
            let alpha = 0.4 + band_value * 0.4;
            let color = Self::hsv_to_rgba(hue, 0.8, 0.8, alpha);

            let points = vec![
                pt2(center.x + base_offset_x, center.y + base_offset_y),
                pt2(tip_x, tip_y),
                pt2(center.x - base_offset_x, center.y - base_offset_y),
            ];

            draw.polygon().points(points).color(color);
        }
    }
}
