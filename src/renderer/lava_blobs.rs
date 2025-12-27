//! Organic blob visualization with metaball effects.
//!
//! Renders fluid, pulsating organic shapes that merge and split in response
//! to bass frequencies.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

/// Number of metaballs in the simulation
const NUM_BLOBS: usize = if cfg!(debug_assertions) { 8 } else { 16 };
/// Resolution of the metaball field (pixels between samples)
const FIELD_RESOLUTION: f32 = if cfg!(debug_assertions) { 20.0 } else { 12.0 };
/// Threshold for metaball surface
const SURFACE_THRESHOLD: f32 = 1.0;

#[derive(Clone)]
struct Blob {
    /// Position
    x: f32,
    y: f32,
    /// Velocity
    vx: f32,
    vy: f32,
    /// Radius (affects field strength)
    radius: f32,
    /// Target radius (for pulsing)
    target_radius: f32,
    /// Color hue
    hue: f32,
}

pub struct LavaBlobs {
    /// All blobs
    blobs: Vec<Blob>,
    /// Global hue offset that cycles
    hue_offset: f32,
    /// Bass level for pulsing
    bass: f32,
    /// Energy for color saturation
    energy: f32,
    /// Frame counter for noise
    frame_count: u32,
    /// Bounds for physics (updated on draw)
    bounds_w: f32,
    bounds_h: f32,
}

impl Default for LavaBlobs {
    fn default() -> Self {
        let mut rng = rand::rng();

        let blobs: Vec<Blob> = (0..NUM_BLOBS)
            .map(|i| {
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                let speed = rng.random_range(0.5..2.0);
                Blob {
                    x: rng.random_range(-100.0..100.0),
                    y: rng.random_range(-100.0..100.0),
                    vx: angle.cos() * speed,
                    vy: angle.sin() * speed,
                    radius: rng.random_range(30.0..60.0),
                    target_radius: rng.random_range(30.0..60.0),
                    hue: (i as f32 / NUM_BLOBS as f32) * 120.0, // Spread across warm colors
                }
            })
            .collect();

        Self {
            blobs,
            hue_offset: 0.0,
            bass: 0.0,
            energy: 0.0,
            frame_count: 0,
            bounds_w: 400.0,
            bounds_h: 300.0,
        }
    }
}

impl LavaBlobs {
    /// Calculate metaball field value at a point
    fn field_value(&self, x: f32, y: f32) -> f32 {
        let mut sum = 0.0;
        for blob in &self.blobs {
            let dx = x - blob.x;
            let dy = y - blob.y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > 0.01 {
                // Metaball field: r^2 / d^2
                sum += (blob.radius * blob.radius) / dist_sq;
            } else {
                sum += 100.0; // Very close to center
            }
        }
        sum
    }

    /// Get dominant color at a point based on nearby blobs
    fn field_color(&self, x: f32, y: f32) -> (f32, f32, f32) {
        let mut total_weight = 0.0;
        let mut weighted_hue = 0.0;

        for blob in &self.blobs {
            let dx = x - blob.x;
            let dy = y - blob.y;
            let dist_sq = dx * dx + dy * dy;
            let weight = (blob.radius * blob.radius) / (dist_sq + 1.0);
            weighted_hue += (blob.hue + self.hue_offset) * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            let hue = (weighted_hue / total_weight) % 360.0;
            let saturation = 0.7 + self.energy * 0.3;
            let value = 0.8 + self.bass * 0.2;
            Self::hsv_to_rgb(hue, saturation, value)
        } else {
            (0.0, 0.0, 0.0)
        }
    }

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

    /// Simple 2D noise function
    fn noise(x: f32, y: f32, t: f32) -> f32 {
        ((x * 0.1 + t).sin() * (y * 0.1 + t * 0.7).cos() + 1.0) * 0.5
    }
}

impl Visualization for LavaBlobs {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Smooth tracking
        self.bass = self.bass * 0.7 + analysis.bass * 0.3;
        self.energy = self.energy * 0.9 + analysis.energy * 0.1;

