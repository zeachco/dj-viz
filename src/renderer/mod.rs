pub mod black_hole;
pub mod crt_phosphor;
pub mod feedback;
pub mod kaleidoscope;
pub mod lava_blobs;
pub mod solar_beat;
pub mod spectrogram;
pub mod squares;
pub mod tesla_coil;
pub mod vhs_distortion;

use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

pub use black_hole::BlackHole;
pub use crt_phosphor::CrtPhosphor;
pub use feedback::FeedbackRenderer;
pub use kaleidoscope::Kaleidoscope;
pub use lava_blobs::LavaBlobs;
pub use solar_beat::SolarBeat;
pub use spectrogram::Spectrogram;
pub use squares::Squares;
pub use tesla_coil::TeslaCoil;
pub use vhs_distortion::VhsDistortion;

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
    /// Indices of overlay visualizations to blend with burn effect (0-3)
    overlay_indices: Vec<usize>,
    cooldown: u32,
    notification_text: Option<String>,
    notification_frames: u32,
}

impl Renderer {
    pub fn new(visualization: Box<dyn Visualization>) -> Self {
        Self {
            visualizations: vec![visualization],
            current_idx: 0,
            overlay_indices: Vec::new(),
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
            Box::new(TeslaCoil::new()),
            Box::new(Kaleidoscope::new()),
            Box::new(LavaBlobs::new()),
            Box::new(VhsDistortion::new()),
            Box::new(CrtPhosphor::new()),
            Box::new(BlackHole::new()),
        ];

        let mut rng = rand::rng();
        let current_idx = rng.random_range(0..visualizations.len());
        let overlay_indices = Self::select_overlays_for(current_idx, visualizations.len(), &mut rng);

        Self {
            visualizations,
            current_idx,
            overlay_indices,
            cooldown: 0,
            notification_text: None,
            notification_frames: 0,
        }
    }

    /// Selects 0-3 random overlay visualization indices (excluding the primary)
    fn select_overlays_for(primary_idx: usize, count: usize, rng: &mut impl rand::Rng) -> Vec<usize> {
        if count <= 1 {
            return Vec::new();
        }

        // Randomly choose 0-3 overlays
        let num_overlays = rng.random_range(0..=3.min(count - 1));
        let mut overlays = Vec::with_capacity(num_overlays);

        while overlays.len() < num_overlays {
            let idx = rng.random_range(0..count);
            if idx != primary_idx && !overlays.contains(&idx) {
                overlays.push(idx);
            }
        }

        overlays
    }

    /// Selects new overlay visualizations for the current primary
    fn select_overlays(&mut self) {
        let mut rng = rand::rng();
        self.overlay_indices = Self::select_overlays_for(
            self.current_idx,
            self.visualizations.len(),
            &mut rng,
        );
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
            self.select_overlays();
            self.cooldown = COOLDOWN_FRAMES;
            println!(
                "Switched to visualization {} with {} overlays",
                self.current_idx,
                self.overlay_indices.len()
            );
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
            self.select_overlays();
            self.cooldown = COOLDOWN_FRAMES;
            println!(
                "Switched to visualization {} with {} overlays",
                self.current_idx,
                self.overlay_indices.len()
            );
        }

        // Update the active visualization
        self.visualizations[self.current_idx].update(analysis);

        // Update overlay visualizations
        for &idx in &self.overlay_indices {
            self.visualizations[idx].update(analysis);
        }
    }

    /// Draw the primary visualization
    pub fn draw_primary(&self, draw: &Draw, bounds: Rect) {
        self.visualizations[self.current_idx].draw(draw, bounds);
    }

    /// Draw overlay visualizations (to be blended with burn effect)
    pub fn draw_overlays(&self, draws: &[&Draw], bounds: Rect) {
        for (i, &idx) in self.overlay_indices.iter().enumerate() {
            if i < draws.len() {
                self.visualizations[idx].draw(draws[i], bounds);
            }
        }
    }

    /// Returns the number of active overlays
    pub fn overlay_count(&self) -> usize {
        self.overlay_indices.len()
    }

    /// Legacy draw method for backwards compatibility (draws primary only)
    pub fn draw(&self, draw: &Draw, bounds: Rect) {
        // Note: Don't draw background - feedback shader clears to black and preserves trails
        self.draw_primary(draw, bounds);

        // Draw notification text at middle top
        if let Some(ref text) = self.notification_text {
            let alpha = (self.notification_frames as f32 / NOTIFICATION_FRAMES as f32).min(1.0);
            draw.text(text)
                .x_y(0.0, bounds.top() - 30.0)
                .color(rgba(1.0, 1.0, 1.0, alpha))
                .font_size(24);
        }
    }

    /// Draw notification overlay (should be drawn after all visualizations)
    pub fn draw_notification(&self, draw: &Draw, bounds: Rect) {
        if let Some(ref text) = self.notification_text {
            let alpha = (self.notification_frames as f32 / NOTIFICATION_FRAMES as f32).min(1.0);
            draw.text(text)
                .x_y(0.0, bounds.top() - 30.0)
                .color(rgba(1.0, 1.0, 1.0, alpha))
                .font_size(24);
        }
    }
}
