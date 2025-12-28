//! Psychedelic spiral tunnel visualization.
//!
//! Creates swirling, morphing spiral patterns with rainbow color cycling
//! and tunnel depth effects driven by audio energy.

use super::Visualization;
use nannou::prelude::*;

use crate::audio::AudioAnalysis;

/// Number of spiral arms
const NUM_ARMS: usize = 5;
/// Points per arm for smooth curves
const POINTS_PER_ARM: usize = if cfg!(debug_assertions) { 40 } else { 60 };
/// Number of concentric rings for tunnel effect
const NUM_RINGS: usize = if cfg!(debug_assertions) { 12 } else { 18 };

pub struct PsychedelicSpiral {
    /// Spiral rotation angle
    rotation: f32,
    /// Tunnel depth offset (scrolls inward)
    depth_offset: f32,
    /// Morphing phase for shape distortion
    morph_phase: f32,
    /// Color hue offset cycling through rainbow
    hue_offset: f32,
    /// Bass level for pulsing effects
    bass: f32,
    /// Treble for sparkle/detail
    treble: f32,
    /// Mid frequencies for color saturation
    mids: f32,
    /// Overall energy for speed
    energy: f32,
    /// Frame counter
    frame_count: u32,
}

impl Default for PsychedelicSpiral {
    fn default() -> Self {
        Self {
            rotation: 0.0,
            depth_offset: 0.0,
            morph_phase: 0.0,
            hue_offset: 0.0,
            bass: 0.0,
            treble: 0.0,
            mids: 0.0,
            energy: 0.0,
            frame_count: 0,
        }
    }
}

impl PsychedelicSpiral {
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

impl Visualization for PsychedelicSpiral {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Smooth audio tracking
        self.bass = self.bass * 0.7 + analysis.bass * 0.3;
        self.treble = self.treble * 0.8 + analysis.treble * 0.2;
        self.mids = self.mids * 0.75 + analysis.mids * 0.25;
        self.energy = self.energy * 0.85 + analysis.energy * 0.15;

        // Rotation speed syncs to BPM when available, falls back to energy-based
        let rotation_speed = if analysis.bpm > 0.0 {
            // Rotate proportional to BPM (faster at higher BPM)
            (analysis.bpm / 120.0) * 0.025
        } else {
            0.015 + self.energy * 0.04
        };
        self.rotation += rotation_speed;

        // Tunnel scrolls inward with bass pulses
        self.depth_offset += 0.02 + self.bass * 0.08;
        if self.depth_offset > 1.0 {
            self.depth_offset -= 1.0;
        }

        // Morphing phase for wobbly distortion
        self.morph_phase += 0.03 + self.treble * 0.05;

        // Rainbow cycling - faster with more energy
        self.hue_offset += 1.0 + self.energy * 3.0;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0;

        // Draw tunnel rings from back to front
        for ring in (0..NUM_RINGS).rev() {
            let ring_t = (ring as f32 + self.depth_offset) / NUM_RINGS as f32;
            let ring_radius = max_radius * (0.1 + ring_t * 0.9);

            // Rings closer to center (deeper) are more transparent
            let depth_alpha = 0.3 + ring_t * 0.5;

            // Wobble amount increases toward center
            let wobble = (1.0 - ring_t) * 0.15 * (1.0 + self.bass * 0.5);

            // Draw ring segments
            let segments = 36;
            for seg in 0..segments {
                let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
                let next_angle = ((seg + 1) as f32 / segments as f32) * std::f32::consts::TAU;

                // Wobble distortion
                let wobble_offset = (angle * 3.0 + self.morph_phase).sin() * wobble * ring_radius;
                let next_wobble = (next_angle * 3.0 + self.morph_phase).sin() * wobble * ring_radius;

                let r1 = ring_radius + wobble_offset;
                let r2 = ring_radius + next_wobble;

                // Color varies by angle and depth
                let hue = (self.hue_offset + angle.to_degrees() + ring_t * 120.0) % 360.0;
                let saturation = 0.7 + self.mids * 0.3;
                let value = 0.5 + ring_t * 0.4;
                let color = Self::hsv_to_rgba(hue, saturation, value, depth_alpha * 0.6);

                let x1 = center.x + r1 * (angle + self.rotation).cos();
                let y1 = center.y + r1 * (angle + self.rotation).sin();
                let x2 = center.x + r2 * (next_angle + self.rotation).cos();
                let y2 = center.y + r2 * (next_angle + self.rotation).sin();

                draw.line()
                    .start(pt2(x1, y1))
                    .end(pt2(x2, y2))
                    .weight(2.0 + ring_t * 3.0)
                    .color(color);
            }
        }

        // Draw spiral arms
        for arm in 0..NUM_ARMS {
            let arm_offset = (arm as f32 / NUM_ARMS as f32) * std::f32::consts::TAU;

            let points: Vec<Point2> = (0..POINTS_PER_ARM)
                .map(|i| {
                    let t = i as f32 / POINTS_PER_ARM as f32;

                    // Logarithmic spiral
                    let spiral_angle = arm_offset + self.rotation + t * std::f32::consts::TAU * 2.5;
                    let base_radius = max_radius * t;

                    // Add psychedelic wobble
                    let wobble = (t * 8.0 + self.morph_phase).sin() * 20.0 * (1.0 + self.bass);
                    let radius = base_radius + wobble;

                    let x = center.x + radius * spiral_angle.cos();
                    let y = center.y + radius * spiral_angle.sin();
                    pt2(x, y)
                })
                .collect();

            // Draw the spiral arm with varying colors
            for i in 0..points.len().saturating_sub(1) {
                let t = i as f32 / POINTS_PER_ARM as f32;
                let hue = (self.hue_offset + arm as f32 * 72.0 + t * 180.0) % 360.0;
                let saturation = 0.8 + self.treble * 0.2;
                let value = 0.6 + t * 0.3;
                let alpha = 0.4 + t * 0.4;

                let color = Self::hsv_to_rgba(hue, saturation, value, alpha);
                let weight = 2.0 + t * 6.0 * (1.0 + self.bass * 0.3);

                draw.line()
                    .start(points[i])
                    .end(points[i + 1])
                    .weight(weight)
                    .color(color);
            }
        }

        // Central vortex glow
        let glow_layers = 8;
        for i in 0..glow_layers {
            let t = i as f32 / glow_layers as f32;
            let radius = max_radius * 0.15 * (1.0 - t) * (1.0 + self.bass * 0.5);
            let hue = (self.hue_offset + t * 90.0) % 360.0;
            let alpha = t * 0.3;
            let color = Self::hsv_to_rgba(hue, 0.9, 1.0, alpha);

            draw.ellipse()
                .xy(center)
                .radius(radius)
                .color(color);
        }

        // Pulsing outer ring on strong bass
        if self.bass > 0.5 {
            let pulse_alpha = (self.bass - 0.5) * 0.6;
            let pulse_radius = max_radius * (0.95 + self.bass * 0.1);
            let hue = (self.hue_offset + 180.0) % 360.0;
            let color = Self::hsv_to_rgba(hue, 1.0, 1.0, pulse_alpha);

            draw.ellipse()
                .xy(center)
                .radius(pulse_radius)
                .no_fill()
                .stroke(color)
                .stroke_weight(3.0 + self.bass * 4.0);
        }
    }
}
