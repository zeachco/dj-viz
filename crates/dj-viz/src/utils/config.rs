//! Configuration file management.
//!
//! Handles loading and saving user preferences to `~/.dj-viz.toml`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_DEVICE_TIMEOUT_SECS: u64 = 3;

const CONFIG_TEMPLATE: &str = r#"# dj-viz configuration file

# Timeout in seconds when switching audio devices (default: 3)
# device_timeout_secs = 3

# Last selected audio device (auto-saved)
# last_device = "Device Name"
# last_device_is_input = false

# Last selected PipeWire stream target (auto-saved)
# pw_link_target = "Spotify:output_FL"

# =============================================================================
# Detection Thresholds
# =============================================================================

# Punch detection (calm-before-spike)
# punch_floor_threshold = 0.25    # Max energy floor for "calm" state
# punch_spike_threshold = 0.4     # Min energy jump above floor to trigger
# punch_rise_rate = 0.2           # Min rise rate required
# punch_cooldown_frames = 30      # Frames between punches (~0.5s at 60fps)

# Break detection (silence-based)
# break_silence_frames = 90       # Frames without beat to trigger break (~1.5s)
# break_cooldown_frames = 180     # Cooldown between break detections (~3s)

# Instrument detection (spectral complexity)
# complexity_threshold = 0.15     # Band energy threshold to count as active
# complexity_change_ratio = 1.5   # Ratio change to trigger add/remove

# Visualization switching
# cooldown_frames = 45            # Base cooldown between switches (~0.75s)
# energy_drop_rate = -0.15        # Rise rate below this = energy dropping

# =============================================================================
# Visualization Energy Ranges [min, max]
# =============================================================================
# Each visualization has an energy range it works best at.
# Format: [[min, max], ...] indexed by visualization order.

# viz_energy_ranges = [
#   [0.5, 1.0],  # SolarBeat
#   [0.3, 0.8],  # SpectroRoad
#   [0.2, 0.6],  # Squares
#   [0.6, 1.0],  # TeslaCoil
#   [0.3, 0.7],  # Kaleidoscope
#   [0.2, 0.5],  # LavaBlobs
#   [0.4, 0.9],  # BeatBars
#   [0.3, 0.7],  # CrtPhosphor
#   [0.5, 1.0],  # BlackHole
#   [0.1, 0.6],  # GravityFlames
#   [0.1, 0.4],  # FractalTree
#   [0.3, 0.7],  # DancingSkeletons
#   [0.2, 0.6],  # ShufflingSkeletons
#   [0.5, 1.0],  # PsychedelicSpiral
#   [0.6, 1.0],  # SpiralTunnel
#   [0.1, 0.5],  # ParticleNebula
#   [0.3, 0.7],  # FreqMandala
#   [0.7, 1.0],  # StrobeGrid
# ]
"#;

/// Detection thresholds configuration
#[derive(Serialize, Deserialize, Clone)]
pub struct DetectionConfig {
    // Punch detection
    pub punch_floor_threshold: Option<f32>,
    pub punch_spike_threshold: Option<f32>,
    pub punch_rise_rate: Option<f32>,
    pub punch_cooldown_frames: Option<u32>,

    // Break detection (silence-based)
    pub break_silence_frames: Option<u32>, // Frames without beat to trigger break
    pub break_cooldown_frames: Option<u32>, // Cooldown between break detections

    // Instrument detection
    pub complexity_threshold: Option<f32>,
    pub complexity_change_ratio: Option<f32>,

    // Visualization switching
    pub cooldown_frames: Option<u32>,
    pub energy_drop_rate: Option<f32>,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            punch_floor_threshold: None,
            punch_spike_threshold: None,
            punch_rise_rate: None,
            punch_cooldown_frames: None,
            break_silence_frames: None,
            break_cooldown_frames: None,
            complexity_threshold: None,
            complexity_change_ratio: None,
            cooldown_frames: None,
            energy_drop_rate: None,
        }
    }
}

impl DetectionConfig {
    // Punch detection defaults
    pub fn punch_floor_threshold(&self) -> f32 {
        self.punch_floor_threshold.unwrap_or(0.25)
    }
    pub fn punch_spike_threshold(&self) -> f32 {
        self.punch_spike_threshold.unwrap_or(0.4)
    }
    pub fn punch_rise_rate(&self) -> f32 {
        self.punch_rise_rate.unwrap_or(0.2)
    }
    pub fn punch_cooldown_frames(&self) -> u32 {
        self.punch_cooldown_frames.unwrap_or(30)
    }

