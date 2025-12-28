//! Visualization orchestration and cycling.
//!
//! Manages the visualization pipeline, including automatic cycling between effects
//! on detected musical transitions and overlay blending.

pub mod beat_bars;
pub mod black_hole;
pub mod crt_phosphor;
pub mod dancing_skeletons;
pub mod debug;
pub mod effects;
pub mod fractal_tree;
pub mod freq_mandala;
pub mod gravity_flames;
pub mod kaleidoscope;
pub mod lava_blobs;
pub mod particle_nebula;
pub mod psychedelic_spiral;
pub mod scripted;
pub mod shuffling_skeletons;
pub mod solar_beat;
pub mod spectro_road;
pub mod spiral_tunnel;
pub mod squares;
pub mod strobe_grid;
pub mod tesla_coil;

use enum_dispatch::enum_dispatch;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;
use crate::utils::DetectionConfig;

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
    &[VisLabel::Geometric, VisLabel::Intense], // 0: SolarBeat
    &[VisLabel::Geometric, VisLabel::Retro],   // 1: SpectroRoad
    &[VisLabel::Geometric],                    // 2: Squares
    &[VisLabel::Glitchy, VisLabel::Intense],   // 3: TeslaCoil
    &[VisLabel::Geometric],                    // 4: Kaleidoscope
    &[VisLabel::Organic],                      // 5: LavaBlobs
    &[VisLabel::Geometric, VisLabel::Retro],   // 6: BeatBars
    &[VisLabel::Retro, VisLabel::Glitchy],     // 7: CrtPhosphor
    &[VisLabel::Organic, VisLabel::Intense],   // 8: BlackHole
    &[VisLabel::Organic, VisLabel::Intense],   // 9: GravityFlames
    &[VisLabel::Organic],                      // 10: FractalTree
    &[VisLabel::Cartoon],                      // 11: DancingSkeletons
    &[VisLabel::Cartoon],                      // 12: ShufflingSkeletons
    &[VisLabel::Organic, VisLabel::Intense],   // 13: PsychedelicSpiral
    &[VisLabel::Geometric, VisLabel::Intense], // 14: SpiralTunnel
    &[VisLabel::Organic],                      // 15: ParticleNebula
    &[VisLabel::Geometric],                    // 16: FreqMandala
    &[VisLabel::Glitchy, VisLabel::Intense],   // 17: StrobeGrid
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
pub use effects::FeedbackRenderer;
pub use fractal_tree::FractalTree;
pub use freq_mandala::FreqMandala;
pub use gravity_flames::GravityFlames;
pub use kaleidoscope::Kaleidoscope;
pub use lava_blobs::LavaBlobs;
pub use particle_nebula::ParticleNebula;
pub use psychedelic_spiral::PsychedelicSpiral;
pub use scripted::ScriptManager;
pub use shuffling_skeletons::ShufflingSkeletons;
pub use solar_beat::SolarBeat;
pub use spectro_road::SpectroRoad;
pub use spiral_tunnel::SpiralTunnel;
pub use squares::Squares;
pub use strobe_grid::StrobeGrid;
pub use tesla_coil::TeslaCoil;

/// Generates the Viz enum, constructor, and name lookup
macro_rules! viz_enum {
    ($($name:ident),* $(,)?) => {
        #[enum_dispatch(Visualization)]
        pub enum Viz {
            $($name($name),)*
        }

        /// Names of all visualizations (indexed same as Viz::all())
        pub const VIZ_NAMES: &[&str] = &[$(stringify!($name),)*];

        impl Viz {
            /// Returns a Vec containing one instance of each visualization
            pub fn all() -> Vec<Viz> {
                vec![$(Viz::$name($name::default()),)*]
            }

            /// Get visualization name by index
            pub fn name(idx: usize) -> &'static str {
                VIZ_NAMES.get(idx).copied().unwrap_or("Unknown")
            }
        }
    };
}

viz_enum! {
    SolarBeat,
    SpectroRoad,
    Squares,
    TeslaCoil,
    Kaleidoscope,
    LavaBlobs,
    BeatBars,
    CrtPhosphor,
    BlackHole,
    GravityFlames,
    FractalTree,
    DancingSkeletons,
    ShufflingSkeletons,
    PsychedelicSpiral,
    SpiralTunnel,
    ParticleNebula,
    FreqMandala,
    StrobeGrid,
}

