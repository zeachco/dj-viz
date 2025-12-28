//! Symmetric kaleidoscope pattern visualization.
//!
//! Creates radially symmetric patterns with frequency-driven geometry and
//! color cycling.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

/// Number of mirror segments in the kaleidoscope
const NUM_SEGMENTS: usize = 6;
/// Number of particles (not per segment - total unique particles)
const NUM_PARTICLES: usize = if cfg!(debug_assertions) { 15 } else { 25 };

#[derive(Clone)]
struct Particle {
    /// Angle within segment (0 to segment_angle)
    local_angle: f32,
    /// Distance from center (0 to 1, normalized)
    radius: f32,
    /// Velocity outward
    velocity: f32,
    /// Color hue (0-360)
    hue: f32,
    /// Size multiplier
    size: f32,
}

pub struct Kaleidoscope {
    /// All particles (will be mirrored across segments)
    particles: Vec<Particle>,
    /// Current rotation offset
    rotation: f32,
    /// Rotation velocity
    rotation_velocity: f32,
    /// Zoom pulsing factor
    zoom: f32,
    /// Color hue offset that cycles over time
    hue_offset: f32,
    /// Bass level for effects
    bass: f32,
    /// Treble for sparkle effects
    treble: f32,
    /// Frame counter
    frame_count: u32,
}

impl Default for Kaleidoscope {
    fn default() -> Self {
        let mut rng = rand::rng();
        let segment_angle = std::f32::consts::TAU / NUM_SEGMENTS as f32;

        let particles: Vec<Particle> = (0..NUM_PARTICLES)
            .map(|_| Particle {
                local_angle: rng.random_range(0.0..segment_angle),
                radius: rng.random_range(0.1..1.0),
                velocity: rng.random_range(-0.01..0.02),
                hue: rng.random_range(0.0..360.0),
                size: rng.random_range(0.5..1.5),
            })
            .collect();

        Self {
            particles,
            rotation: 0.0,
            rotation_velocity: 0.0,
            zoom: 1.0,
            hue_offset: 0.0,
            bass: 0.0,
            treble: 0.0,
            frame_count: 0,
        }
    }
}

impl Kaleidoscope {
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

impl Visualization for Kaleidoscope {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Smooth bass and treble tracking
        self.bass = self.bass * 0.7 + analysis.bass * 0.3;
        self.treble = self.treble * 0.8 + analysis.treble * 0.2;

        // Rotation syncs to BPM when available, with bass-based acceleration
        let base_rotation = if analysis.bpm > 0.0 {
            // One rotation per 8 beats at detected BPM
            (analysis.bpm / 60.0) * std::f32::consts::TAU / (8.0 * 60.0)
        } else {
            0.005
        };
        self.rotation_velocity += analysis.bass * 0.02;
        self.rotation_velocity *= 0.95; // Decay
        self.rotation += base_rotation + self.rotation_velocity;

        // Zoom pulses with bass
        let target_zoom = 1.0 + self.bass * 0.3;
        self.zoom = self.zoom * 0.9 + target_zoom * 0.1;

        // Hue cycles based on energy
        self.hue_offset += 0.5 + analysis.energy * 2.0;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }

        // Update particles
        let segment_angle = std::f32::consts::TAU / NUM_SEGMENTS as f32;
        let mut rng = rand::rng();

        for (i, particle) in self.particles.iter_mut().enumerate() {
            // Move outward/inward based on velocity
            particle.radius += particle.velocity * (1.0 + analysis.energy);

            // Respawn if too far or too close
            if particle.radius > 1.0 || particle.radius < 0.05 {
                particle.radius = rng.random_range(0.1..0.3);
                particle.velocity = rng.random_range(0.005..0.02) * (1.0 + self.bass);
                particle.local_angle = rng.random_range(0.0..segment_angle);
                particle.hue = rng.random_range(0.0..360.0);
            }

            // Swirl within segment based on bands
            let band_idx = i % 8;
            particle.local_angle += analysis.bands_normalized[band_idx] * 0.02;
            if particle.local_angle > segment_angle {
                particle.local_angle -= segment_angle;
            }

            // Size pulses with corresponding band
            particle.size = 0.5 + analysis.bands_normalized[band_idx] * 1.5;
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0 * self.zoom;
        let segment_angle = std::f32::consts::TAU / NUM_SEGMENTS as f32;

        // Draw each particle mirrored across all segments
        for particle in &self.particles {
            let hue = (particle.hue + self.hue_offset) % 360.0;
            let saturation = 0.7 + self.treble * 0.3;
            let value = 0.5 + particle.radius * 0.3;
            let alpha = 0.2 + self.bass * 0.25;

            let color = Self::hsv_to_rgba(hue, saturation, value, alpha);
            let particle_size = 10.0 + particle.size * 15.0 * (1.0 + self.bass * 0.3);
            let r = particle.radius * max_radius;

            // Draw in each segment (mirrored)
            for seg in 0..NUM_SEGMENTS {
                let base_angle = seg as f32 * segment_angle + self.rotation;

                // Original position
                let angle1 = base_angle + particle.local_angle;
                let x1 = center.x + r * angle1.cos();
                let y1 = center.y + r * angle1.sin();

                draw.ellipse()
                    .x_y(x1, y1)
                    .w_h(particle_size, particle_size)
                    .color(color);

                // Mirrored position (reflect within segment)
                let angle2 = base_angle + segment_angle - particle.local_angle;
                let x2 = center.x + r * angle2.cos();
                let y2 = center.y + r * angle2.sin();

                draw.ellipse()
                    .x_y(x2, y2)
                    .w_h(particle_size, particle_size)
                    .color(color);
            }
        }

        // Draw connecting lines between adjacent particles on beat
        if self.bass > 0.4 {
            let line_alpha = (self.bass - 0.4) * 0.5;
            for seg in 0..NUM_SEGMENTS {
                let base_angle = seg as f32 * segment_angle + self.rotation;

                for i in 0..self.particles.len().saturating_sub(1) {
                    if i % 3 != 0 {
                        continue;
                    } // Only draw every 3rd for performance

                    let p1 = &self.particles[i];
                    let p2 = &self.particles[(i + 1) % self.particles.len()];

                    let angle1 = base_angle + p1.local_angle;
                    let angle2 = base_angle + p2.local_angle;

                    let r1 = p1.radius * max_radius;
                    let r2 = p2.radius * max_radius;

                    let x1 = center.x + r1 * angle1.cos();
                    let y1 = center.y + r1 * angle1.sin();
                    let x2 = center.x + r2 * angle2.cos();
                    let y2 = center.y + r2 * angle2.sin();

                    let hue = (p1.hue + self.hue_offset) % 360.0;
                    let color = Self::hsv_to_rgba(hue, 0.8, 0.9, line_alpha);

                    draw.line()
                        .start(pt2(x1, y1))
                        .end(pt2(x2, y2))
                        .weight(1.0 + self.bass)
                        .color(color);
                }
            }
        }

        // Center mandala glow
        let glow_radius = max_radius * 0.12 * (1.0 + self.bass * 0.3);
        for i in 0..7 {
            let t = i as f32 / 7.0;
            let r = glow_radius * (1.0 - t);
            let hue = (self.hue_offset + t * 60.0) % 360.0;
            let alpha = t * 0.2 * (0.5 + self.bass * 0.3);
            let color = Self::hsv_to_rgba(hue, 0.6, 0.9, alpha);

            draw.ellipse().xy(center).radius(r).color(color);
        }
    }
}
