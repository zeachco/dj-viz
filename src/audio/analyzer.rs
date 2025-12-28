//! Audio analysis and FFT processing.
//!
//! Performs real-time FFT analysis on audio samples to extract frequency band energies,
//! detect beats/transitions, and compute aggregate metrics (bass, mids, treble).

use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

use crate::utils::DetectionConfig;

/// Number of frequency bands for visualization
pub const NUM_BANDS: usize = 8;

/// FFT size - needs to be large enough for good low-frequency resolution
/// At 44.1kHz: 2048 gives ~21.5 Hz bins (good for 20-60 Hz bass range)
const FFT_SIZE: usize = 2048;

/// Frequency band boundaries (Hz) for 44.1kHz sample rate
/// Sub-bass, Bass, Low-mid, Mid, Upper-mid, Presence, Brilliance, Air
const BAND_EDGES: [f32; NUM_BANDS + 1] = [
    20.0, 60.0, 250.0, 500.0, 2000.0, 4000.0, 6000.0, 12000.0, 20000.0,
];

/// Number of spectrum bins to expose (half of FFT size, up to Nyquist)
pub const SPECTRUM_SIZE: usize = FFT_SIZE / 2;

/// Pre-computed analysis results - no allocations needed by visualizations
#[derive(Clone)]
pub struct AudioAnalysis {
    /// Energy in each frequency band (0-1, smoothed)
    pub bands: [f32; NUM_BANDS],
    /// Full frequency spectrum magnitudes (0-1 normalized, SPECTRUM_SIZE bins)
    /// Index 0 = DC, Index N = N * sample_rate / FFT_SIZE Hz
    pub spectrum: Vec<f32>,
    /// Difference from previous frame's spectrum (for velocity/change visualization)
    pub spectrum_diff: Vec<f32>,
    /// Bands normalized relative to tracked min/max range (can be outside 0-1)
    /// If a band oscillates between 0.6-0.9, this maps it to 0.0-1.0 range
    pub bands_normalized: [f32; NUM_BANDS],
    /// Tracked minimum values for each band
    pub band_mins: [f32; NUM_BANDS],
    /// Tracked maximum values for each band
    pub band_maxs: [f32; NUM_BANDS],
    /// Overall energy/volume (0-1)
    pub energy: f32,
    /// Whether a musical transition was detected
    pub transition_detected: bool,
    /// Bass energy (bands 0-1 combined)
    pub bass: f32,
    /// Mid energy (bands 2-4 combined)
    pub mids: f32,
    /// Treble energy (bands 5-7 combined)
    pub treble: f32,
    /// Difference between current energy and lagged energy (can be negative)
    pub energy_diff: f32,
    /// Whether zoom direction should shift (triggered when energy_diff crosses ±0.15)
    pub zoom_direction_shift: bool,
    /// Estimated tempo in beats per minute (smoothed)
    pub bpm: f32,
    /// Index of the dominant frequency band (0-7, updated max once per second)
    pub dominant_band: usize,
    /// Steps (frames) since last drastic band change (resets on major energy shift)
    pub last_mark: u32,
    /// Whether a visualization change should be triggered (drastic change + high energy)
    pub viz_change_triggered: bool,

    // Punch detection (calm-before-spike)
    /// Whether a punch (calm-to-high energy spike) was detected
    pub punch_detected: bool,
    /// Tracked minimum energy floor for punch detection
    pub energy_floor: f32,
    /// Rate at which energy is rising (positive = rising)
    pub rise_rate: f32,

    // Beat subdivision detection
    /// Whether a break/subdivision pattern was detected
    pub break_detected: bool,

    // Instrument/complexity detection
    /// Whether spectral complexity increased (instrument added)
    pub instrument_added: bool,
    /// Whether spectral complexity decreased (instrument removed)
    pub instrument_removed: bool,
    /// Weighted average frequency (spectral centroid in Hz)
    pub spectral_centroid: f32,
}

