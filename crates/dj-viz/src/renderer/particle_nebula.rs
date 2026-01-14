//! Orbital particle nebula visualization.
//!
//! Thousands of particles in orbital motion, colored by frequency band,
//! swirling with mid energy.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

/// Number of particles
const NUM_PARTICLES: usize = if cfg!(debug_assertions) { 300 } else { 800 };

#[derive(Clone)]
struct Particle {
    /// Orbital radius
    radius: f32,
    /// Orbital angle
    angle: f32,
    /// Orbital speed
    speed: f32,
    /// Frequency band this particle responds to (0-7)
    band_idx: usize,
    /// Size
    size: f32,
    /// Vertical offset (for 3D-ish effect)
    z_offset: f32,
    /// Base hue
    hue: f32,
}

pub struct ParticleNebula {
    /// All particles
    particles: Vec<Particle>,
    /// Global rotation
    rotation: f32,
    /// Expansion factor (pulsing with bass)
    expansion: f32,
    /// Hue offset for color cycling
    hue_offset: f32,
    /// Smoothed bass
    bass: f32,
    /// Smoothed mids
    mids: f32,
    /// Smoothed treble
    treble: f32,
    /// Frame counter
    frame_count: u32,
}

impl Default for ParticleNebula {
    fn default() -> Self {
        let mut rng = rand::rng();

        let particles: Vec<Particle> = (0..NUM_PARTICLES)
            .map(|i| {
                let band_idx = i % 8;
                // Particles in outer bands have larger orbits
                let base_radius = 0.1 + (band_idx as f32 / 8.0) * 0.7;
                let radius = base_radius + rng.random_range(-0.1..0.1);

                Particle {
                    radius,
                    angle: rng.random_range(0.0..std::f32::consts::TAU),
                    speed: 0.01 + rng.random_range(-0.005..0.01),
                    band_idx,
                    size: rng.random_range(1.5..4.0),
                    z_offset: rng.random_range(-0.2..0.2),
                    hue: (band_idx as f32 / 8.0) * 360.0 + rng.random_range(-20.0..20.0),
                }
            })
            .collect();

        Self {
            particles,
            rotation: 0.0,
            expansion: 1.0,
            hue_offset: 0.0,
            bass: 0.0,
            mids: 0.0,
            treble: 0.0,
            frame_count: 0,
        }
    }
}

impl ParticleNebula {
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

impl Visualization for ParticleNebula {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Fast attack, slow decay
        let attack = 0.7;
        let decay = 0.15;

        if analysis.bass > self.bass {
            self.bass = self.bass * (1.0 - attack) + analysis.bass * attack;
        } else {
            self.bass = self.bass * (1.0 - decay) + analysis.bass * decay;
        }

        self.mids = self.mids * 0.85 + analysis.mids * 0.15;
        self.treble = self.treble * 0.8 + analysis.treble * 0.2;

        // Global rotation based on mids
        self.rotation += 0.005 + self.mids * 0.02;

        // Expansion pulses with bass
        let target_expansion = 1.0 + self.bass * 0.3;
        self.expansion = self.expansion * 0.9 + target_expansion * 0.1;

        // Color cycling
        self.hue_offset += 0.3 + analysis.energy * 1.5;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }

        // Update particles
        for particle in &mut self.particles {
            // Orbital motion - speed affected by corresponding band
            let band_energy = analysis.bands_normalized[particle.band_idx];
            particle.angle += particle.speed * (1.0 + band_energy * 2.0);

            if particle.angle > std::f32::consts::TAU {
                particle.angle -= std::f32::consts::TAU;
            }

            // Radius wobbles with band energy
            let base_radius = 0.1 + (particle.band_idx as f32 / 8.0) * 0.7;
            particle.radius = base_radius + band_energy * 0.1;

            // Size pulses with band
            particle.size = 2.0 + band_energy * 4.0;
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0 * self.expansion;

        // Sort particles by z for proper layering (back to front)
        let mut sorted_particles: Vec<_> = self.particles.iter().collect();
        sorted_particles.sort_by(|a, b| {
            a.z_offset.partial_cmp(&b.z_offset).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Draw connection lines between nearby particles (creates nebula web effect)
        if self.mids > 0.3 {
            let line_alpha = (self.mids - 0.3) * 0.3;
            for i in 0..self.particles.len().min(100) {
                let p1 = &self.particles[i];
                let p2 = &self.particles[(i + 1) % self.particles.len()];

                // Only connect particles in adjacent bands
                if (p1.band_idx as i32 - p2.band_idx as i32).abs() <= 1 {
                    let r1 = p1.radius * max_radius;
                    let r2 = p2.radius * max_radius;
                    let angle1 = p1.angle + self.rotation;
                    let angle2 = p2.angle + self.rotation;

                    let x1 = center.x + r1 * angle1.cos();
                    let y1 = center.y + r1 * angle1.sin() * 0.6 + p1.z_offset * max_radius * 0.3;
                    let x2 = center.x + r2 * angle2.cos();
                    let y2 = center.y + r2 * angle2.sin() * 0.6 + p2.z_offset * max_radius * 0.3;

                    let hue = (p1.hue + self.hue_offset) % 360.0;
                    let color = Self::hsv_to_rgba(hue, 0.5, 0.6, line_alpha);

                    draw.line()
                        .start(pt2(x1, y1))
                        .end(pt2(x2, y2))
                        .weight(0.5)
                        .color(color);
                }
            }
        }

        // Draw particles
        for particle in sorted_particles {
            let r = particle.radius * max_radius;
            let angle = particle.angle + self.rotation;

            // Elliptical orbit (flattened for nebula disc effect)
            let x = center.x + r * angle.cos();
            let y = center.y + r * angle.sin() * 0.6 + particle.z_offset * max_radius * 0.3;

            // Depth affects brightness
            let depth_factor = (particle.z_offset + 0.2) / 0.4;
            let _band_energy = 0.5; // Approximate since we don't have analysis here

            let hue = (particle.hue + self.hue_offset) % 360.0;
            let saturation = 0.7 + self.treble * 0.3;
            let value = 0.4 + depth_factor * 0.3 + self.bass * 0.3;
            let alpha = 0.4 + depth_factor * 0.3;

            let color = Self::hsv_to_rgba(hue, saturation, value, alpha);

            // Draw particle with glow
            let glow_size = particle.size * 2.0;
            draw.ellipse()
                .x_y(x, y)
                .w_h(glow_size, glow_size)
                .color(Self::hsv_to_rgba(hue, saturation * 0.5, value * 0.5, alpha * 0.3));

            draw.ellipse()
                .x_y(x, y)
                .w_h(particle.size, particle.size)
                .color(color);
        }

        // Central glow (nebula core)
        let core_radius = max_radius * 0.15 * (1.0 + self.bass * 0.3);
        for i in 0..10 {
            let t = i as f32 / 10.0;
            let r = core_radius * (1.0 - t * 0.8);
            let hue = (self.hue_offset + t * 30.0) % 360.0;
            let alpha = (1.0 - t) * 0.2;
            let color = Self::hsv_to_rgba(hue, 0.4, 0.8, alpha);

            draw.ellipse()
                .xy(center)
                .w_h(r * 2.0, r * 1.2)
                .color(color);
        }
    }
}