/// Trait that all visualizations must implement
#[enum_dispatch]
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

    pub fn current(force_windowed: bool) -> Self {
        if force_windowed || cfg!(debug_assertions) {
            Self::debug()
        } else {
            Self::release()
        }
    }
}

const NOTIFICATION_FRAMES: u32 = 180; // ~3 seconds at 60fps

/// Main renderer that manages the visualization pipeline and cycling
pub struct Renderer {
    visualizations: Vec<Viz>,
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
    /// Smoothed energy level for selection decisions
    tracked_energy: f32,
    /// Detection configuration (from config file)
    detection_config: DetectionConfig,
    /// Energy ranges for visualizations (from config file)
    viz_energy_ranges: Vec<[f32; 2]>,
}

impl Renderer {
    /// Creates a renderer that cycles between visualizations
    /// when audio transitions are detected, starting with a random one
    pub fn with_cycling(
        detection_config: DetectionConfig,
        viz_energy_ranges: Vec<[f32; 2]>,
    ) -> Self {
        let visualizations = Viz::all();

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
            tracked_energy: 0.5,
            detection_config,
            viz_energy_ranges,
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
            overlays
                .iter()
                .map(|&i| Self::visualization_name(i))
                .collect::<Vec<_>>()
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

    /// Returns indices of visualizations suitable for the given energy level
    fn vizs_for_energy(&self, energy: f32) -> Vec<usize> {
        self.viz_energy_ranges
            .iter()
            .enumerate()
            .filter(|(_, range)| energy >= range[0] && energy <= range[1])
            .map(|(i, _)| i)
            .collect()
    }

    /// Select visualizations matching both energy level and optionally labels
    /// Returns (primary_idx, overlay_indices)
    fn select_for_energy_and_labels(
        &self,
        rng: &mut impl rand::Rng,
        energy: f32,
        preferred_labels: Option<&[VisLabel]>,
    ) -> (usize, Vec<usize>) {
        // First filter by energy
        let energy_matches = self.vizs_for_energy(energy);

        if energy_matches.is_empty() {
            // Fallback: find closest energy match
            let closest = self
                .viz_energy_ranges
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let mid_a = (a[0] + a[1]) / 2.0;
                    let mid_b = (b[0] + b[1]) / 2.0;
                    (energy - mid_a)
                        .abs()
                        .partial_cmp(&(energy - mid_b).abs())
                        .unwrap()
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            return (closest, Vec::new());
        }

        // Then filter by labels if provided
        let candidates: Vec<usize> = if let Some(labels) = preferred_labels {
            energy_matches
                .iter()
                .filter(|&&i| {
                    VIZ_LABELS.get(i).map_or(false, |viz_labels| {
                        labels.iter().any(|l| viz_labels.contains(l))
                    })
                })
                .copied()
                .collect()
        } else {
            energy_matches.clone()
        };

        let final_candidates = if candidates.is_empty() {
            energy_matches
        } else {
            candidates
        };

        // Select 1-4 from candidates
        let count = rng.random_range(1..=4).min(final_candidates.len());
        let mut selected: Vec<usize> = Vec::with_capacity(count);

        while selected.len() < count {
            let idx = final_candidates[rng.random_range(0..final_candidates.len())];
            if !selected.contains(&idx) {
                selected.push(idx);
            }
        }

        let primary = selected[0];
        let overlays = selected[1..].to_vec();

        // println!(
        //     "Energy-based selection (energy={:.2}) → {} (primary: {}, overlays: {:?})",
        //     energy,
        //     selected.len(),
        //     Self::visualization_name(primary),
        //     overlays
        //         .iter()
        //         .map(|&i| Self::visualization_name(i))
        //         .collect::<Vec<_>>()
        // );

        (primary, overlays)
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
            self.cooldown = self.detection_config.cooldown_frames();
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
        self.cooldown = self.detection_config.cooldown_frames();
        self.locked = true;

        let name = Self::visualization_name(idx);
        println!("Locked to visualization {}: {}", idx, name);
        Some(name)
    }

    /// Get visualization name by index
    fn visualization_name(idx: usize) -> &'static str {
        Viz::name(idx)
    }