impl Default for AudioAnalysis {
    fn default() -> Self {
        Self {
            bands: [0.0; NUM_BANDS],
            spectrum: vec![0.0; SPECTRUM_SIZE],
            spectrum_diff: vec![0.0; SPECTRUM_SIZE],
            bands_normalized: [0.0; NUM_BANDS],
            band_mins: [0.0; NUM_BANDS],
            band_maxs: [0.0; NUM_BANDS],
            energy: 0.0,
            transition_detected: false,
            bass: 0.0,
            mids: 0.0,
            treble: 0.0,
            energy_diff: 0.0,
            zoom_direction_shift: false,
            bpm: 0.0,
            dominant_band: 0,
            last_mark: 600, // Start at max (10 seconds at 60fps)
            viz_change_triggered: false,
            // Punch detection
            punch_detected: false,
            energy_floor: 0.0,
            rise_rate: 0.0,
            // Break detection
            break_detected: false,
            // Instrument detection
            instrument_added: false,
            instrument_removed: false,
            spectral_centroid: 1000.0,
        }
    }
}

/// Centralized audio analyzer - performs FFT once and extracts all needed metrics
pub struct AudioAnalyzer {
    // FFT resources (pre-allocated)
    fft: Arc<dyn Fft<f32>>,
    fft_buffer: Vec<Complex<f32>>,
    fft_window: Vec<f32>,

    // Band bin ranges (pre-computed)
    band_bins: [(usize, usize); NUM_BANDS],

    // Smoothed values
    smoothed_bands: [f32; NUM_BANDS],
    smoothed_energy: f32,
    lagged_energy: f32,

    // Transition detection state
    energy_history: Vec<f32>,
    freq_ratio_history: Vec<f32>,
    history_idx: usize,
    was_high_energy: bool,
    was_high_freq: bool,

    // Peak detection
    prev_bands: [f32; NUM_BANDS],

    // Min/max tracking for normalization (slowly drift towards 0)
    band_mins: [f32; NUM_BANDS],
    band_maxs: [f32; NUM_BANDS],

    // Zoom direction shift detection
    prev_energy_diff: f32,

    // BPM detection
    beat_times: Vec<f32>,      // Timestamps of recent beats (in seconds)
    last_beat_time: f32,       // Last detected beat time
    smoothed_bpm: f32,         // Smoothed BPM estimate
    locked_bpm: f32,           // Locked BPM (only updates with high confidence)
    bpm_confidence: u32,       // Number of consistent readings
    frame_time: f32,           // Accumulated time for timestamping
    prev_bass_energy: f32,     // Previous frame's bass energy for onset detection
    bass_energy_avg: f32,      // Running average of bass energy for threshold
    low_bass_frames: u32,      // Frames with low bass (for break detection)

    // Dominant band detection
    dominant_band: usize,           // Current dominant band index
    last_dominant_update_time: f32, // Last time dominant band was updated

    // Drastic band change detection (last_mark)
    last_mark: u32,                    // Frames since last drastic change
    reference_bands: [f32; NUM_BANDS], // Reference bands for comparison
    viz_change_cooldown: u32,          // Cooldown frames before viz_change can trigger again

    // Frame skipping for performance
    frame_count: u32,
    last_analysis: AudioAnalysis,

    // Punch detection state
    energy_floor: f32,
    punch_cooldown: u32,

    // Break detection state (silence-based)
    frames_since_beat: u32,
    break_cooldown: u32,

    // Spectral complexity tracking
    spectral_complexity: f32,
    prev_spectral_complexity: f32,

    // Full spectrum tracking (pre-allocated, reused each frame)
    spectrum: Vec<f32>,
    spectrum_diff: Vec<f32>,
    spectrum_mins: Vec<f32>,
    spectrum_maxs: Vec<f32>,

    // Detection configuration (from config file)
    detection_config: DetectionConfig,
}

