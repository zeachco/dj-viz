use super::Visualization;
use nannou::prelude::*;
use num_complex::Complex;
use rand::Rng;
use rustfft::FftPlanner;

use crate::audio::BUFFER_SIZE;

const FFT_SIZE: usize = BUFFER_SIZE;
const MAX_SQUARES: usize = 60;
const PULSE_DURATION_SECS: f32 = 10.0;
const PULSE_DURATION_FRAMES: f32 = PULSE_DURATION_SECS * 60.0; // assuming 60fps
const HIGH_FREQ_THRESHOLD: f32 = 0.5;

/// A single square with position, size, and frequency-based properties
struct Square {
    x: f32,
    y: f32,
    size: f32,
    freq_bin: usize,
    lifetime: f32,
}

/// Squares visualization with hue-cycling and pulsing borders
pub struct Squares {
    squares: Vec<Square>,
    fft_planner: FftPlanner<f32>,
    fft_window: Vec<f32>,
    magnitudes: Vec<f32>,
    smoothed_magnitudes: Vec<f32>,

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
        let fft_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()))
            .collect();

        Self {
            squares: Vec::with_capacity(MAX_SQUARES),
            fft_planner: FftPlanner::new(),
            fft_window,
            magnitudes: vec![0.0; FFT_SIZE / 2],
            smoothed_magnitudes: vec![0.0; FFT_SIZE / 2],
            hue_offset: 0.0,
            frame_count: 0,
            translation_x: 0.0,
            translation_y: 0.0,
            bounds_w: 800.0,
            bounds_h: 600.0,
            peak_detected: false,
        }
    }

    /// Convert frequency bin to hue angle (0-1)
    fn freq_to_hue(&self, freq_bin: usize) -> f32 {
        let num_bins = self.magnitudes.len();
        let normalized = freq_bin as f32 / num_bins as f32;
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

    /// Detect if high frequencies have peaked
    fn detect_high_peak(&self) -> bool {
        let num_bins = self.magnitudes.len();
        let high_start = num_bins * 3 / 4;

        let high_energy: f32 = self.smoothed_magnitudes[high_start..]
            .iter()
            .sum::<f32>() / (num_bins - high_start) as f32;

        high_energy > HIGH_FREQ_THRESHOLD
    }

    /// Spawn a new square at a random position
    fn spawn_square(&mut self) {
        let mut rng = rand::rng();

        // Random frequency bin for color
        let freq_bin = rng.random_range(0..self.magnitudes.len());

        // Random position within bounds
        let x = rng.random_range(-self.bounds_w / 2.0..self.bounds_w / 2.0);
        let y = rng.random_range(-self.bounds_h / 2.0..self.bounds_h / 2.0);

        // Random size based on frequency (lower = larger)
        let size_factor = 1.0 - (freq_bin as f32 / self.magnitudes.len() as f32);
        let size = 20.0 + size_factor * 80.0 + rng.random_range(0.0..40.0);

        self.squares.push(Square {
            x,
            y,
            size,
            freq_bin,
            lifetime: 0.0,
        });
    }
}

impl Visualization for Squares {
    fn update(&mut self, samples: &[f32]) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Apply window function and perform FFT
        let windowed: Vec<f32> = samples
            .iter()
            .zip(self.fft_window.iter())
            .map(|(s, w)| s * w)
            .collect();

        let fft = self.fft_planner.plan_fft_forward(FFT_SIZE);
        let mut buffer: Vec<Complex<f32>> = windowed
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .collect();

        fft.process(&mut buffer);

        // Calculate magnitude spectrum
        self.magnitudes = buffer[..FFT_SIZE / 2]
            .iter()
            .map(|c| {
                let mag = c.norm() / FFT_SIZE as f32;
                let db = 20.0 * (mag + 1e-10).log10();
                ((db + 60.0) / 60.0).clamp(0.0, 1.0)
            })
            .collect();

        // Smooth magnitudes
        for i in 0..self.smoothed_magnitudes.len() {
            let smoothing = 0.3;
            self.smoothed_magnitudes[i] =
                self.smoothed_magnitudes[i] * smoothing + self.magnitudes[i] * (1.0 - smoothing);
        }

        // Detect high peak and cycle hue
        let peak_now = self.detect_high_peak();
        if peak_now && !self.peak_detected {
            // Shift hue by a golden ratio fraction for pleasing color jumps
            self.hue_offset = (self.hue_offset + 0.618033988749895) % 1.0;
        }
        self.peak_detected = peak_now;

        // Calculate overall energy for translation movement
        let total_energy: f32 = self.smoothed_magnitudes.iter().sum::<f32>()
            / self.smoothed_magnitudes.len() as f32;

        // Move translation based on energy (psychedelic drift)
        let mut rng = rand::rng();
        self.translation_x += (rng.random_range(-1.0..1.0) * total_energy * 2.0) as f32;
        self.translation_y += (rng.random_range(-1.0..1.0) * total_energy * 2.0) as f32;

        // Dampen translation to prevent flying off
        self.translation_x *= 0.95;
        self.translation_y *= 0.95;

        // Update square lifetimes and remove old ones
        for square in &mut self.squares {
            square.lifetime += 1.0;
        }
        self.squares.retain(|s| s.lifetime < 180.0); // ~3 seconds

        // Spawn new squares based on energy (lower threshold, higher chance)
        let spawn_chance = 0.3 + total_energy * 0.7;
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

            // Get magnitude for this square's frequency bin
            let magnitude = self.smoothed_magnitudes
                .get(square.freq_bin)
                .copied()
                .unwrap_or(0.5);

            // Calculate fill color from frequency
            let hue = self.freq_to_hue(square.freq_bin);
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
                .color(hsla(border_hue, saturation, lightness, alpha));

            // Draw fill
            draw.rect()
                .x_y(x, y)
                .w_h(size, size)
                .color(hsla(hue, saturation, lightness, alpha));
        }
    }
}
