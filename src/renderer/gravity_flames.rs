//! Zero-gravity flames visualization.
//!
//! Creates flame-like particles that emanate from the center in directions
//! corresponding to frequency bands, with slowly rotating base angles to
//! prevent directional bias in unbalanced audio.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

/// Number of flame particles
const NUM_PARTICLES: usize = if cfg!(debug_assertions) { 200 } else { 500 };
/// Probability of spawning per frame when energy > 0.3
const SPAWN_RATE: f32 = 0.8;
/// Maximum particle lifetime in frames (~2 seconds at 60fps)
const PARTICLE_MAX_AGE: f32 = 120.0;
/// Base outward velocity
const BASE_VELOCITY: f32 = 2.0;
/// How much energy affects velocity
const VELOCITY_ENERGY_MULTIPLIER: f32 = 8.0;
/// Radians per frame for base angle rotation (~21 sec full rotation)
const ROTATION_SPEED: f32 = 0.01;
/// Velocity decay per frame (zero-G drift)
const DRAG_COEFFICIENT: f32 = 0.98;
/// Smoothing attack for frequency bands
const ATTACK: f32 = 0.7;
/// Smoothing decay for frequency bands
const DECAY: f32 = 0.15;

#[derive(Clone)]
struct FlameParticle {
    /// Current position
    position: Vec2,
    /// Current velocity (direction and speed)
    velocity: Vec2,
    /// Frames since birth
    age: f32,
    /// 0.0-1.0, affects color (hot=1.0, cool=0.0)
    temperature: f32,
    /// Which frequency band (0-7) controls this particle
    band_idx: usize,
    /// Particle radius
    size: f32,
}

pub struct GravityFlames {
    /// Flame particles
    particles: Vec<FlameParticle>,
    /// Current rotation offset for base angles
    base_angle_rotation: f32,
    /// Smoothed frequency band values
    smoothed_bands: [f32; 8],
    /// Smoothed overall energy
    energy: f32,
    /// Frame counter for timing effects
    frame_count: u32,
}

impl GravityFlames {
    /// Create a new GravityFlames visualization
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(NUM_PARTICLES),
            base_angle_rotation: 0.0,
            smoothed_bands: [0.0; 8],
            energy: 0.0,
            frame_count: 0,
        }
    }

    /// Map temperature (0-1) to flame color gradient
    fn temperature_to_color(&self, temp: f32) -> Srgba<u8> {
        let temp = temp.clamp(0.0, 1.0);

        let (r, g, b) = if temp > 0.9 {
            // White hot (0.9-1.0)
            let t = (temp - 0.9) / 0.1;
            (1.0, 1.0, 0.9 + t * 0.1)
        } else if temp > 0.7 {
            // Yellow (0.7-0.9)
            let t = (temp - 0.7) / 0.2;
            (1.0, 1.0, 0.7 + t * 0.2)
        } else if temp > 0.4 {
            // Orange (0.4-0.7)
            let t = (temp - 0.4) / 0.3;
            (1.0, 0.5 + t * 0.5, 0.1 + t * 0.6)
        } else if temp > 0.15 {
            // Red (0.15-0.4)
            let t = (temp - 0.15) / 0.25;
            (0.8 + t * 0.2, 0.1 + t * 0.4, 0.05 + t * 0.05)
        } else {
            // Dark red (0.0-0.15)
            let t = temp / 0.15;
            (0.4 + t * 0.4, t * 0.1, 0.0)
        };

        srgba((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255)
    }

    /// Spawn a new particle
    fn spawn_particle(&mut self, band_idx: usize) {
        let mut rng = rand::rng();

        // Calculate base angle for this frequency band
        let angle = (band_idx as f32 / 8.0) * TAU + self.base_angle_rotation;

        // Add random jitter to angle (±15 degrees)
        let jitter = rng.random_range(-PI / 12.0..PI / 12.0);
        let final_angle = angle + jitter;

        // Calculate velocity magnitude
        let speed = BASE_VELOCITY + self.energy * VELOCITY_ENERGY_MULTIPLIER;

        // Create particle at center with outward velocity
        let particle = FlameParticle {
            position: Vec2::ZERO,
            velocity: Vec2::new(final_angle.cos() * speed, final_angle.sin() * speed),
            age: 0.0,
            temperature: 1.0,
            band_idx,
            size: 3.0 + rng.random_range(0.0..5.0),
        };

        self.particles.push(particle);
    }
}