impl AudioAnalyzer {
    pub fn with_config(sample_rate: f32, detection_config: DetectionConfig) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        // Pre-compute Hann window
        let fft_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()))
            .collect();

        // Pre-compute which FFT bins correspond to each frequency band
        let bin_width = sample_rate / FFT_SIZE as f32;
        let mut band_bins = [(0usize, 0usize); NUM_BANDS];

        for i in 0..NUM_BANDS {
            let low_bin = (BAND_EDGES[i] / bin_width).floor() as usize;
            let high_bin = (BAND_EDGES[i + 1] / bin_width).ceil() as usize;
            band_bins[i] = (low_bin.max(1), high_bin.min(FFT_SIZE / 2));
        }

        const HISTORY_SIZE: usize = 300; // ~5 seconds at 60fps for stable detection
        const BPM_HISTORY_SIZE: usize = 16; // Track last 16 beats for stable BPM

        Self {
            fft,
            fft_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            fft_window,
            band_bins,
            smoothed_bands: [0.0; NUM_BANDS],
            smoothed_energy: 0.0,
            lagged_energy: 0.0,
            energy_history: vec![0.0; HISTORY_SIZE],
            freq_ratio_history: vec![0.0; HISTORY_SIZE],
            history_idx: 0,
            was_high_energy: false,
            was_high_freq: false,
            prev_bands: [0.0; NUM_BANDS],
            band_mins: [0.0; NUM_BANDS],
            band_maxs: [0.0; NUM_BANDS],
            prev_energy_diff: 0.0,
            beat_times: Vec::with_capacity(BPM_HISTORY_SIZE),
            last_beat_time: 0.0,
            smoothed_bpm: 0.0,
            locked_bpm: 0.0,
            bpm_confidence: 0,
            frame_time: 0.0,
            prev_bass_energy: 0.0,
            bass_energy_avg: 0.0,
            low_bass_frames: 0,
            dominant_band: 0,
            last_dominant_update_time: 0.0,
            last_mark: 600, // Start at max (10 seconds at 60fps)
            reference_bands: [0.0; NUM_BANDS],
            viz_change_cooldown: 0,
            frame_count: 0,
            last_analysis: AudioAnalysis::default(),
            // Punch detection
            energy_floor: 0.0,
            punch_cooldown: 0,
            // Break detection
            frames_since_beat: 0,
            break_cooldown: 0,
            // Spectral complexity
            spectral_complexity: 0.0,
            prev_spectral_complexity: 0.0,
            // Full spectrum tracking (pre-allocated)
            spectrum: vec![0.0; SPECTRUM_SIZE],
            spectrum_diff: vec![0.0; SPECTRUM_SIZE],
            spectrum_mins: vec![0.0; SPECTRUM_SIZE],
            spectrum_maxs: vec![0.0; SPECTRUM_SIZE],
            // Detection config
            detection_config,
        }
    }

    /// Analyze audio samples. Call once per frame.
    /// Returns cached result if called multiple times per frame.
    pub fn analyze(&mut self, samples: &[f32]) -> AudioAnalysis {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Update frame time (assuming ~60fps = 0.0167s per frame)
        const FRAME_DELTA: f32 = 1.0 / 60.0;
        self.frame_time += FRAME_DELTA;

        // Take FFT_SIZE samples from the input (or pad with zeros)
        let sample_count = samples.len().min(FFT_SIZE);

        // Apply window and fill buffer (reusing pre-allocated buffer)
        for i in 0..FFT_SIZE {
            if i < sample_count {
                self.fft_buffer[i] = Complex::new(samples[i] * self.fft_window[i], 0.0);
            } else {
                self.fft_buffer[i] = Complex::new(0.0, 0.0);
            }
        }

        // Perform FFT
        self.fft.process(&mut self.fft_buffer);

        // Calculate band energies
        let mut bands_raw = [0.0f32; NUM_BANDS];

        for (i, &(low, high)) in self.band_bins.iter().enumerate() {
            if high > low {
                let energy: f32 = self.fft_buffer[low..high]
                    .iter()
                    .map(|c| c.norm_sqr())
                    .sum();

                // Normalize and convert to dB-ish scale
                let avg_energy = energy / (high - low) as f32;

                // Convert to dB scale and do initial rough normalization
                let db = 10.0 * (avg_energy + 1e-10).log10();
                let rough_normalized = ((db + 100.0) / 160.0).clamp(0.0, 1.0); // Rough -100 to +60 dB range

                // Adaptive normalization: track min/max of the output (0-1 range)
                // This creates perceptual adaptation - sustained intensity becomes less intense
                const MIN_DRIFT: f32 = 0.985; // Faster drift towards current (~1 sec at 60fps)
                const MAX_DRIFT: f32 = 0.985;

                // Update minimum - track lowest output, slowly drift up towards current
                if rough_normalized < self.band_mins[i] || self.band_mins[i] == 0.0 {
                    self.band_mins[i] = rough_normalized;
                } else {
                    // Drift upwards towards current value
                    self.band_mins[i] =
                        self.band_mins[i] * MIN_DRIFT + rough_normalized * (1.0 - MIN_DRIFT);
                }

                // Update maximum - track highest output, slowly drift down towards current
                if rough_normalized > self.band_maxs[i] {
                    self.band_maxs[i] = rough_normalized;
                } else {
                    // Drift downwards towards current value
                    self.band_maxs[i] =
                        self.band_maxs[i] * MAX_DRIFT + rough_normalized * (1.0 - MAX_DRIFT);
                }

                // Re-normalize using tracked min/max to utilize full 0-1 range
                // If something stays intense, min drifts up and range shrinks, making it less intense
                let range = (self.band_maxs[i] - self.band_mins[i]).max(0.01); // Prevent division by zero
                let normalized = ((rough_normalized - self.band_mins[i]) / range).clamp(0.0, 1.0);

                bands_raw[i] = normalized;
            }
        }

        // Calculate full spectrum magnitudes (for visualizations that want specific frequencies)
        // Reuse pre-allocated buffers to avoid allocations per frame
        const SPECTRUM_MIN_DRIFT: f32 = 0.99;   // Min adapts in ~1-2 seconds
        const SPECTRUM_MAX_DRIFT: f32 = 0.999;  // Max decays very slowly (~10 sec to 50%) to avoid spikes during quiet moments

        for i in 1..SPECTRUM_SIZE {
            // Get magnitude in dB scale
            let magnitude = self.fft_buffer[i].norm_sqr();
            let db = 10.0 * (magnitude + 1e-10).log10();
            let rough_normalized = ((db + 100.0) / 160.0).clamp(0.0, 1.0);

            // Adaptive min/max tracking per bin
            if rough_normalized < self.spectrum_mins[i] || self.spectrum_mins[i] == 0.0 {
                self.spectrum_mins[i] = rough_normalized;
            } else {
                self.spectrum_mins[i] =
                    self.spectrum_mins[i] * SPECTRUM_MIN_DRIFT + rough_normalized * (1.0 - SPECTRUM_MIN_DRIFT);
            }

            if rough_normalized > self.spectrum_maxs[i] {
                self.spectrum_maxs[i] = rough_normalized;
            } else {
                self.spectrum_maxs[i] =
                    self.spectrum_maxs[i] * SPECTRUM_MAX_DRIFT + rough_normalized * (1.0 - SPECTRUM_MAX_DRIFT);
            }

            // Normalize to 0-1 using tracked range
            let range = (self.spectrum_maxs[i] - self.spectrum_mins[i]).max(0.01);
            let prev_val = self.spectrum[i];
            self.spectrum[i] = ((rough_normalized - self.spectrum_mins[i]) / range).clamp(0.0, 1.0);

            // Compute diff from previous frame (store previous before overwriting)
            self.spectrum_diff[i] = self.spectrum[i] - prev_val;
        }

        // Smooth bands (fast attack, slower decay)
        let attack = 0.7;
        let decay = 0.15;
        for i in 0..NUM_BANDS {
            if bands_raw[i] > self.smoothed_bands[i] {
                self.smoothed_bands[i] =
                    self.smoothed_bands[i] * (1.0 - attack) + bands_raw[i] * attack;
            } else {
                self.smoothed_bands[i] =
                    self.smoothed_bands[i] * (1.0 - decay) + bands_raw[i] * decay;
            }
        }

        // Calculate overall energy (use max band value instead of average)
        let energy_raw: f32 = bands_raw.iter().cloned().fold(0.0f32, f32::max);
        if energy_raw > self.smoothed_energy {
            self.smoothed_energy = self.smoothed_energy * 0.3 + energy_raw * 0.7;
        } else {
            self.smoothed_energy = self.smoothed_energy * 0.9 + energy_raw * 0.1;
        }

        // Update lagged energy with much slower smoothing (creates lag effect)
        self.lagged_energy = self.lagged_energy * 0.95 + self.smoothed_energy * 0.05;

        // Compute energy difference (positive = rising energy, negative = falling)
        let energy_diff = self.smoothed_energy - self.lagged_energy;

        self.prev_energy_diff = energy_diff;
        self.prev_bands = bands_raw;

        // Transition detection
        let transition_detected = self.detect_transition(energy_raw, &bands_raw);

        // BPM detection using bass onset detection
        // Use sub-bass + bass bands for beat detection (where kick drums live)
        let bass_energy = (bands_raw[0] + bands_raw[1]) / 2.0;

        // Update running average of bass energy (very slow adaptation for stability)
        const BASS_AVG_DECAY: f32 = 0.995; // ~3 seconds to adapt at 60fps
        self.bass_energy_avg = self.bass_energy_avg * BASS_AVG_DECAY + bass_energy * (1.0 - BASS_AVG_DECAY);

        // Track low bass periods (breaks in techno)
        const LOW_BASS_THRESHOLD: f32 = 0.15; // Bass below this = likely in a break
        const BREAK_FRAMES: u32 = 30;         // ~0.5 sec of low bass = break
        if bass_energy < LOW_BASS_THRESHOLD {
            self.low_bass_frames = self.low_bass_frames.saturating_add(1);
        } else {
            self.low_bass_frames = self.low_bass_frames.saturating_sub(2); // Faster recovery
        }
        let in_break = self.low_bass_frames > BREAK_FRAMES;

        // During breaks: freeze BPM updates, use locked value
        // This prevents BPM drift when kicks drop out
        if in_break {
            // Don't update beat detection during breaks
            // Just use the locked BPM (or smoothed if no lock yet)
            if self.locked_bpm > 0.0 {
                self.smoothed_bpm = self.locked_bpm;
            }
        } else {
            // Normal beat detection when not in break
            // Detect beat: bass energy rising sharply above recent average
            const BEAT_THRESHOLD_RATIO: f32 = 1.5; // Current must be 50% above average
            const MIN_BASS_FOR_BEAT: f32 = 0.2;    // Minimum absolute bass level
            let is_onset = bass_energy > self.prev_bass_energy
                && bass_energy > self.bass_energy_avg * BEAT_THRESHOLD_RATIO
                && bass_energy > MIN_BASS_FOR_BEAT;

            if is_onset {
                let time_since_last_beat = self.frame_time - self.last_beat_time;

                // Only count as a beat if enough time has passed (avoid double-counting)
                // Min 0.3s allows up to 200 BPM, max 1.5s filters out false positives
                const MIN_BEAT_INTERVAL: f32 = 0.3;
                const MAX_BEAT_INTERVAL: f32 = 1.5;
                if time_since_last_beat >= MIN_BEAT_INTERVAL && time_since_last_beat <= MAX_BEAT_INTERVAL {
                    self.beat_times.push(self.frame_time);
                    self.last_beat_time = self.frame_time;

                    // Keep only last 16 beats (~8-16 seconds of history for stable BPM)
                    const MAX_BEAT_HISTORY: usize = 16;
                    if self.beat_times.len() > MAX_BEAT_HISTORY {
                        self.beat_times.remove(0);
                    }

                    // Calculate BPM from intervals between beats
                    // Require more beats for stable reading
                    if self.beat_times.len() >= 8 {
                        let mut intervals = Vec::new();
                        for i in 1..self.beat_times.len() {
                            intervals.push(self.beat_times[i] - self.beat_times[i - 1]);
                        }

                        // Use median interval instead of average (more robust to outliers)
                        intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let median_interval = intervals[intervals.len() / 2];

                        // Convert to BPM (beats per minute)
                        let instant_bpm = 60.0 / median_interval;

                        // Clamp to reasonable BPM range (60-200)
                        let clamped_bpm = instant_bpm.clamp(60.0, 200.0);

                        // Check if this reading is consistent with smoothed BPM
                        let is_consistent = self.smoothed_bpm == 0.0
                            || (clamped_bpm - self.smoothed_bpm).abs() / self.smoothed_bpm < 0.1;

                        if is_consistent {
                            self.bpm_confidence = self.bpm_confidence.saturating_add(1);
                        } else {
                            self.bpm_confidence = self.bpm_confidence.saturating_sub(1);
                        }

                        // Update smoothed BPM
                        if self.smoothed_bpm == 0.0 {
                            self.smoothed_bpm = clamped_bpm;
                        } else {
                            let diff_ratio = (clamped_bpm - self.smoothed_bpm).abs() / self.smoothed_bpm;
                            if diff_ratio < 0.15 {
                                // Very close - update normally
                                self.smoothed_bpm = self.smoothed_bpm * 0.9 + clamped_bpm * 0.1;
                            } else if diff_ratio < 0.3 {
                                // Moderately close - update slowly
                                self.smoothed_bpm = self.smoothed_bpm * 0.95 + clamped_bpm * 0.05;
                            }
                            // Larger differences ignored
                        }

                        // Lock BPM when we have high confidence (consistent readings)
                        // This value persists through breaks
                        const CONFIDENCE_THRESHOLD: u32 = 8; // 8 consistent readings to lock
                        if self.bpm_confidence >= CONFIDENCE_THRESHOLD {
                            self.locked_bpm = self.smoothed_bpm;
                        }
                    }
                } else if time_since_last_beat > MAX_BEAT_INTERVAL {
                    // Reset beat tracking if too long since last beat
                    self.last_beat_time = self.frame_time;
                }
            }
        }

        self.prev_bass_energy = bass_energy;

        // Update dominant band (max once per second)
        const DOMINANT_UPDATE_INTERVAL: f32 = 1.0; // 1 second
        if self.frame_time - self.last_dominant_update_time >= DOMINANT_UPDATE_INTERVAL {
            // Find the band with the highest smoothed energy
            let mut max_band = 0;
            let mut max_energy = self.smoothed_bands[0];
            for i in 1..NUM_BANDS {
                if self.smoothed_bands[i] > max_energy {
                    max_energy = self.smoothed_bands[i];
                    max_band = i;
                }
            }
            self.dominant_band = max_band;
            self.last_dominant_update_time = self.frame_time;
        }

        // Detect drastic band changes with adaptive threshold
        const MAX_MARK_STEPS: u32 = 600; // 10 seconds at 60fps
        const MIN_DIVISOR: f32 = 1.0; // Minimum divisor to keep threshold activatable
        const MAX_DIVISOR: f32 = 60.0; // Maximum divisor (at 600 steps -> 600/60 = 10x easier)
        const BASE_THRESHOLD: f32 = 2.0; // Base threshold for detecting drastic change
        const MIN_THRESHOLD: f32 = 0.30; // Minimum threshold to ensure effort is required (stricter)

        // Increment last_mark (capped at MAX_MARK_STEPS)
        self.last_mark = (self.last_mark + 1).min(MAX_MARK_STEPS);

        // Calculate adaptive threshold: gets easier over time but stays above minimum
        let divisor = (self.last_mark as f32 / 10.0).clamp(MIN_DIVISOR, MAX_DIVISOR);
        let adaptive_threshold = (BASE_THRESHOLD / divisor).max(MIN_THRESHOLD);

        // Detect drastic change by comparing current smoothed bands to reference bands
        let mut max_band_change = 0.0f32;
        for i in 0..NUM_BANDS {
            let change = (self.smoothed_bands[i] - self.reference_bands[i]).abs();
            max_band_change = max_band_change.max(change);
        }

        // If drastic change detected, reset last_mark and update reference
        if max_band_change >= adaptive_threshold {
            self.last_mark = 1;
            self.reference_bands = self.smoothed_bands;
        }

        // Zoom direction shift only happens when last_mark is 1 (drastic change just occurred)
        let zoom_direction_shift = self.last_mark == 1;

        // Decrement viz_change cooldown
        if self.viz_change_cooldown > 0 {
            self.viz_change_cooldown -= 1;
        }

        // Visualization change triggers when zoom shift happens with high energy
        // Requires cooldown to have expired (prevents rapid re-triggering)
        const VIZ_CHANGE_ENERGY_THRESHOLD: f32 = 0.95;
        const VIZ_CHANGE_COOLDOWN_FRAMES: u32 = 180; // 3 seconds at 60fps
        let viz_change_triggered = zoom_direction_shift
            && self.smoothed_energy >= VIZ_CHANGE_ENERGY_THRESHOLD
            && self.viz_change_cooldown == 0;

        if viz_change_triggered {
            self.viz_change_cooldown = VIZ_CHANGE_COOLDOWN_FRAMES;
        }

        // New detection methods
        let (punch_detected, energy_floor, rise_rate) = self.detect_punch(self.smoothed_energy);
        let break_detected = self.detect_break(transition_detected, self.smoothed_energy);
        let bands_copy = self.smoothed_bands; // Copy to avoid borrow conflict
        let (instrument_added, instrument_removed, spectral_centroid) =
            self.detect_instrument_changes(&bands_copy);

        // Compute aggregate values
        let bass = (self.smoothed_bands[0] + self.smoothed_bands[1]) / 2.0;
        let mids = (self.smoothed_bands[2] + self.smoothed_bands[3] + self.smoothed_bands[4]) / 3.0;
        let treble =
            (self.smoothed_bands[5] + self.smoothed_bands[6] + self.smoothed_bands[7]) / 3.0;

        // Compute normalized bands relative to tracked min/max
        // This allows values outside [0, 1] when current is outside the tracked range
        let mut bands_normalized = [0.0f32; NUM_BANDS];
        for i in 0..NUM_BANDS {
            let range = self.band_maxs[i] - self.band_mins[i];
            if range > 0.01 {
                bands_normalized[i] = (self.smoothed_bands[i] - self.band_mins[i]) / range;
            } else {
                bands_normalized[i] = 0.0;
            }
        }

        self.last_analysis = AudioAnalysis {
            bands: self.smoothed_bands,
            spectrum: self.spectrum.clone(),
            spectrum_diff: self.spectrum_diff.clone(),
            bands_normalized,
            band_mins: self.band_mins,
            band_maxs: self.band_maxs,
            energy: self.smoothed_energy,
            transition_detected,
            bass,
            mids,
            treble,
            energy_diff,
            zoom_direction_shift,
            bpm: self.smoothed_bpm,
            dominant_band: self.dominant_band,
            last_mark: self.last_mark,
            viz_change_triggered,
            // New detection fields
            punch_detected,
            energy_floor,
            rise_rate,
            break_detected,
            instrument_added,
            instrument_removed,
            spectral_centroid,
        };

        self.last_analysis.clone()
    }

    fn detect_transition(&mut self, energy: f32, bands: &[f32; NUM_BANDS]) -> bool {
        // High frequency ratio
        let low_energy: f32 = bands[0..3].iter().sum();
        let high_energy: f32 = bands[5..8].iter().sum();
        let total = low_energy + high_energy;
        let freq_ratio = if total > 0.0 {
            high_energy / total
        } else {
            0.0
        };

        // Store in history
        let history_size = self.energy_history.len();
        self.energy_history[self.history_idx] = energy;
        self.freq_ratio_history[self.history_idx] = freq_ratio;
        self.history_idx = (self.history_idx + 1) % history_size;

        // Recent vs long-term averages (increased window for stability)
        let recent_frames = 60; // ~1 second at 60fps
        let recent_energy = self.recent_average(&self.energy_history, recent_frames);
        let recent_freq = self.recent_average(&self.freq_ratio_history, recent_frames);

        let long_energy: f32 = self.energy_history.iter().sum::<f32>() / history_size as f32;
        let long_freq: f32 = self.freq_ratio_history.iter().sum::<f32>() / history_size as f32;

        // Detect state transitions (lower thresholds = more sensitive)
        let is_high_energy = recent_energy > long_energy * 1.15;
        let is_high_freq = recent_freq > long_freq + 0.08;

        let threshold = 0.15;
        let energy_diff = (recent_energy - long_energy).abs();
        let freq_diff = (recent_freq - long_freq).abs();

        let norm_energy_diff = if long_energy > 0.01 {
            energy_diff / long_energy
        } else {
            energy_diff * 10.0
        };

        let energy_transition =
            is_high_energy != self.was_high_energy && norm_energy_diff > threshold;
        let freq_transition = is_high_freq != self.was_high_freq && freq_diff > threshold;

        self.was_high_energy = is_high_energy;
        self.was_high_freq = is_high_freq;

        energy_transition || freq_transition
    }

    fn recent_average(&self, history: &[f32], frames: usize) -> f32 {
        let history_size = history.len();
        let mut sum = 0.0;
        for i in 0..frames {
            let idx = (self.history_idx + history_size - 1 - i) % history_size;
            sum += history[idx];
        }
        sum / frames as f32
    }

    /// Detect punch (calm-before-spike): energy was low then suddenly spiked
    /// Returns (punch_detected, energy_floor, rise_rate)
    fn detect_punch(&mut self, current_energy: f32) -> (bool, f32, f32) {
        const FLOOR_DECAY: f32 = 0.998; // Very slowly drift floor up (~8 sec at 60fps)
        const FLOOR_ATTACK: f32 = 0.1;  // Moderately drop floor on new lows
        const FLOOR_SPIKE_ATTACK: f32 = 0.05; // Slow rise when energy spikes high
        const SPIKE_THRESHOLD: f32 = 0.4; // Energy above floor to trigger fast rise (stricter)

        // Get thresholds from config
        let floor_threshold = self.detection_config.punch_floor_threshold();
        let punch_threshold = self.detection_config.punch_spike_threshold();
        let min_rise_rate = self.detection_config.punch_rise_rate();
        let cooldown_frames = self.detection_config.punch_cooldown_frames();

        // Update energy floor (adaptive minimum tracking)
        let energy_gap = current_energy - self.energy_floor;
        if current_energy < self.energy_floor || self.energy_floor == 0.0 {
            // New low - quickly adopt it
            self.energy_floor =
                self.energy_floor * (1.0 - FLOOR_ATTACK) + current_energy * FLOOR_ATTACK;
        } else if energy_gap > SPIKE_THRESHOLD {
            // Energy spiked high - quickly raise floor to follow
            self.energy_floor =
                self.energy_floor * (1.0 - FLOOR_SPIKE_ATTACK) + current_energy * FLOOR_SPIKE_ATTACK;
        } else {
            // Slowly drift floor up toward current
            self.energy_floor =
                self.energy_floor * FLOOR_DECAY + current_energy * (1.0 - FLOOR_DECAY);
        }

        // Calculate rise rate (slope of energy change)
        let rise_rate = current_energy - self.lagged_energy;

        // Detect punch: floor was calm AND current energy spiked significantly
        let punch_detected = self.punch_cooldown == 0
            && self.energy_floor < floor_threshold
            && (current_energy - self.energy_floor) > punch_threshold
            && rise_rate > min_rise_rate;

        if punch_detected {
            self.punch_cooldown = cooldown_frames;
        }
        if self.punch_cooldown > 0 {
            self.punch_cooldown -= 1;
        }

        (punch_detected, self.energy_floor, rise_rate)
    }

    /// Detect break patterns: silence (no beats) for extended period
    /// Returns whether a break was detected
    fn detect_break(&mut self, is_beat: bool, _current_energy: f32) -> bool {
        // Get thresholds from config
        let silence_threshold = self.detection_config.break_silence_frames();
        let cooldown_threshold = self.detection_config.break_cooldown_frames();

        // Decrement cooldown
        if self.break_cooldown > 0 {
            self.break_cooldown -= 1;
        }

        // Track frames since last beat
        if is_beat {
            self.frames_since_beat = 0;
        } else {
            self.frames_since_beat += 1;
        }

        // Break detected when no beat for extended period and not in cooldown
        if self.frames_since_beat >= silence_threshold && self.break_cooldown == 0 {
            self.break_cooldown = cooldown_threshold;
            self.frames_since_beat = 0; // Reset to avoid immediate re-trigger
            return true;
        }

        false
    }

    /// Detect instrument changes via spectral complexity
    /// Returns (instrument_added, instrument_removed, spectral_centroid)
    fn detect_instrument_changes(&mut self, bands: &[f32; NUM_BANDS]) -> (bool, bool, f32) {
        const SMOOTHING: f32 = 0.95; // Slower smoothing for stability

        // Get thresholds from config
        let complexity_threshold = self.detection_config.complexity_threshold();
        let change_ratio = self.detection_config.complexity_change_ratio();

        // Calculate spectral complexity (weighted count of active bands)
        let mut active_weight = 0.0f32;
        let mut total_energy = 0.0f32;
        let mut weighted_freq_sum = 0.0f32;

        for (i, &band_energy) in bands.iter().enumerate() {
            if band_energy > complexity_threshold {
                active_weight += band_energy; // Weight by energy, not just count
            }
            total_energy += band_energy;
            // Spectral centroid: weighted average frequency
            let band_center_freq = (BAND_EDGES[i] + BAND_EDGES[i + 1]) / 2.0;
            weighted_freq_sum += band_center_freq * band_energy;
        }

        let spectral_centroid = if total_energy > 0.01 {
            weighted_freq_sum / total_energy
        } else {
            1000.0 // Default to mid frequency
        };

        // Smooth complexity
        let new_complexity = active_weight;
        self.spectral_complexity =
            self.spectral_complexity * SMOOTHING + new_complexity * (1.0 - SMOOTHING);

        // Detect changes
        let complexity_ratio = if self.prev_spectral_complexity > 0.1 {
            self.spectral_complexity / self.prev_spectral_complexity
        } else {
            1.0
        };

        let instrument_added = complexity_ratio > change_ratio;
        let instrument_removed = complexity_ratio < 1.0 / change_ratio;

        self.prev_spectral_complexity = self.spectral_complexity;

        (instrument_added, instrument_removed, spectral_centroid)
    }
}
