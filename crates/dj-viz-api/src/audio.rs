//! ABI-stable audio analysis types

use abi_stable::{std_types::RVec, StableAbi};

pub const NUM_BANDS: usize = 8;
pub const SPECTRUM_SIZE: usize = 1024;

/// FFI-safe version of AudioAnalysis
#[repr(C)]
#[derive(StableAbi, Clone)]
pub struct AudioAnalysisFFI {
    /// Energy in each frequency band (0-1, smoothed)
    pub bands: [f32; NUM_BANDS],

    /// Full frequency spectrum magnitudes (0-1 normalized, SPECTRUM_SIZE bins)
    pub spectrum: RVec<f32>,

    /// Difference from previous frame's spectrum
    pub spectrum_diff: RVec<f32>,

    /// Bands normalized relative to tracked min/max range
    pub bands_normalized: [f32; NUM_BANDS],

    /// Overall energy/volume (0-1)
    pub energy: f32,

    /// Bass energy (bands 0-1 combined)
    pub bass: f32,

    /// Mid energy (bands 2-4 combined)
    pub mids: f32,

    /// Treble energy (bands 5-7 combined)
    pub treble: f32,

    /// Whether a musical transition was detected
    pub transition_detected: bool,

    /// Whether a punch (calm-to-high energy spike) was detected
    pub punch_detected: bool,

    /// Whether a break/subdivision pattern was detected
    pub break_detected: bool,

    /// Whether spectral complexity increased (instrument added)
    pub instrument_added: bool,

    /// Whether spectral complexity decreased (instrument removed)
    pub instrument_removed: bool,

    /// Whether a kick drum was detected this frame
    pub kick_detected: bool,

    /// Confidence of kick detection (0-1)
    pub kick_confidence: f32,

    /// Band envelopes for kick detection [sub_bass, low_mid, attack]
    pub kick_envelopes: [f32; 3],

    /// Spectral flux per kick band [sub_bass, low_mid, attack]
    pub kick_flux: [f32; 3],

    /// Time since last kick in seconds
    pub kick_time_since: f32,

    /// Estimated tempo in beats per minute (smoothed)
    pub bpm: f32,

    /// Index of the dominant frequency band (0-7)
    pub dominant_band: usize,

    /// Difference between current energy and lagged energy
    pub energy_diff: f32,

    /// Rate at which energy is rising (positive = rising)
    pub rise_rate: f32,

    /// Weighted average frequency (spectral centroid in Hz)
    pub spectral_centroid: f32,

    /// Steps (frames) since last drastic band change
    pub last_mark: u32,

    /// Whether zoom direction should shift
    pub zoom_direction_shift: bool,
}

impl Default for AudioAnalysisFFI {
    fn default() -> Self {
        Self {
            bands: [0.0; NUM_BANDS],
            spectrum: RVec::new(),
            spectrum_diff: RVec::new(),
            bands_normalized: [0.0; NUM_BANDS],
            energy: 0.0,
            bass: 0.0,
            mids: 0.0,
            treble: 0.0,
            transition_detected: false,
            punch_detected: false,
            break_detected: false,
            instrument_added: false,
            instrument_removed: false,
            kick_detected: false,
            kick_confidence: 0.0,
            kick_envelopes: [0.0; 3],
            kick_flux: [0.0; 3],
            kick_time_since: 0.0,
            bpm: 0.0,
            dominant_band: 0,
            energy_diff: 0.0,
            rise_rate: 0.0,
            spectral_centroid: 0.0,
            last_mark: 0,
            zoom_direction_shift: false,
        }
    }
}

// NOTE: Conversion from native AudioAnalysis will be implemented
// in the main binary where both types are available