        // Hue cycles with mids
        self.hue_offset += 0.3 + analysis.mids * 1.5;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }

        let half_w = self.bounds_w / 2.0;
        let half_h = self.bounds_h / 2.0;
        let t = self.frame_count as f32 * 0.01;

        // Update blob physics
        for (i, blob) in self.blobs.iter_mut().enumerate() {
            // Apply noise-based acceleration
            let noise_x = Self::noise(blob.x * 0.01, blob.y * 0.01, t) - 0.5;
            let noise_y = Self::noise(blob.y * 0.01, blob.x * 0.01 + 100.0, t) - 0.5;

            blob.vx += noise_x * 0.3 + analysis.bands_normalized[i % 8] * noise_x * 2.0;
            blob.vy += noise_y * 0.3 + analysis.bands_normalized[i % 8] * noise_y * 2.0;

            // Damping
            blob.vx *= 0.98;
            blob.vy *= 0.98;

            // Speed limit
            let speed = (blob.vx * blob.vx + blob.vy * blob.vy).sqrt();
            let max_speed = 4.0 + analysis.energy * 3.0;
            if speed > max_speed {
                blob.vx = blob.vx / speed * max_speed;
                blob.vy = blob.vy / speed * max_speed;
            }

            // Move
            blob.x += blob.vx;
            blob.y += blob.vy;

            // Bounce off walls with some randomness
            if blob.x < -half_w + blob.radius {
                blob.x = -half_w + blob.radius;
                blob.vx = blob.vx.abs() * 0.8;
            } else if blob.x > half_w - blob.radius {
                blob.x = half_w - blob.radius;
                blob.vx = -blob.vx.abs() * 0.8;
            }

            if blob.y < -half_h + blob.radius {
                blob.y = -half_h + blob.radius;
                blob.vy = blob.vy.abs() * 0.8;
            } else if blob.y > half_h - blob.radius {
                blob.y = half_h - blob.radius;
                blob.vy = -blob.vy.abs() * 0.8;
            }

            // Radius pulses with corresponding frequency band
            let band_value = analysis.bands_normalized[i % 8];
            blob.target_radius = 35.0 + band_value * 50.0;
            blob.radius = blob.radius * 0.9 + blob.target_radius * 0.1;
        }

        // Blobs attract/repel based on bass
        let attraction_strength = if analysis.bass > 0.5 { -0.5 } else { 0.2 };
        for i in 0..self.blobs.len() {
            for j in (i + 1)..self.blobs.len() {
                let dx = self.blobs[j].x - self.blobs[i].x;
                let dy = self.blobs[j].y - self.blobs[i].y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                if dist < 150.0 {
                    let force = attraction_strength / dist;
                    let fx = dx / dist * force;
                    let fy = dy / dist * force;

                    self.blobs[i].vx += fx;
                    self.blobs[i].vy += fy;
                    self.blobs[j].vx -= fx;
                    self.blobs[j].vy -= fy;
                }
            }
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        // Update bounds for physics (will be used next frame)
        let bounds_w = bounds.w();
        let bounds_h = bounds.h();

        let left = bounds.left();
        let bottom = bounds.bottom();

        // Sample the field and draw filled regions
        let cols = (bounds_w / FIELD_RESOLUTION) as usize + 1;
        let rows = (bounds_h / FIELD_RESOLUTION) as usize + 1;

        for row in 0..rows {
            for col in 0..cols {
                let x = left + col as f32 * FIELD_RESOLUTION + FIELD_RESOLUTION / 2.0;
                let y = bottom + row as f32 * FIELD_RESOLUTION + FIELD_RESOLUTION / 2.0;

                let field = self.field_value(x, y);

                if field > SURFACE_THRESHOLD {
                    // Inside a blob - draw a filled square
                    let intensity = ((field - SURFACE_THRESHOLD) / 2.0).min(1.0);
                    let (r, g, b) = self.field_color(x, y);

                    // Alpha based on field intensity and overall energy
                    let alpha = 0.3 + intensity * 0.5 + self.bass * 0.2;

                    let color = srgba(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        (alpha.min(1.0) * 255.0) as u8,
                    );

                    draw.rect()
                        .x_y(x, y)
                        .w_h(FIELD_RESOLUTION + 1.0, FIELD_RESOLUTION + 1.0)
                        .color(color);
                } else if field > SURFACE_THRESHOLD * 0.5 {
                    // Near surface - draw glow
                    let t = (field - SURFACE_THRESHOLD * 0.5) / (SURFACE_THRESHOLD * 0.5);
                    let (r, g, b) = self.field_color(x, y);
                    let alpha = t * 0.2;

                    let color = srgba(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        (alpha * 255.0) as u8,
                    );

                    draw.rect()
                        .x_y(x, y)
                        .w_h(FIELD_RESOLUTION + 1.0, FIELD_RESOLUTION + 1.0)
                        .color(color);
                }
            }
        }

        // Draw bright cores at blob centers
        for blob in &self.blobs {
            let hue = (blob.hue + self.hue_offset) % 360.0;
            let (r, g, b) = Self::hsv_to_rgb(hue, 0.3, 1.0);

            // Glow rings
            for i in 0..5 {
                let t = i as f32 / 5.0;
                let radius = blob.radius * 0.3 * (1.0 - t);
                let alpha = t * 0.4;

                draw.ellipse()
                    .x_y(blob.x, blob.y)
                    .radius(radius)
                    .color(srgba(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        (alpha * 255.0) as u8,
                    ));
            }
        }
    }
}

// Store bounds for physics on next frame
impl LavaBlobs {
    #[allow(dead_code)]
    fn update_bounds(&mut self, w: f32, h: f32) {
        self.bounds_w = w;
        self.bounds_h = h;
    }
}
