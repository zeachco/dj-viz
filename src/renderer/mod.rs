//! Visualization orchestration and cycling.
//!
//! Manages the visualization pipeline, including automatic cycling between effects
//! on detected musical transitions and overlay blending.

pub mod beat_bars;
pub mod black_hole;
pub mod crt_phosphor;
pub mod dancing_skeletons;
pub mod debug;
pub mod feedback;
pub mod fractal_tree;
pub mod gravity_flames;
pub mod kaleidoscope;
pub mod lava_blobs;
pub mod solar_beat;
pub mod spectro_road;
pub mod squares;
pub mod tesla_coil;

use nannou::prelude::*;

use crate::audio::AudioAnalysis;

/// Labels for categorizing visualizations that can be layered together
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisLabel {
    Organic,
    Geometric,
    Cartoon,
    Glitchy,
    Intense,
    Retro,
}

/// Labels for each visualization (indexed same as visualizations vec)
const VIZ_LABELS: &[&[VisLabel]] = &[
    &[VisLabel::Geometric, VisLabel::Intense],  // 0: SolarBeat
    &[VisLabel::Geometric, VisLabel::Retro],    // 1: SpectroRoad
    &[VisLabel::Geometric],                     // 2: Squares
    &[VisLabel::Glitchy, VisLabel::Intense],    // 3: TeslaCoil
    &[VisLabel::Geometric],                     // 4: Kaleidoscope
    &[VisLabel::Organic],                       // 5: LavaBlobs
    &[VisLabel::Geometric, VisLabel::Retro],    // 6: BeatBars
    &[VisLabel::Retro, VisLabel::Glitchy],      // 7: CrtPhosphor
    &[VisLabel::Organic, VisLabel::Intense],    // 8: BlackHole
    &[VisLabel::Organic, VisLabel::Intense],    // 9: GravityFlames
    &[VisLabel::Organic],                       // 10: FractalTree
    &[VisLabel::Cartoon],                       // 11: DancingSkeletons
];

const ALL_LABELS: &[VisLabel] = &[
    VisLabel::Organic,
    VisLabel::Geometric,
    VisLabel::Cartoon,
    VisLabel::Glitchy,
    VisLabel::Intense,
    VisLabel::Retro,
];

pub use beat_bars::BeatBars;
pub use black_hole::BlackHole;
pub use crt_phosphor::CrtPhosphor;
pub use dancing_skeletons::DancingSkeletons;
pub use debug::DebugViz;
pub use feedback::FeedbackRenderer;
pub use fractal_tree::FractalTree;
pub use gravity_flames::GravityFlames;
pub use kaleidoscope::Kaleidoscope;
pub use lava_blobs::LavaBlobs;
pub use solar_beat::SolarBeat;
pub use spectro_road::SpectroRoad;
pub use squares::Squares;
pub use tesla_coil::TeslaCoil;

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
            width: 640,
            height: 480,
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
    pub debug_viz_visible: bool,
}

impl Renderer {
    /// Creates a renderer that cycles between visualizations
    /// when audio transitions are detected, starting with a random one
    pub fn with_cycling() -> Self {
        let visualizations: Vec<Box<dyn Visualization>> = vec![
            Box::new(SolarBeat::new()),
            Box::new(SpectroRoad::new()),
            Box::new(Squares::new()),
            Box::new(TeslaCoil::new()),
            Box::new(Kaleidoscope::new()),
            Box::new(LavaBlobs::new()),
            Box::new(BeatBars::new()),
            Box::new(CrtPhosphor::new()),
            Box::new(BlackHole::new()),
            Box::new(GravityFlames::new()),
            Box::new(FractalTree::new()),
            Box::new(DancingSkeletons::new()),
        ];

        let mut rng = rand::rng();
        // Select initial visualizations by matching labels
        let (current_idx, overlay_indices) = Self::select_by_labels(&mut rng);

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

    /// Selects 1-4 visualizations by picking 1-2 random labels and finding matches
    /// Returns (primary_idx, overlay_indices)
    fn select_by_labels(rng: &mut impl rand::Rng) -> (usize, Vec<usize>) {
        // Pick 1 or 2 random labels
        let num_labels = rng.random_range(1..=2);
        let mut selected_labels = Vec::with_capacity(num_labels);

        while selected_labels.len() < num_labels {
            let label = ALL_LABELS[rng.random_range(0..ALL_LABELS.len())];
            if !selected_labels.contains(&label) {
                selected_labels.push(label);
            }
        }

        // Find all visualizations matching ANY of the selected labels
        let matching: Vec<usize> = VIZ_LABELS
            .iter()
            .enumerate()
            .filter(|(_, labels)| selected_labels.iter().any(|l| labels.contains(l)))
            .map(|(i, _)| i)
            .collect();

        if matching.is_empty() {
            // Fallback to first visualization
            return (0, Vec::new());
        }

        // Select 1-4 from matching
        let count = rng.random_range(1..=4).min(matching.len());
        let mut selected: Vec<usize> = Vec::with_capacity(count);

        while selected.len() < count {
            let idx = matching[rng.random_range(0..matching.len())];
            if !selected.contains(&idx) {
                selected.push(idx);
            }
        }

        // First one is primary, rest are overlays
        let primary = selected[0];
        let overlays = selected[1..].to_vec();

        println!(
            "Selected labels {:?} → {} visualizations (primary: {}, overlays: {:?})",
            selected_labels,
            selected.len(),
            Self::visualization_name(primary),
            overlays.iter().map(|&i| Self::visualization_name(i)).collect::<Vec<_>>()
        );

        (primary, overlays)
    }

    /// Selects new visualizations based on matching labels
    fn select_new_visualizations(&mut self) {
        let mut rng = rand::rng();
        let (primary, overlays) = Self::select_by_labels(&mut rng);
        self.current_idx = primary;
        self.overlay_indices = overlays;
    }

    /// Shows a notification message for 3 seconds
    pub fn show_notification(&mut self, text: String) {
        self.notification_text = Some(text);
        self.notification_frames = NOTIFICATION_FRAMES;
    }

    /// Manually cycle to new visualizations (unlocks auto-cycling)
    /// Selects new set based on matching labels
    pub fn cycle_next(&mut self, _analysis: &AudioAnalysis) {
        if self.visualizations.len() > 1 {
            self.select_new_visualizations();
            self.cooldown = COOLDOWN_FRAMES;
            self.locked = false; // Space unlocks and resumes auto-cycling
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

    /// Get visualization name by index
    fn visualization_name(idx: usize) -> &'static str {
        match idx {
            0 => "SolarBeat",
            1 => "SpectroRoad",
            2 => "Squares",
            3 => "TeslaCoil",
            4 => "Kaleidoscope",
            5 => "LavaBlobs",
            6 => "BeatBars",
            7 => "CrtPhosphor",
            8 => "BlackHole",
            9 => "GravityFlames",
            10 => "FractalTree",
            11 => "DancingSkeletons",
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
            // Select new visualizations based on matching labels
            self.select_new_visualizations();
            self.cooldown = COOLDOWN_FRAMES;
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