    // Break detection defaults
    pub fn break_silence_frames(&self) -> u32 {
        self.break_silence_frames.unwrap_or(90) // ~1.5 seconds at 60fps
    }
    pub fn break_cooldown_frames(&self) -> u32 {
        self.break_cooldown_frames.unwrap_or(180) // 3 seconds between break detections
    }

    // Instrument detection defaults
    pub fn complexity_threshold(&self) -> f32 {
        self.complexity_threshold.unwrap_or(0.15)
    }
    pub fn complexity_change_ratio(&self) -> f32 {
        self.complexity_change_ratio.unwrap_or(1.5)
    }

    // Visualization switching defaults
    pub fn cooldown_frames(&self) -> u32 {
        self.cooldown_frames.unwrap_or(45)
    }
    pub fn energy_drop_rate(&self) -> f32 {
        self.energy_drop_rate.unwrap_or(-0.15)
    }
}

/// Default energy ranges for visualizations
pub const DEFAULT_VIZ_ENERGY_RANGES: &[[f32; 2]; 18] = &[
    [0.5, 0.9], // SolarBeat
    [0.8, 1.0], // SpectroRoad
    [0.4, 0.6], // Squares
    [0.6, 1.0], // TeslaCoil
    [0.3, 0.7], // Kaleidoscope
    [0.1, 0.5], // LavaBlobs
    [0.4, 0.9], // BeatBars
    [0.3, 0.7], // CrtPhosphor
    [0.5, 1.0], // BlackHole
    [0.1, 0.6], // GravityFlames
    [0.0, 0.4], // FractalTree
    [0.1, 0.6], // DancingSkeletons
    [0.7, 1.0], // ShufflingSkeletons
    [0.3, 0.9], // PsychedelicSpiral
    [0.6, 1.0], // SpiralTunnel
    [0.1, 0.5], // ParticleNebula
    [0.3, 0.7], // FreqMandala
    [0.5, 1.0], // StrobeGrid
];

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub last_device: Option<String>,
    pub last_device_is_input: Option<bool>,
    pub device_timeout_secs: Option<u64>,
    pub pw_link_target: Option<String>,

    // Detection thresholds (flattened for simpler TOML)
    pub punch_floor_threshold: Option<f32>,
    pub punch_spike_threshold: Option<f32>,
    pub punch_rise_rate: Option<f32>,
    pub punch_cooldown_frames: Option<u32>,
    pub break_silence_frames: Option<u32>,
    pub break_cooldown_frames: Option<u32>,
    pub complexity_threshold: Option<f32>,
    pub complexity_change_ratio: Option<f32>,
    pub cooldown_frames: Option<u32>,
    pub energy_drop_rate: Option<f32>,

    // Visualization energy ranges
    pub viz_energy_ranges: Option<Vec<[f32; 2]>>,
}

impl Config {
    fn path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".dj-viz.toml"))
    }

    pub fn load() -> Self {
        let path = match Self::path() {
            Some(p) => p,
            None => return Self::default(),
        };

        // Create template file if it doesn't exist
        if !path.exists() {
            let _ = fs::write(&path, CONFIG_TEMPLATE);
            println!("Created config template at {:?}", path);
        }

        fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn device_timeout_secs(&self) -> u64 {
        self.device_timeout_secs
            .unwrap_or(DEFAULT_DEVICE_TIMEOUT_SECS)
    }

    pub fn save(&self) {
        if let Some(path) = Self::path() {
            if let Ok(content) = toml::to_string(self) {
                let _ = fs::write(&path, &content);
                println!("Config saved to {:?}", path);
            }
        }
    }

    pub fn set_device(&mut self, name: &str, is_input: bool) {
        self.last_device = Some(name.to_string());
        self.last_device_is_input = Some(is_input);
        self.save();
    }

    /// Get detection configuration with defaults
    pub fn detection(&self) -> DetectionConfig {
        DetectionConfig {
            punch_floor_threshold: self.punch_floor_threshold,
            punch_spike_threshold: self.punch_spike_threshold,
            punch_rise_rate: self.punch_rise_rate,
            punch_cooldown_frames: self.punch_cooldown_frames,
            break_silence_frames: self.break_silence_frames,
            break_cooldown_frames: self.break_cooldown_frames,
            complexity_threshold: self.complexity_threshold,
            complexity_change_ratio: self.complexity_change_ratio,
            cooldown_frames: self.cooldown_frames,
            energy_drop_rate: self.energy_drop_rate,
        }
    }

    /// Get visualization energy ranges (with defaults if not configured)
    pub fn viz_energy_ranges(&self) -> Vec<[f32; 2]> {
        self.viz_energy_ranges
            .clone()
            .unwrap_or_else(|| DEFAULT_VIZ_ENERGY_RANGES.to_vec())
    }
}
