//! Gravitational lens black hole visualization.
//!
//! Creates a black hole effect with spacetime distortion and light bending
//! that responds to bass frequencies.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

/// Number of particles in the accretion disk
const NUM_PARTICLES: usize = if cfg!(debug_assertions) { 200 } else { 500 };
/// Number of rings in the accretion disk
const NUM_RINGS: usize = if cfg!(debug_assertions) { 30 } else { 60 };
/// Number of stars in background
const NUM_STARS: usize = if cfg!(debug_assertions) { 50 } else { 150 };

#[derive(Clone)]
struct Particle {
    /// Angle around the black hole
    angle: f32,
    /// Distance from center (normalized, 0-1 where 0 is event horizon)
    radius: f32,
    /// Angular velocity
    angular_velocity: f32,
    /// Radial velocity (inward)
    radial_velocity: f32,
    /// Color temperature (hot inner, cooler outer)
    temperature: f32,
    /// Size
    size: f32,
}

#[derive(Clone)]
struct Star {
    x: f32,
    y: f32,
    brightness: f32,
    twinkle_phase: f32,
}

pub struct BlackHole {
    /// Accretion disk particles
    particles: Vec<Particle>,
    /// Background stars
    stars: Vec<Star>,
    /// Event horizon radius (pulsing with bass)
    event_horizon: f32,
    /// Gravitational lensing strength
    lensing: f32,
    /// Disk rotation speed
    rotation_speed: f32,
    /// Overall disk brightness
    disk_brightness: f32,
    /// Jet intensity (on bass hits)
    jet_intensity: f32,
    /// Bass level
    bass: f32,
    /// Treble level
    treble: f32,
    /// Energy level
    energy: f32,
    /// Frame counter
    frame_count: u32,
    /// Hue shift for disk colors
    hue_shift: f32,
}

impl Default for BlackHole {
    fn default() -> Self {
        let mut rng = rand::rng();

        // Initialize particles in disk
        let particles: Vec<Particle> = (0..NUM_PARTICLES)
            .map(|_| {
                let radius = rng.random_range(0.2..1.0);
                Particle {
                    angle: rng.random_range(0.0..std::f32::consts::TAU),
                    radius,
                    angular_velocity: 0.02 / (radius * radius), // Kepler-ish
                    radial_velocity: rng.random_range(-0.001..0.0),
                    temperature: 1.0 - radius, // Hotter near center
                    size: rng.random_range(1.0..3.0),
                }
            })
            .collect();

        // Initialize stars
        let stars: Vec<Star> = (0..NUM_STARS)
            .map(|_| Star {
                x: rng.random_range(-1.0..1.0),
                y: rng.random_range(-1.0..1.0),
                brightness: rng.random_range(0.2..1.0),
                twinkle_phase: rng.random_range(0.0..std::f32::consts::TAU),
            })
            .collect();

        Self {
            particles,
            stars,
            event_horizon: 0.15,
            lensing: 1.0,
            rotation_speed: 1.0,
            disk_brightness: 0.7,
            jet_intensity: 0.0,
            bass: 0.0,
            treble: 0.0,
            energy: 0.0,
            frame_count: 0,
            hue_shift: 0.0,
        }
    }
}

impl BlackHole {
    /// Convert temperature (0-1) to blackbody-ish color
    fn temperature_to_color(&self, temp: f32, brightness: f32) -> Srgba<u8> {
        // Hot (1.0) = white/blue, Cool (0.0) = red/orange
        let temp = temp.clamp(0.0, 1.0);

        let (r, g, b) = if temp > 0.8 {
            // White-blue (very hot)
            let t = (temp - 0.8) / 0.2;
            (0.9 - t * 0.2, 0.9 - t * 0.1, 1.0)
        } else if temp > 0.6 {
            // Yellow-white (hot)
            let t = (temp - 0.6) / 0.2;
            (1.0, 0.8 + t * 0.1, 0.5 + t * 0.4)
        } else if temp > 0.4 {
            // Orange (warm)
            let t = (temp - 0.4) / 0.2;
            (1.0, 0.5 + t * 0.3, 0.1 + t * 0.4)
        } else if temp > 0.2 {
            // Red-orange (cool)
            let t = (temp - 0.2) / 0.2;
            (0.8 + t * 0.2, 0.2 + t * 0.3, 0.05 + t * 0.05)
        } else {
            // Deep red (coldest visible)
            let t = temp / 0.2;
            (0.4 + t * 0.4, t * 0.2, 0.0)
        };

        let alpha = brightness * 0.8;

        srgba(
            (r * brightness * 255.0) as u8,
            (g * brightness * 255.0) as u8,
            (b * brightness * 255.0) as u8,
            (alpha * 255.0) as u8,
        )
    }