impl Visualization for GravityFlames {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count += 1;

        // Smooth frequency bands and energy
        for i in 0..8 {
            if analysis.bands[i] > self.smoothed_bands[i] {
                self.smoothed_bands[i] =
                    self.smoothed_bands[i] * (1.0 - ATTACK) + analysis.bands[i] * ATTACK;
            } else {
                self.smoothed_bands[i] =
                    self.smoothed_bands[i] * (1.0 - DECAY) + analysis.bands[i] * DECAY;
            }
        }

        // Smooth energy
        if analysis.energy > self.energy {
            self.energy = self.energy * (1.0 - ATTACK) + analysis.energy * ATTACK;
        } else {
            self.energy = self.energy * (1.0 - DECAY) + analysis.energy * DECAY;
        }

        // Rotate base angles
        self.base_angle_rotation += ROTATION_SPEED;
        if self.base_angle_rotation >= TAU {
            self.base_angle_rotation -= TAU;
        }

        // Update existing particles
        for particle in &mut self.particles {
            particle.age += 1.0;

            // Apply velocity to position
            particle.position += particle.velocity;

            // Apply drag (zero-gravity drift)
            particle.velocity *= DRAG_COEFFICIENT;

            // Update temperature based on age (cools as it ages)
            let age_factor = particle.age / PARTICLE_MAX_AGE;
            particle.temperature = (1.0 - age_factor).max(0.0);

            // Size shrinks as it ages
            particle.size = 3.0 + (1.0 - age_factor) * 5.0;
        }

        // Remove dead particles
        self.particles.retain(|p| p.age < PARTICLE_MAX_AGE);

        // Spawn new particles based on energy
        if self.energy > 0.3 {
            let mut rng = rand::rng();
            let mut spawn_chance = self.energy * SPAWN_RATE;

            while rng.random::<f32>() < spawn_chance && self.particles.len() < NUM_PARTICLES {
                // Pick random frequency band
                let band_idx = rng.random_range(0..8);
                self.spawn_particle(band_idx);

                // Reduce spawn chance to prevent too many spawns per frame
                spawn_chance *= 0.7;
            }
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();

        // Draw each particle
        for particle in &self.particles {
            // Calculate screen position
            let x = center.x + particle.position.x;
            let y = center.y + particle.position.y;

            // Skip if outside bounds (optimization)
            if x < bounds.left() || x > bounds.right() || y < bounds.bottom() || y > bounds.top() {
                continue;
            }

            // Get color based on temperature
            let color = self.temperature_to_color(particle.temperature);

            // Calculate alpha based on age
            let age_factor = particle.age / PARTICLE_MAX_AGE;
            let alpha = ((1.0 - age_factor).max(0.0) * 0.5 * 255.0) as u8;

            // Draw glow (2 rings for bloom effect)
            for i in 0..2 {
                let t = i as f32 / 2.0;
                let glow_size = particle.size * (1.5 + t * 1.5);
                let glow_alpha = ((alpha as f32) * (1.0 - t) * 0.25) as u8;
                let glow_color = srgba(color.red, color.green, color.blue, glow_alpha);

                draw.ellipse().x_y(x, y).radius(glow_size).color(glow_color);
            }

            // Draw core particle
            let core_color = srgba(color.red, color.green, color.blue, alpha);
            draw.ellipse()
                .x_y(x, y)
                .radius(particle.size)
                .color(core_color);
        }

        // Optional: Draw center point indicator (subtle pulsing)
        let pulse = ((self.frame_count as f32 * 0.1).sin() * 0.5 + 0.5) * 0.15;
        let center_alpha = (pulse * 255.0) as u8;
        draw.ellipse()
            .x_y(center.x, center.y)
            .radius(2.0 + pulse * 3.0)
            .color(srgba(255, 255, 255, center_alpha));
    }
}
