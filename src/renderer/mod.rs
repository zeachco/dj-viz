//! Visualization orchestration and cycling.
//!
//! Manages the visualization pipeline, including automatic cycling between effects
//! on detected musical transitions and overlay blending.

pub mod black_hole;
pub mod crt_phosphor;
pub mod debug;
pub mod feedback;
pub mod gravity_flames;
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
pub use debug::DebugViz;
pub use feedback::FeedbackRenderer;
pub use gravity_flames::GravityFlames;
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
    /// When true, auto-cycling is disabled (user manually selected a visualization)
    locked: bool,
    /// Debug visualization - toggled with 'd' key
    debug_viz: DebugViz,
    debug_viz_visible: bool,
}

impl Renderer {
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
            Box::new(GravityFlames::new()),
        ];

        let mut rng = rand::rng();
        let current_idx = rng.random_range(0..visualizations.len());
        // Start with medium energy (0.5) for initial overlay selection
        let overlay_indices =
            Self::select_overlays_for(current_idx, visualizations.len(), 0.5, &mut rng);

        Self {
            visualizations,
            current_idx,
            overlay_indices,
            cooldown: 0,
            notification_text: None,
            notification_frames: 0,
            locked: false,
            debug_viz: DebugViz::new(),
            debug_viz_visible: false,
        }
    }

    /// Selects 0-3 random overlay visualization indices (excluding the primary)
    /// based on audio energy: low energy → fewer overlays, high energy → more overlays
    fn select_overlays_for(
        primary_idx: usize,
        count: usize,
        energy: f32,
        rng: &mut impl rand::Rng,
    ) -> Vec<usize> {
        if count <= 1 {
            return Vec::new();
        }

        // Map energy (0-1) to overlay count (0-3)
        // Low energy (< 0.3): 0-1 overlays
        // Medium energy (0.3-0.6): 1-2 overlays
        // High energy (> 0.6): 2-3 overlays
        let max_overlays = if energy < 0.3 {
            rng.random_range(0..=1)
        } else if energy < 0.6 {
            rng.random_range(1..=2)
        } else {
            rng.random_range(2..=3)
        };

        let num_overlays = max_overlays.min(count - 1);
        let mut overlays = Vec::with_capacity(num_overlays);

        while overlays.len() < num_overlays {
            let idx = rng.random_range(0..count);
            if idx != primary_idx && !overlays.contains(&idx) {
                overlays.push(idx);
            }
        }

        overlays
    }

    /// Selects new overlay visualizations for the current primary based on audio energy
    fn select_overlays(&mut self, energy: f32) {
        let mut rng = rand::rng();
        self.overlay_indices =
            Self::select_overlays_for(self.current_idx, self.visualizations.len(), energy, &mut rng);
    }

    /// Shows a notification message for 3 seconds
    pub fn show_notification(&mut self, text: String) {
        self.notification_text = Some(text);
        self.notification_frames = NOTIFICATION_FRAMES;
    }

    /// Manually cycle to the next visualization (unlocks auto-cycling)
    pub fn cycle_next(&mut self) {
        if self.visualizations.len() > 1 {
            self.current_idx = (self.current_idx + 1) % self.visualizations.len();
            self.select_overlays(0.5); // Use medium energy for manual cycling
            self.cooldown = COOLDOWN_FRAMES;
            self.locked = false; // Space unlocks and resumes auto-cycling
            println!(
                "Switched to visualization {} with {} overlays",
                self.current_idx,
                self.overlay_indices.len()
            );
        }
    }

    /// Set a specific visualization by index and lock (disable auto-cycling)
    /// Returns the visualization name if successful
    pub fn set_visualization(&mut self, idx: usize) -> Option<&'static str> {
        if idx >= self.visualizations.len() {
            self.locked = true;
            return None;
        }
        self.current_idx = idx;
        self.overlay_indices.clear(); // No overlays when locked to single viz
        self.cooldown = COOLDOWN_FRAMES;
        self.locked = true;

        let name = Self::visualization_name(idx);
        println!("Locked to visualization {}: {}", idx, name);
        Some(name)
    }

    /// Returns the number of available visualizations
    pub fn visualization_count(&self) -> usize {
        self.visualizations.len()
    }

    /// Get visualization name by index
    fn visualization_name(idx: usize) -> &'static str {
        match idx {
            0 => "SolarBeat",
            1 => "Spectrogram",
            2 => "Squares",
            3 => "TeslaCoil",
            4 => "Kaleidoscope",
            5 => "LavaBlobs",
            6 => "VhsDistortion",
            7 => "CrtPhosphor",
            8 => "BlackHole",
            9 => "GravityFlames",
            _ => "Unknown",
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

        // Check for visualization switch if multiple visualizations, cooldown expired, and not locked
        if self.visualizations.len() > 1
            && self.cooldown == 0
            && !self.locked
            && analysis.transition_detected
        {
            // Switch to a random different visualization
            let mut rng = rand::rng();
            let new_idx = loop {
                let idx = rng.random_range(0..self.visualizations.len());
                if idx != self.current_idx || self.visualizations.len() == 1 {
                    break idx;
                }
            };
            self.current_idx = new_idx;

            // Use combined energy metric: overall energy + treble (for hi-hats/cymbals) + bass (for kicks)
            // This makes rapid sounds (techno kicks, hi-hats) increase overlay count
            let combined_energy = (analysis.energy * 0.5 + analysis.treble * 0.3 + analysis.bass * 0.2).min(1.0);
            self.select_overlays(combined_energy);
            self.cooldown = COOLDOWN_FRAMES;
            println!(
                "Switched to visualization {} with {} overlays (energy: {:.2})",
                self.current_idx,
                self.overlay_indices.len(),
                combined_energy
            );
        }

        // Update the active visualization
        self.visualizations[self.current_idx].update(analysis);

        // Update overlay visualizations
        for &idx in &self.overlay_indices {
            self.visualizations[idx].update(analysis);
        }

        // Always update debug viz (even if not visible, so it's ready when toggled)
        self.debug_viz.update(analysis);
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

    /// Toggle debug visualization visibility
    pub fn toggle_debug_viz(&mut self) {
        self.debug_viz_visible = !self.debug_viz_visible;
        let status = if self.debug_viz_visible { "ON" } else { "OFF" };
        println!("Debug visualization: {}", status);
    }

    /// Draw debug visualization (CrtNumbers) if visible
    pub fn draw_debug_viz(&self, draw: &Draw, bounds: Rect) {
        if self.debug_viz_visible {
            self.debug_viz.draw(draw, bounds);
        }
    }
}
