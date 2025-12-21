//! Geometric grid visualization with beat-reactive squares.
//!
//! Displays a grid of rotating, pulsing squares that respond to frequency bands
//! and spawn particle bursts on beat detection.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::{AudioAnalysis, NUM_BANDS};

const MAX_SQUARES: usize = 60;
const PULSE_DURATION_SECS: f32 = 10.0;
const PULSE_DURATION_FRAMES: f32 = PULSE_DURATION_SECS * 60.0; // assuming 60fps

/// A single square with position, size, and frequency-based properties
struct Square {
    x: f32,
    y: f32,
    size: f32,
    band_idx: usize,
    lifetime: f32,
    rotation: f32,
    rotation_dir: f32, // 1.0 or -1.0 for clockwise/counter-clockwise
}

/// Squares visualization with hue-cycling and pulsing borders
pub struct Squares {
    squares: Vec<Square>,
    smoothed_bands: [f32; NUM_BANDS],

    // Hue offset that cycles on high peaks
    hue_offset: f32,

    // Frame counter for border pulse effect
    frame_count: u32,

    // Translation offset for movement
    translation_x: f32,
    translation_y: f32,

    // Bounds for spawning (cached)
    bounds_w: f32,
    bounds_h: f32,

    // Peak detection
    peak_detected: bool,
}

impl Squares {
    pub fn new() -> Self {
        Self {
            squares: Vec::with_capacity(MAX_SQUARES),
            smoothed_bands: [0.0; NUM_BANDS],
            hue_offset: 0.0,
            frame_count: 0,
            translation_x: 0.0,
            translation_y: 0.0,
            bounds_w: 800.0,
            bounds_h: 600.0,
            peak_detected: false,
        }
    }

    /// Convert band index to hue angle (0-1)
    fn band_to_hue(&self, band_idx: usize) -> f32 {
        let normalized = band_idx as f32 / NUM_BANDS as f32;
        // Map to hue: bass = red/orange, mids = green/cyan, highs = blue/purple
        (normalized + self.hue_offset) % 1.0
    }

    /// Get chromatically opposite hue
    fn opposite_hue(hue: f32) -> f32 {
        (hue + 0.5) % 1.0
    }

    /// Calculate border width based on pulse effect (1-10px over 10 seconds)
    fn border_width(&self) -> f32 {
        let pulse_progress = (self.frame_count as f32 % PULSE_DURATION_FRAMES) / PULSE_DURATION_FRAMES;
        // Sine wave for smooth pulse: oscillates between 1 and 10
        let pulse = (pulse_progress * std::f32::consts::TAU).sin();
        1.0 + (pulse + 1.0) * 4.5 // maps -1..1 to 1..10
    }

    /// Spawn a new square at a random position
    fn spawn_square(&mut self) {
        let mut rng = rand::rng();

        // Random band for color
        let band_idx = rng.random_range(0..NUM_BANDS);

        // Random position within bounds
        let x = rng.random_range(-self.bounds_w / 2.0..self.bounds_w / 2.0);
        let y = rng.random_range(-self.bounds_h / 2.0..self.bounds_h / 2.0);

        // Random size based on band (lower = larger), scaled down for fewer bands
        let size_factor = 1.0 - (band_idx as f32 / NUM_BANDS as f32);
        let size = 10.0 + size_factor * 40.0 + rng.random_range(0.0..20.0);

        // Random rotation direction
        let rotation_dir = if rng.random::<bool>() { 1.0 } else { -1.0 };

        self.squares.push(Square {
            x,
            y,
            size,
            band_idx,
            lifetime: 0.0,
            rotation: 0.0,
            rotation_dir,
        });
    }
}

impl Visualization for Squares {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Smooth band values
        for i in 0..NUM_BANDS {
            let smoothing = 0.3;
            self.smoothed_bands[i] =
                self.smoothed_bands[i] * smoothing + analysis.bands_normalized[i] * (1.0 - smoothing);
        }

        // Detect high peak and cycle hue (use treble from analysis)
        let peak_now = analysis.treble > 0.5;
        if peak_now && !self.peak_detected {
            // Shift hue by a golden ratio fraction for pleasing color jumps
            self.hue_offset = (self.hue_offset + 0.618_034) % 1.0;
        }
        self.peak_detected = peak_now;

        // Move translation based on energy (psychedelic drift)
        let mut rng = rand::rng();
        self.translation_x += rng.random_range(-1.0..1.0) * analysis.energy * 2.0;
        self.translation_y += rng.random_range(-1.0..1.0) * analysis.energy * 2.0;

        // Dampen translation to prevent flying off
        self.translation_x *= 0.95;
        self.translation_y *= 0.95;

        // Update square lifetimes and rotation based on energy
        for square in &mut self.squares {
            square.lifetime += 1.0;
            // Tilt rotation based on energy - subtle wobble following the beat
            let band_energy = self.smoothed_bands
                .get(square.band_idx)
                .copied()
                .unwrap_or(0.0);
            square.rotation += square.rotation_dir * band_energy * 0.15;
            // Dampen rotation back towards zero
            square.rotation *= 0.92;
        }
        self.squares.retain(|s| s.lifetime < 180.0); // ~3 seconds

        // Spawn new squares based on energy (lower threshold, higher chance)
        let spawn_chance = 0.3 + analysis.energy * 0.7;
        while rng.random::<f32>() < spawn_chance && self.squares.len() < MAX_SQUARES {
            self.spawn_square();
            // Reduce chance for subsequent spawns this frame
            if rng.random::<f32>() > 0.5 {
                break;
            }
        }

        // Always ensure some squares exist
        while self.squares.len() < 5 {
            self.spawn_square();
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        // Update cached bounds (can't mutate in draw, so we use current values)
        let bounds_w = bounds.w();
        let bounds_h = bounds.h();

        let border_width = self.border_width();

        // Draw each square
        for square in &self.squares {
            // Scale position to current bounds
            let scale_x = bounds_w / self.bounds_w.max(1.0);
            let scale_y = bounds_h / self.bounds_h.max(1.0);

            let x = square.x * scale_x + self.translation_x;
            let y = square.y * scale_y + self.translation_y;

            // Get magnitude for this square's band
            let magnitude = self.smoothed_bands
                .get(square.band_idx)
                .copied()
                .unwrap_or(0.5);

            // Calculate fill color from band
            let hue = self.band_to_hue(square.band_idx);
            let saturation = 0.8 + magnitude * 0.2;
            let lightness = 0.4 + magnitude * 0.3;

            // Border color is chromatically opposite
            let border_hue = Self::opposite_hue(hue);

            // Size modulated by magnitude
            let size = square.size * (0.5 + magnitude);

            // Alpha fade based on lifetime
            let alpha = 1.0 - (square.lifetime / 180.0).min(1.0);

            // Draw border (larger rectangle behind)
            draw.rect()
                .x_y(x, y)
                .w_h(size + border_width * 2.0, size + border_width * 2.0)
                .rotate(square.rotation)
                .color(hsla(border_hue, saturation, lightness, alpha));

            // Draw fill
            draw.rect()
                .x_y(x, y)
                .w_h(size, size)
                .rotate(square.rotation)
                .color(hsla(hue, saturation, lightness, alpha));
        }
    }
}
