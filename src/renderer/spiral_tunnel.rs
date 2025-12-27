//! Hypnotic spiral tunnel visualization.
//!
//! Rotating spiral rings that zoom/pulse with bass kicks, creating an
//! infinite tunnel effect.

use super::Visualization;
use nannou::prelude::*;

use crate::audio::AudioAnalysis;

/// Number of rings in the tunnel
const NUM_RINGS: usize = if cfg!(debug_assertions) { 40 } else { 80 };
/// Number of spiral arms
const NUM_ARMS: usize = 3;

pub struct SpiralTunnel {
    /// Rotation angle
    rotation: f32,
    /// Zoom factor (pulsing with bass)
    zoom: f32,
    /// Tunnel depth offset (creates forward motion)
    depth_offset: f32,
    /// Hue offset for color cycling
    hue_offset: f32,
    /// Smoothed bass
    bass: f32,
    /// Smoothed treble
    treble: f32,
    /// Smoothed energy
    energy: f32,
    /// Frame counter
    frame_count: u32,
}

impl SpiralTunnel {
    pub fn new() -> Self {
        Self {
            rotation: 0.0,
            zoom: 1.0,
            depth_offset: 0.0,
            hue_offset: 0.0,
            bass: 0.0,
            treble: 0.0,
            energy: 0.0,
            frame_count: 0,
        }
    }

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

impl Visualization for SpiralTunnel {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Fast attack, slow decay for techno responsiveness
        let attack = 0.7;
        let decay = 0.15;

        if analysis.bass > self.bass {
            self.bass = self.bass * (1.0 - attack) + analysis.bass * attack;
        } else {
            self.bass = self.bass * (1.0 - decay) + analysis.bass * decay;
        }

        self.treble = self.treble * 0.8 + analysis.treble * 0.2;
        self.energy = self.energy * 0.9 + analysis.energy * 0.1;

        // Rotation speeds up with energy
        self.rotation += 0.01 + self.energy * 0.03;

        // Zoom pulses with bass (creates "punch" effect)
        let target_zoom = 1.0 + self.bass * 0.4;
        self.zoom = self.zoom * 0.85 + target_zoom * 0.15;

        // Forward motion through tunnel
        self.depth_offset += 0.02 + self.energy * 0.05;
        if self.depth_offset > 1.0 {
            self.depth_offset -= 1.0;
        }

        // Color cycling
        self.hue_offset += 0.5 + self.energy * 2.0;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0;

        // Draw rings from back to front (largest to smallest)
        for i in 0..NUM_RINGS {
            let t = i as f32 / NUM_RINGS as f32;
            // Add depth offset for forward motion
            let adjusted_t = (t + self.depth_offset) % 1.0;

            // Radius shrinks toward center (perspective)
            let depth_factor = 1.0 - adjusted_t;
            let radius = max_radius * depth_factor * self.zoom;

            if radius < 5.0 {
                continue;
            }

            // Spiral twist increases with depth
            let spiral_angle = self.rotation + adjusted_t * std::f32::consts::TAU * 2.0;

            // Color based on depth and hue offset
            let hue = (adjusted_t * 120.0 + self.hue_offset) % 360.0;
            let saturation = 0.7 + self.treble * 0.3;
            let value = 0.4 + depth_factor * 0.4 + self.bass * 0.2;
            let alpha = 0.3 + depth_factor * 0.4;

            // Draw spiral arms as arcs
            for arm in 0..NUM_ARMS {
                let arm_offset = arm as f32 * std::f32::consts::TAU / NUM_ARMS as f32;
                let arm_angle = spiral_angle + arm_offset;

                // Draw arc segment for this arm
                let arc_length = std::f32::consts::TAU / NUM_ARMS as f32 * 0.8;
                let num_points = 20;

                let points: Vec<Vec2> = (0..=num_points)
                    .map(|j| {
                        let arc_t = j as f32 / num_points as f32;
                        let angle = arm_angle + arc_t * arc_length;
                        let wobble = (angle * 3.0 + self.frame_count as f32 * 0.05).sin()
                            * self.treble * 5.0;
                        let r = radius + wobble;
                        pt2(center.x + r * angle.cos(), center.y + r * angle.sin())
                    })
                    .collect();

                let arm_hue = (hue + arm as f32 * 40.0) % 360.0;
                let color = Self::hsv_to_rgba(arm_hue, saturation, value, alpha);
                let weight = 2.0 + depth_factor * 4.0 + self.bass * 3.0;

                draw.polyline().weight(weight).points(points).color(color);
            }
        }

        // Center glow (vanishing point)
        let glow_radius = max_radius * 0.08 * (1.0 + self.bass * 0.5);
        for i in 0..8 {
            let t = i as f32 / 8.0;
            let r = glow_radius * (1.0 - t);
            let hue = (self.hue_offset + t * 60.0) % 360.0;
            let alpha = (1.0 - t) * 0.4 * (0.5 + self.bass * 0.5);
            let color = Self::hsv_to_rgba(hue, 0.8, 0.9, alpha);

            draw.ellipse().xy(center).radius(r).color(color);
        }
    }
}
