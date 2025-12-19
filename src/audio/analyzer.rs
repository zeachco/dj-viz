use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// Number of frequency bands for visualization
pub const NUM_BANDS: usize = 8;

/// FFT size - smaller than buffer for better performance
const FFT_SIZE: usize = 512;

/// Frequency band boundaries (Hz) for 44.1kHz sample rate
/// Sub-bass, Bass, Low-mid, Mid, Upper-mid, Presence, Brilliance, Air
const BAND_EDGES: [f32; NUM_BANDS + 1] = [20.0, 60.0, 250.0, 500.0, 2000.0, 4000.0, 6000.0, 12000.0, 20000.0];

/// Pre-computed analysis results - no allocations needed by visualizations
#[derive(Clone)]
pub struct AudioAnalysis {
    /// Energy in each frequency band (0-1, smoothed)
    pub bands: [f32; NUM_BANDS],
    /// Raw band values before smoothing (for reactive effects)
    pub bands_raw: [f32; NUM_BANDS],
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
    /// Peak detected this frame (any band spiked)
    pub peak: bool,
}

impl Default for AudioAnalysis {
    fn default() -> Self {
        Self {
            bands: [0.0; NUM_BANDS],
            bands_raw: [0.0; NUM_BANDS],
            energy: 0.0,
            transition_detected: false,
            bass: 0.0,
            mids: 0.0,
            treble: 0.0,
            peak: false,
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

    // Transition detection state
    energy_history: Vec<f32>,
    freq_ratio_history: Vec<f32>,
    history_idx: usize,
    was_high_energy: bool,
    was_high_freq: bool,

    // Peak detection
    prev_bands: [f32; NUM_BANDS],

    // Frame skipping for performance
    frame_count: u32,
    last_analysis: AudioAnalysis,

    // Sample rate for bin calculations
    sample_rate: f32,
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
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos())
            })
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

        Self {
            fft,
            fft_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            fft_window,
            band_bins,
            smoothed_bands: [0.0; NUM_BANDS],
            smoothed_energy: 0.0,
            energy_history: vec![0.0; HISTORY_SIZE],
            freq_ratio_history: vec![0.0; HISTORY_SIZE],
            history_idx: 0,
            was_high_energy: false,
            was_high_freq: false,
            prev_bands: [0.0; NUM_BANDS],
            frame_count: 0,
            last_analysis: AudioAnalysis::default(),
            sample_rate,
        }
    }

    /// Analyze audio samples. Call once per frame.
    /// Returns cached result if called multiple times per frame.
    pub fn analyze(&mut self, samples: &[f32]) -> AudioAnalysis {
        self.frame_count = self.frame_count.wrapping_add(1);

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
                let db = 10.0 * (avg_energy + 1e-10).log10();
                // Map roughly -60dB to 0dB -> 0 to 1
                bands_raw[i] = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            }
        }

        // Smooth bands (fast attack, slower decay)
        let attack = 0.7;
        let decay = 0.15;
        for i in 0..NUM_BANDS {
            if bands_raw[i] > self.smoothed_bands[i] {
                self.smoothed_bands[i] = self.smoothed_bands[i] * (1.0 - attack) + bands_raw[i] * attack;
            } else {
                self.smoothed_bands[i] = self.smoothed_bands[i] * (1.0 - decay) + bands_raw[i] * decay;
            }
        }

        // Calculate overall energy
        let energy_raw: f32 = bands_raw.iter().sum::<f32>() / NUM_BANDS as f32;
        if energy_raw > self.smoothed_energy {
            self.smoothed_energy = self.smoothed_energy * 0.3 + energy_raw * 0.7;
        } else {
            self.smoothed_energy = self.smoothed_energy * 0.9 + energy_raw * 0.1;
        }

        // Detect peaks (any band jumped significantly)
        let peak = bands_raw.iter().zip(self.prev_bands.iter())
            .any(|(curr, prev)| *curr > *prev + 0.15 && *curr > 0.3);
        self.prev_bands = bands_raw;

        // Transition detection
        let transition_detected = self.detect_transition(energy_raw, &bands_raw);

        // Compute aggregate values
        let bass = (self.smoothed_bands[0] + self.smoothed_bands[1]) / 2.0;
        let mids = (self.smoothed_bands[2] + self.smoothed_bands[3] + self.smoothed_bands[4]) / 3.0;
        let treble = (self.smoothed_bands[5] + self.smoothed_bands[6] + self.smoothed_bands[7]) / 3.0;

        self.last_analysis = AudioAnalysis {
            bands: self.smoothed_bands,
            bands_raw,
            energy: self.smoothed_energy,
            transition_detected,
            bass,
            mids,
            treble,
            peak,
        };

        self.last_analysis.clone()
    }

    fn detect_transition(&mut self, energy: f32, bands: &[f32; NUM_BANDS]) -> bool {
        // High frequency ratio
        let low_energy: f32 = bands[0..3].iter().sum();
        let high_energy: f32 = bands[5..8].iter().sum();
        let total = low_energy + high_energy;
        let freq_ratio = if total > 0.0 { high_energy / total } else { 0.0 };

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

        let norm_energy_diff = if long_energy > 0.01 { energy_diff / long_energy } else { energy_diff * 10.0 };

        let energy_transition = is_high_energy != self.was_high_energy && norm_energy_diff > threshold;
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