    pub fn update(&mut self, analysis: &AudioAnalysis, bounds: Rect) {
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

        // Track energy smoothly for selection decisions
        self.tracked_energy = self.tracked_energy * 0.9 + analysis.energy * 0.1;

        // Skip auto-switching if locked or in cooldown
        if !self.locked && self.cooldown == 0 && self.visualizations.len() > 1 {
            let mut rng = rand::rng();
            let cooldown_frames = self.detection_config.cooldown_frames();
            let energy_drop_rate = self.detection_config.energy_drop_rate();

            // Priority 1: Punch detection - major visual change
            if analysis.punch_detected {
                let (primary, overlays) =
                    self.select_for_energy_and_labels(&mut rng, analysis.energy, None);
                let overlay_count = overlays.len();
                self.current_idx = primary;
                self.overlay_indices = overlays;
                self.cooldown = cooldown_frames * 2; // Longer cooldown for punches
                println!(
                    "PUNCH! Switched to {} with {} overlays",
                    Self::visualization_name(primary),
                    overlay_count
                );
            }
            // Priority 2: Instrument added - add overlay
            else if analysis.instrument_added && self.overlay_indices.len() < 3 {
                let candidates = self.vizs_for_energy(analysis.energy);
                if !candidates.is_empty() {
                    let new_overlay = candidates[rng.random_range(0..candidates.len())];
                    if !self.overlay_indices.contains(&new_overlay)
                        && new_overlay != self.current_idx
                    {
                        self.overlay_indices.push(new_overlay);
                        self.cooldown = cooldown_frames / 2;
                        println!(
                            "Instrument added: +overlay {}",
                            Self::visualization_name(new_overlay)
                        );
                    }
                }
            }
            // Priority 3: Instrument removed OR energy dropping - remove overlay and maybe switch primary
            else if analysis.instrument_removed
                || (analysis.rise_rate < energy_drop_rate && self.tracked_energy < 0.4)
            {
                if !self.overlay_indices.is_empty() {
                    let removed = self.overlay_indices.pop();
                    println!(
                        "Energy/instrument drop: -overlay {:?}",
                        removed.map(Self::visualization_name)
                    );
                }

                // If energy is low, also switch primary to calmer viz
                if self.tracked_energy < 0.3 {
                    let calm_vizs = self.vizs_for_energy(self.tracked_energy);
                    if !calm_vizs.is_empty() && !calm_vizs.contains(&self.current_idx) {
                        self.current_idx = calm_vizs[rng.random_range(0..calm_vizs.len())];
                        self.cooldown = cooldown_frames;
                        println!(
                            "Energy low: switched to calmer {}",
                            Self::visualization_name(self.current_idx)
                        );
                    }
                }
            }
            // Priority 4: Break detected - dramatic change
            else if analysis.break_detected {
                let (primary, overlays) =
                    self.select_for_energy_and_labels(&mut rng, self.tracked_energy, None);
                self.current_idx = primary;
                self.overlay_indices = overlays;
                self.cooldown = cooldown_frames;
                // println!("Break! Switched to {}", Self::visualization_name(primary));
            }
            // Priority 5: Regular transition - existing behavior but energy-aware
            else if analysis.transition_detected {
                let (primary, overlays) =
                    self.select_for_energy_and_labels(&mut rng, self.tracked_energy, None);
                self.current_idx = primary;
                self.overlay_indices = overlays;
                self.cooldown = cooldown_frames;
            }
        }

        // Update the active visualization
        self.visualizations[self.current_idx].update(analysis);

        // Update overlay visualizations
        for &idx in &self.overlay_indices {
            self.visualizations[idx].update(analysis);
        }

        // Always update debug viz (even if not visible, so it's ready when toggled)
        self.debug_viz.update(analysis, bounds);
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
        } else {
            // Invisible draw to keep coordinate system synchronized with window size
            draw.rect()
                .x_y(bounds.x(), bounds.y())
                .w_h(bounds.w(), bounds.h())
                .color(srgba(0u8, 0u8, 0u8, 0u8));
        }
    }
}
