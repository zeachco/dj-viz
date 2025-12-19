pub mod radial;
pub mod spectrogram;

use nannou::prelude::*;
use num_complex::Complex;
use rand::Rng;
use rustfft::FftPlanner;

use crate::audio::BUFFER_SIZE;

pub use radial::Radial;
pub use spectrogram::Spectrogram;

/// Trait that all visualizations must implement
pub trait Visualization {
    /// Update the visualization state with new audio samples
    fn update(&mut self, samples: &[f32]);

    /// Draw the visualization
    fn draw(&self, draw: &Draw, bounds: Rect);
}

/// Resolution settings for renderers
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

impl Resolution {
    pub fn debug() -> Self {
        Self {
            width: 400,
            height: 300,
            fullscreen: false,
        }
    }

    pub fn release() -> Self {
        Self {
            width: 1280,
            height: 720,
            fullscreen: true,
        }
    }

    pub fn current() -> Self {
        if cfg!(debug_assertions) {
            Self::debug()
        } else {
            Self::release()
        }
    }
}

const FFT_SIZE: usize = BUFFER_SIZE;
const HIGH_FREQ_THRESHOLD: f32 = 0.4;
const COOLDOWN_FRAMES: u32 = 120; // ~2 seconds at 60fps
const NOTIFICATION_FRAMES: u32 = 180; // ~3 seconds at 60fps

/// Main renderer that manages the visualization pipeline and cycling
pub struct Renderer {
    visualizations: Vec<Box<dyn Visualization>>,
    current_idx: usize,
    fft_planner: FftPlanner<f32>,
    fft_window: Vec<f32>,
    cooldown: u32,
    notification_text: Option<String>,
    notification_frames: u32,
}

impl Renderer {
    pub fn new(visualization: Box<dyn Visualization>) -> Self {
        let fft_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()))
            .collect();

        Self {
            visualizations: vec![visualization],
            current_idx: 0,
            fft_planner: FftPlanner::new(),
            fft_window,
            cooldown: 0,
            notification_text: None,
            notification_frames: 0,
        }
    }

    pub fn with_spectrogram() -> Self {
        Self::new(Box::new(Spectrogram::new()))
    }

    pub fn with_radial() -> Self {
        Self::new(Box::new(Radial::new()))
    }

    /// Creates a renderer that cycles between spectrogram and radial
    /// when high frequencies are loud
    pub fn with_cycling() -> Self {
        let fft_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()))
            .collect();

        Self {
            visualizations: vec![
                Box::new(Radial::new()),
                Box::new(Spectrogram::new()),
            ],
            current_idx: 0,
            fft_planner: FftPlanner::new(),
            fft_window,
            cooldown: 0,
            notification_text: None,
            notification_frames: 0,
        }
    }

    /// Shows a notification message for 3 seconds
    pub fn show_notification(&mut self, text: String) {
        self.notification_text = Some(text);
        self.notification_frames = NOTIFICATION_FRAMES;
    }

    /// Manually cycle to the next visualization
    pub fn cycle_next(&mut self) {
        if self.visualizations.len() > 1 {
            self.current_idx = (self.current_idx + 1) % self.visualizations.len();
            self.cooldown = COOLDOWN_FRAMES;
            println!("Switched to visualization {}", self.current_idx);
        }
    }

    /// Detects if high frequencies are playing loudly
    fn detect_high_freq_peak(&mut self, samples: &[f32]) -> bool {
        // Apply window function
        let windowed: Vec<f32> = samples
            .iter()
            .zip(self.fft_window.iter())
            .map(|(s, w)| s * w)
            .collect();

        // Perform FFT
        let fft = self.fft_planner.plan_fft_forward(FFT_SIZE);
        let mut buffer: Vec<Complex<f32>> = windowed
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .collect();

        fft.process(&mut buffer);

        // Check high frequency bins (upper 25% of spectrum)
        let num_bins = FFT_SIZE / 2;
        let high_freq_start = num_bins * 3 / 4;

        let high_freq_energy: f32 = buffer[high_freq_start..num_bins]
            .iter()
            .map(|c| {
                let mag = c.norm() / FFT_SIZE as f32;
                let db = 20.0 * (mag + 1e-10).log10();
                ((db + 60.0) / 60.0).clamp(0.0, 1.0)
            })
            .sum::<f32>()
            / (num_bins - high_freq_start) as f32;

        high_freq_energy > HIGH_FREQ_THRESHOLD
    }

    pub fn update(&mut self, samples: &[f32]) {
        // Update cooldowns
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }
        if self.notification_frames > 0 {
            self.notification_frames -= 1;
            if self.notification_frames == 0 {
                self.notification_text = None;
            }
        }

        // Check for visualization switch if multiple visualizations and cooldown expired
        if self.visualizations.len() > 1 && self.cooldown == 0 {
            if self.detect_high_freq_peak(samples) {
                // Switch to a random different visualization
                let mut rng = rand::rng();
                let new_idx = loop {
                    let idx = rng.random_range(0..self.visualizations.len());
                    if idx != self.current_idx || self.visualizations.len() == 1 {
                        break idx;
                    }
                };
                self.current_idx = new_idx;
                self.cooldown = COOLDOWN_FRAMES;
                println!("Switched to visualization {}", self.current_idx);
            }
        }

        // Update all visualizations to keep them in sync
        for viz in &mut self.visualizations {
            viz.update(samples);
        }
    }

    pub fn draw(&self, draw: &Draw, bounds: Rect) {
        draw.background().color(BLACK);
        self.visualizations[self.current_idx].draw(draw, bounds);

        // Draw notification text at middle top
        if let Some(ref text) = self.notification_text {
            let alpha = (self.notification_frames as f32 / NOTIFICATION_FRAMES as f32).min(1.0);
            draw.text(text)
                .x_y(0.0, bounds.top() - 30.0)
                .color(rgba(1.0, 1.0, 1.0, alpha))
                .font_size(24);
        }
    }
}
