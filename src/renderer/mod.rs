pub mod feedback;
pub mod solar_beat;
pub mod spectrogram;
pub mod squares;

use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

pub use feedback::FeedbackRenderer;
pub use solar_beat::SolarBeat;
pub use spectrogram::Spectrogram;
pub use squares::Squares;

/// Trait that all visualizations must implement
pub trait Visualization {
    /// Update the visualization state with pre-computed audio analysis
    fn update(&mut self, analysis: &AudioAnalysis);

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

const COOLDOWN_FRAMES: u32 = 45; // ~0.75 seconds at 60fps (more responsive)
const NOTIFICATION_FRAMES: u32 = 180; // ~3 seconds at 60fps

/// Main renderer that manages the visualization pipeline and cycling
pub struct Renderer {
    visualizations: Vec<Box<dyn Visualization>>,
    current_idx: usize,
    cooldown: u32,
    notification_text: Option<String>,
    notification_frames: u32,
}

impl Renderer {
    pub fn new(visualization: Box<dyn Visualization>) -> Self {
        Self {
            visualizations: vec![visualization],
            current_idx: 0,
            cooldown: 0,
            notification_text: None,
            notification_frames: 0,
        }
    }

    pub fn with_spectrogram() -> Self {
        Self::new(Box::new(Spectrogram::new()))
    }

    pub fn with_solar_beat() -> Self {
        Self::new(Box::new(SolarBeat::new()))
    }

    /// Creates a renderer that cycles between visualizations
    /// when audio transitions are detected, starting with a random one
    pub fn with_cycling() -> Self {
        let visualizations: Vec<Box<dyn Visualization>> = vec![
            Box::new(SolarBeat::new()),
            Box::new(Spectrogram::new()),
            Box::new(Squares::new()),
        ];

        let mut rng = rand::rng();
        let current_idx = rng.random_range(0..visualizations.len());

        Self {
            visualizations,
            current_idx,
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

    pub fn update(&mut self, analysis: &AudioAnalysis) {
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
        if self.visualizations.len() > 1 && self.cooldown == 0 && analysis.transition_detected {
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

        // Only update the active visualization (performance optimization)
        self.visualizations[self.current_idx].update(analysis);
    }

    pub fn draw(&self, draw: &Draw, bounds: Rect) {
        // Note: Don't draw background - feedback shader clears to black and preserves trails
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
