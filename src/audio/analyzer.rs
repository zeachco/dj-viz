//! Audio analysis and FFT processing.
//!
//! Performs real-time FFT analysis on audio samples to extract frequency band energies,
//! detect beats/transitions, and compute aggregate metrics (bass, mids, treble).

use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

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

/// Pre-computed analysis results - no allocations needed by visualizations
#[derive(Clone)]
pub struct AudioAnalysis {
    /// Energy in each frequency band (0-1, smoothed)
    pub bands: [f32; NUM_BANDS],
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
}

impl Default for AudioAnalysis {
    fn default() -> Self {
        Self {
            bands: [0.0; NUM_BANDS],
            energy: 0.0,
            transition_detected: false,
            bass: 0.0,
            mids: 0.0,
            treble: 0.0,
            energy_diff: 0.0,
            zoom_direction_shift: false,
            bpm: 0.0,
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
    beat_times: Vec<f32>, // Timestamps of recent beats (in seconds)
    last_beat_time: f32,  // Last detected beat time
    smoothed_bpm: f32,    // Smoothed BPM estimate
    frame_time: f32,      // Accumulated time for timestamping

    // Frame skipping for performance
    frame_count: u32,
    last_analysis: AudioAnalysis,
}

impl AudioAnalyzer {
    pub fn new() -> Self {
        Self::with_sample_rate(44100.0)
    }

    pub fn with_sample_rate(sample_rate: f32) -> Self {
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

        const HISTORY_SIZE: usize = 180; // ~3 seconds at 60fps
        const BPM_HISTORY_SIZE: usize = 8; // Track last 8 beats for BPM calculation

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
            frame_time: 0.0,
            frame_count: 0,
            last_analysis: AudioAnalysis::default(),
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
                const MIN_DRIFT: f32 = 0.9995; // Very slow drift towards current
                const MAX_DRIFT: f32 = 0.9995;

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

        // Detect zoom direction shift when energy_diff crosses ±0.15 threshold
        const ZOOM_THRESHOLD: f32 = 0.15;
        let zoom_direction_shift = (self.prev_energy_diff < ZOOM_THRESHOLD
            && energy_diff >= ZOOM_THRESHOLD)
            || (self.prev_energy_diff > -ZOOM_THRESHOLD && energy_diff <= -ZOOM_THRESHOLD)
            || (self.prev_energy_diff >= ZOOM_THRESHOLD && energy_diff < ZOOM_THRESHOLD)
            || (self.prev_energy_diff <= -ZOOM_THRESHOLD && energy_diff > -ZOOM_THRESHOLD);

        self.prev_energy_diff = energy_diff;
        self.prev_bands = bands_raw;

        // Transition detection
        let transition_detected = self.detect_transition(energy_raw, &bands_raw);

        // BPM detection - update when we detect a transition
        if transition_detected {
            let time_since_last_beat = self.frame_time - self.last_beat_time;

            // Only count as a beat if enough time has passed (avoid double-counting)
            // Minimum interval: 60bpm = 1 beat per second, so min 0.3s to allow up to 200bpm
            const MIN_BEAT_INTERVAL: f32 = 0.3;
            if time_since_last_beat >= MIN_BEAT_INTERVAL {
                self.beat_times.push(self.frame_time);
                self.last_beat_time = self.frame_time;

                // Keep only last 8 beats
                const MAX_BEAT_HISTORY: usize = 8;
                if self.beat_times.len() > MAX_BEAT_HISTORY {
                    self.beat_times.remove(0);
                }

                // Calculate BPM from intervals between beats
                if self.beat_times.len() >= 2 {
                    let mut intervals = Vec::new();
                    for i in 1..self.beat_times.len() {
                        intervals.push(self.beat_times[i] - self.beat_times[i - 1]);
                    }

                    // Average interval in seconds
                    let avg_interval: f32 = intervals.iter().sum::<f32>() / intervals.len() as f32;

                    // Convert to BPM (beats per minute)
                    let instant_bpm = 60.0 / avg_interval;

                    // Smooth the BPM (slow adjustment to avoid jitter)
                    if self.smoothed_bpm == 0.0 {
                        self.smoothed_bpm = instant_bpm;
                    } else {
                        self.smoothed_bpm = self.smoothed_bpm * 0.85 + instant_bpm * 0.15;
                    }
                }
            }
        }

        // Compute aggregate values
        let bass = (self.smoothed_bands[0] + self.smoothed_bands[1]) / 2.0;
        let mids = (self.smoothed_bands[2] + self.smoothed_bands[3] + self.smoothed_bands[4]) / 3.0;
        let treble =
            (self.smoothed_bands[5] + self.smoothed_bands[6] + self.smoothed_bands[7]) / 3.0;

        self.last_analysis = AudioAnalysis {
            bands: self.smoothed_bands,
            energy: self.smoothed_energy,
            transition_detected,
            bass,
            mids,
            treble,
            energy_diff,
            zoom_direction_shift,
            bpm: self.smoothed_bpm,
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

        // Recent vs long-term averages
        let recent_frames = 30;
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
}