    /// Apply gravitational lensing distortion to a point
    fn lens_distort(&self, x: f32, y: f32, center: Vec2, strength: f32) -> (f32, f32) {
        let dx = x - center.x;
        let dy = y - center.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.01);

        // Einstein ring-like distortion
        let distortion = strength * 20.0 / (dist + 10.0);

        let angle = dy.atan2(dx);
        let new_dist = dist + distortion;

        (
            center.x + new_dist * angle.cos(),
            center.y + new_dist * angle.sin(),
        )
    }
}

impl Visualization for BlackHole {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);
        let mut rng = rand::rng();

        // Smooth audio tracking
        self.bass = self.bass * 0.7 + analysis.bass * 0.3;
        self.treble = self.treble * 0.8 + analysis.treble * 0.2;
        self.energy = self.energy * 0.9 + analysis.energy * 0.1;

        // Event horizon pulses with bass
        self.event_horizon = 0.12 + self.bass * 0.08;

        // Lensing strength tied to energy
        self.lensing = 0.5 + self.energy * 1.5;

        // Rotation speed increases with mids
        self.rotation_speed = 0.5 + analysis.mids * 1.5;

        // Disk brightness follows energy
        self.disk_brightness = 0.5 + self.energy * 0.5;

        // Jet intensity on bass hits, maximum on punch detection
        if analysis.punch_detected {
            self.jet_intensity = 1.0; // Full jets on punch (calm-to-spike)
        } else if analysis.bass > 0.6 {
            self.jet_intensity = analysis.bass;
        } else {
            self.jet_intensity *= 0.92;
        }

        // Hue shift cycles slowly
        self.hue_shift += 0.1 + self.energy * 0.3;

        // Update particles
        for particle in &mut self.particles {
            // Orbital motion (faster near center)
            particle.angle += particle.angular_velocity * self.rotation_speed;
            if particle.angle > std::f32::consts::TAU {
                particle.angle -= std::f32::consts::TAU;
            }

            // Spiral inward slowly (accretion)
            particle.radius += particle.radial_velocity * (1.0 + self.bass * 0.5);

            // Respawn if fallen into event horizon or drifted too far
            if particle.radius < self.event_horizon || particle.radius > 1.2 {
                particle.radius = rng.random_range(0.6..1.0);
                particle.angle = rng.random_range(0.0..std::f32::consts::TAU);
                particle.radial_velocity = rng.random_range(-0.002..0.0);
                particle.angular_velocity = 0.02 / (particle.radius * particle.radius);
            }

            // Temperature based on radius (hotter near center)
            particle.temperature = (1.0 - particle.radius).clamp(0.0, 1.0);

            // Size variation with audio
            let band_idx = ((particle.angle / std::f32::consts::TAU) * 8.0) as usize % 8;
            particle.size = 1.0 + analysis.bands_normalized[band_idx] * 3.0;
        }

        // Update star twinkle
        for star in &mut self.stars {
            star.twinkle_phase += 0.05 + self.treble * 0.1;
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0;

        // Draw background stars (with lensing near center)
        for star in &self.stars {
            let base_x = center.x + star.x * max_radius * 0.9;
            let base_y = center.y + star.y * max_radius * 0.9;

            // Apply lensing
            let (x, y) = self.lens_distort(base_x, base_y, center, self.lensing * 0.5);

            // Skip if lensed into event horizon area
            let dist_from_center = ((x - center.x).powi(2) + (y - center.y).powi(2)).sqrt();
            if dist_from_center < max_radius * self.event_horizon * 1.5 {
                continue;
            }

            let twinkle = (star.twinkle_phase.sin() + 1.0) / 2.0;
            let brightness = star.brightness * (0.5 + twinkle * 0.5);

            let size = 1.0 + brightness * 2.0;

            draw.ellipse()
                .x_y(x, y)
                .w_h(size, size)
                .color(srgba(
                    (brightness * 255.0) as u8,
                    (brightness * 240.0) as u8,
                    (brightness * 255.0) as u8,
                    (brightness * 200.0) as u8,
                ));
        }

        // Draw accretion disk rings (for smooth appearance)
        for i in 0..NUM_RINGS {
            let t = i as f32 / NUM_RINGS as f32;
            let radius = max_radius * (self.event_horizon + t * (1.0 - self.event_horizon));
            let temp = 1.0 - t; // Hotter inner rings

            // Disk is viewed at an angle, so it's elliptical
            let ring_height = radius * 0.3; // Viewing angle

            let brightness = self.disk_brightness * (0.3 + temp * 0.7) * 0.3;
            let color = self.temperature_to_color(temp, brightness);

            // Draw ring as rotated ellipse
            let tilt = 0.2 + self.bass * 0.1; // Slight tilt variation

            draw.ellipse()
                .xy(center)
                .w_h(radius * 2.0, ring_height * 2.0)
                .rotate(tilt)
                .no_fill()
                .stroke_weight(2.0 + temp * 2.0)
                .stroke(color);
        }

        // Draw individual particles for texture
        for particle in &self.particles {
            let r = particle.radius * max_radius;
            let x = center.x + r * particle.angle.cos();
            let y_flat = r * particle.angle.sin();
            let y = center.y + y_flat * 0.3; // Flatten for disk perspective

            // Brightness based on position (front of disk brighter)
            let front_factor = (particle.angle.sin() + 1.0) / 2.0;
            let brightness =
                self.disk_brightness * (0.3 + front_factor * 0.7) * (0.5 + particle.temperature);

            let color = self.temperature_to_color(particle.temperature, brightness);

            draw.ellipse()
                .x_y(x, y)
                .w_h(particle.size, particle.size * 0.5)
                .color(color);
        }

        // Draw event horizon (pure black center)
        let eh_radius = max_radius * self.event_horizon;
        draw.ellipse()
            .xy(center)
            .radius(eh_radius)
            .color(BLACK);

        // Photon sphere glow just outside event horizon
        for i in 0..10 {
            let t = i as f32 / 10.0;
            let radius = eh_radius * (1.0 + t * 0.3);
            let alpha = (1.0 - t) * 0.3 * self.disk_brightness;

            draw.ellipse()
                .xy(center)
                .radius(radius)
                .no_fill()
                .stroke_weight(2.0)
                .stroke(srgba(255, 200, 100, (alpha * 255.0) as u8));
        }

        // Relativistic jets on bass hits
        if self.jet_intensity > 0.1 {
            let jet_length = max_radius * 0.8 * self.jet_intensity;
            let jet_width_base = eh_radius * 0.3;

            for direction in [1.0, -1.0] {
                let _jet_end_y = center.y + direction * jet_length;

                // Jet beam
                for i in 0..20 {
                    let t = i as f32 / 20.0;
                    let y = center.y + direction * t * jet_length;
                    let width = jet_width_base * (1.0 - t * 0.5);
                    let brightness = self.jet_intensity * (1.0 - t * 0.7);

                    draw.ellipse()
                        .x_y(center.x, y)
                        .w_h(width, width * 0.5)
                        .color(srgba(
                            (brightness * 200.0) as u8,
                            (brightness * 220.0) as u8,
                            255,
                            (brightness * 150.0) as u8,
                        ));
                }
            }
        }

        // Gravitational lensing ring (Einstein ring effect)
        let einstein_radius = eh_radius * 2.5;
        for i in 0..5 {
            let t = i as f32 / 5.0;
            let radius = einstein_radius * (1.0 + t * 0.1);
            let alpha = (1.0 - t) * 0.15 * self.lensing;

            draw.ellipse()
                .xy(center)
                .radius(radius)
                .no_fill()
                .stroke_weight(1.0 + self.energy)
                .stroke(srgba(200, 180, 255, (alpha * 255.0) as u8));
        }
    }
}
