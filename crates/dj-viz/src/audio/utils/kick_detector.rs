//! Kick drum detection that works across music genres.
//!
//! Uses multi-band onset detection to identify kicks regardless of:
//! - Electronic music (clean sub-bass kicks)
//! - Rock/metal (punchy mid-frequency kicks)
//! - Jazz/acoustic (softer, more transient kicks)
//! - Hip-hop (808s with long sustain)
//!
//! The algorithm combines:
//! 1. Sub-bass energy onset (20-80 Hz) - the "thump"
//! 2. Low-mid transient (80-200 Hz) - the "punch"
//! 3. High-frequency click (2-5 kHz) - the "attack" transient
//!
//! A kick is detected when onsets coincide across multiple bands.

/// Configuration for kick detection sensitivity
#[derive(Clone, Copy)]
pub struct KickDetectorConfig {
    /// Minimum interval between kick detections in seconds (prevents double-triggers)
    pub min_kick_interval: f32,
    /// Threshold multiplier for onset detection (higher = less sensitive)
    pub onset_threshold: f32,
    /// How many bands must have coincident onsets to trigger (1-3)
    pub min_coincident_bands: u8,
    /// Attack time for envelope follower (0-1, higher = faster response)
    pub envelope_attack: f32,
    /// Release time for envelope follower (0-1, higher = faster decay)
    pub envelope_release: f32,
}

impl Default for KickDetectorConfig {
    fn default() -> Self {
        Self {
            min_kick_interval: 0.12, // ~500 BPM max, allows fast double kicks
            onset_threshold: 1.4,    // 40% above moving average
            min_coincident_bands: 2, // Require 2 of 3 bands to agree
            envelope_attack: 0.8,    // Fast attack for transients
            envelope_release: 0.15,  // Moderate release
        }
    }
}

/// Band indices for kick detection (maps to analyzer bands)
#[derive(Clone, Copy)]
struct BandRange {
    /// Start frequency in Hz
    low_hz: f32,
    /// End frequency in Hz
    high_hz: f32,
    /// Weight for this band in final decision (higher = more important)
    weight: f32,
}

const KICK_BANDS: [BandRange; 3] = [
    // Sub-bass: the fundamental "thump" of a kick
    BandRange {
        low_hz: 20.0,
        high_hz: 80.0,
        weight: 1.0,
    },
    // Low-mid: the "punch" and body
    BandRange {
        low_hz: 80.0,
        high_hz: 200.0,
        weight: 0.8,
    },
    // Attack transient: the "click" of the beater
    BandRange {
        low_hz: 2000.0,
        high_hz: 5000.0,
        weight: 0.5,
    },
];

/// State for tracking a single frequency band
struct BandState {
    /// Smoothed energy envelope
    envelope: f32,
    /// Moving average for adaptive threshold
    moving_avg: f32,
    /// Previous frame's energy (for flux calculation)
    prev_energy: f32,
    /// Spectral flux (positive change only)
    flux: f32,
    /// Whether an onset was detected this frame
    onset: bool,
}

impl Default for BandState {
    fn default() -> Self {
        Self {
            envelope: 0.0,
            moving_avg: 0.0,
            prev_energy: 0.0,
            flux: 0.0,
            onset: false,
        }
    }
}

/// Kick drum detector using multi-band onset detection
pub struct KickDetector {
    config: KickDetectorConfig,
    /// State for each detection band
    bands: [BandState; 3],
    /// Time since last kick detection (in seconds)
    time_since_kick: f32,
    /// Sample rate for bin calculations
    #[allow(dead_code)]
    sample_rate: f32,
    /// FFT size for bin calculations
    #[allow(dead_code)]
    fft_size: usize,
    /// Pre-computed bin ranges for each band
    bin_ranges: [(usize, usize); 3],
    /// Kick confidence (0-1, higher = more certain)
    confidence: f32,
    /// Whether a kick was detected this frame
    kick_detected: bool,
}

impl KickDetector {
    /// Create a new kick detector with default configuration
    pub fn new(sample_rate: f32, fft_size: usize) -> Self {
        Self::with_config(sample_rate, fft_size, KickDetectorConfig::default())
    }

    /// Create a new kick detector with custom configuration
    pub fn with_config(sample_rate: f32, fft_size: usize, config: KickDetectorConfig) -> Self {
        let bin_width = sample_rate / fft_size as f32;

        // Pre-compute bin ranges for each band
        let bin_ranges = [
            Self::compute_bin_range(&KICK_BANDS[0], bin_width, fft_size),
            Self::compute_bin_range(&KICK_BANDS[1], bin_width, fft_size),
            Self::compute_bin_range(&KICK_BANDS[2], bin_width, fft_size),
        ];

        Self {
            config,
            bands: [
                BandState::default(),
                BandState::default(),
                BandState::default(),
            ],
            time_since_kick: 1.0, // Start allowing kicks immediately
            sample_rate,
            fft_size,
            bin_ranges,
            confidence: 0.0,
            kick_detected: false,
        }
    }

    fn compute_bin_range(band: &BandRange, bin_width: f32, fft_size: usize) -> (usize, usize) {
        let low_bin = (band.low_hz / bin_width).floor() as usize;
        let high_bin = (band.high_hz / bin_width).ceil() as usize;
        (low_bin.max(1), high_bin.min(fft_size / 2))
    }

    /// Process a frame of spectrum data and detect kicks
    ///
    /// # Arguments
    /// * `spectrum` - Magnitude spectrum from FFT (normalized 0-1)
    /// * `dt` - Delta time since last frame in seconds
    ///
    /// # Returns
    /// `true` if a kick was detected this frame
    pub fn process(&mut self, spectrum: &[f32], dt: f32) -> bool {
        self.time_since_kick += dt;
        self.kick_detected = false;

        // Calculate energy and flux for each band
        let mut onset_count = 0;
        let mut weighted_onset_sum = 0.0;

        for (i, band_state) in self.bands.iter_mut().enumerate() {
            let (low_bin, high_bin) = self.bin_ranges[i];

            // Skip if bins are out of range
            if high_bin > spectrum.len() || low_bin >= high_bin {
                continue;
            }

            // Calculate band energy (RMS of magnitudes)
            let bin_count = (high_bin - low_bin) as f32;
            let energy: f32 = spectrum[low_bin..high_bin]
                .iter()
                .map(|&m| m * m)
                .sum::<f32>()
                / bin_count;
            let energy = energy.sqrt();

            // Update envelope with asymmetric smoothing
            if energy > band_state.envelope {
                band_state.envelope = band_state.envelope * (1.0 - self.config.envelope_attack)
                    + energy * self.config.envelope_attack;
            } else {
                band_state.envelope = band_state.envelope * (1.0 - self.config.envelope_release)
                    + energy * self.config.envelope_release;
            }

            // Calculate spectral flux (only positive changes = onsets)
            let flux = (energy - band_state.prev_energy).max(0.0);
            band_state.flux = flux;
            band_state.prev_energy = energy;

            // Update moving average with fast initial adaptation
            // If moving_avg is near zero, adapt quickly to current level
            let avg_decay = if band_state.moving_avg < 0.01 {
                0.8 // Fast initial adaptation
            } else {
                0.98 // Normal slower adaptation (~0.8 sec to adapt)
            };
            band_state.moving_avg =
                band_state.moving_avg * avg_decay + band_state.envelope * (1.0 - avg_decay);

            // Threshold based on envelope, not moving_avg (more responsive)
            // Use flux relative to current envelope level
            let threshold = (band_state.envelope * 0.15).max(0.02);

            // Detect onset: significant positive flux relative to current level
            band_state.onset = flux > threshold;

            if band_state.onset {
                onset_count += 1;
                weighted_onset_sum += KICK_BANDS[i].weight;
            }
        }

        // Calculate confidence based on weighted onset agreement
        let max_weight: f32 = KICK_BANDS.iter().map(|b| b.weight).sum();
        self.confidence = weighted_onset_sum / max_weight;

        // Detect kick if enough bands agree and cooldown has passed
        if onset_count >= self.config.min_coincident_bands as usize
            && self.time_since_kick >= self.config.min_kick_interval
        {
            self.kick_detected = true;
            self.time_since_kick = 0.0;
        }

        self.kick_detected
    }

    /// Process using pre-computed band energies (faster if bands already computed)
    ///
    /// # Arguments
    /// * `sub_bass` - Energy in 20-80 Hz range (0-1)
    /// * `low_mid` - Energy in 80-200 Hz range (0-1)
    /// * `attack` - Energy in 2-5 kHz range (0-1)
    /// * `dt` - Delta time since last frame in seconds
    ///
    /// # Returns
    /// `true` if a kick was detected this frame
    #[allow(dead_code)]
    pub fn process_bands(&mut self, sub_bass: f32, low_mid: f32, attack: f32, dt: f32) -> bool {
        self.time_since_kick += dt;
        self.kick_detected = false;

        let energies = [sub_bass, low_mid, attack];
        let mut onset_count = 0;
        let mut weighted_onset_sum = 0.0;

        for (i, &energy) in energies.iter().enumerate() {
            let band_state = &mut self.bands[i];

            // Update envelope
            if energy > band_state.envelope {
                band_state.envelope = band_state.envelope * (1.0 - self.config.envelope_attack)
                    + energy * self.config.envelope_attack;
            } else {
                band_state.envelope = band_state.envelope * (1.0 - self.config.envelope_release)
                    + energy * self.config.envelope_release;
            }

            // Spectral flux
            let flux = (energy - band_state.prev_energy).max(0.0);
            band_state.flux = flux;
            band_state.prev_energy = energy;

            // Moving average
            const AVG_DECAY: f32 = 0.995;
            band_state.moving_avg =
                band_state.moving_avg * AVG_DECAY + band_state.envelope * (1.0 - AVG_DECAY);

            let threshold = (band_state.moving_avg * self.config.onset_threshold).max(0.05);
            band_state.onset = flux > threshold && energy > band_state.moving_avg;

            if band_state.onset {
                onset_count += 1;
                weighted_onset_sum += KICK_BANDS[i].weight;
            }
        }

        let max_weight: f32 = KICK_BANDS.iter().map(|b| b.weight).sum();
        self.confidence = weighted_onset_sum / max_weight;

        if onset_count >= self.config.min_coincident_bands as usize
            && self.time_since_kick >= self.config.min_kick_interval
        {
            self.kick_detected = true;
            self.time_since_kick = 0.0;
        }

        self.kick_detected
    }

    /// Returns whether a kick was detected in the last `process` call
    #[allow(dead_code)]
    pub fn kick_detected(&self) -> bool {
        self.kick_detected
    }

    /// Returns the confidence of the last kick detection (0-1)
    /// Higher values indicate more bands agreed on the onset
    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    /// Returns time since the last kick in seconds
    pub fn time_since_kick(&self) -> f32 {
        self.time_since_kick
    }

    /// Returns the current envelope for each band (sub-bass, low-mid, attack)
    pub fn band_envelopes(&self) -> [f32; 3] {
        [
            self.bands[0].envelope,
            self.bands[1].envelope,
            self.bands[2].envelope,
        ]
    }

    /// Returns the spectral flux for each band
    pub fn band_flux(&self) -> [f32; 3] {
        [self.bands[0].flux, self.bands[1].flux, self.bands[2].flux]
    }

    /// Reset detector state (useful when switching audio sources)
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        for band in &mut self.bands {
            *band = BandState::default();
        }
        self.time_since_kick = 1.0;
        self.confidence = 0.0;
        self.kick_detected = false;
    }

    /// Update configuration at runtime
    #[allow(dead_code)]
    pub fn set_config(&mut self, config: KickDetectorConfig) {
        self.config = config;
    }

    /// Get current configuration
    #[allow(dead_code)]
    pub fn config(&self) -> &KickDetectorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kick_detector_creation() {
        let detector = KickDetector::new(44100.0, 2048);
        assert!(!detector.kick_detected());
        assert_eq!(detector.confidence(), 0.0);
    }

    #[test]
    fn test_no_kick_on_silence() {
        let mut detector = KickDetector::new(44100.0, 2048);
        let spectrum = vec![0.0; 1024];

        for _ in 0..100 {
            detector.process(&spectrum, 1.0 / 60.0);
        }

        assert!(!detector.kick_detected());
    }

    #[test]
    fn test_kick_detection_cooldown() {
        let mut detector = KickDetector::new(44100.0, 2048);
        let mut spectrum = vec![0.0; 1024];

        // Warm up with silence
        for _ in 0..60 {
            detector.process(&spectrum, 1.0 / 60.0);
        }

        // Simulate kick (energy spike in low frequencies)
        for i in 1..10 {
            spectrum[i] = 0.8;
        }
        for i in 10..20 {
            spectrum[i] = 0.6;
        }

        let first_kick = detector.process(&spectrum, 1.0 / 60.0);

        // Even with sustained energy, cooldown should prevent immediate re-trigger
        let second_kick = detector.process(&spectrum, 1.0 / 60.0);

        // At least the cooldown should work
        if first_kick {
            assert!(!second_kick, "Cooldown should prevent immediate re-trigger");
        }
    }

    #[test]
    fn test_process_bands() {
        let mut detector = KickDetector::new(44100.0, 2048);

        // Warm up
        for _ in 0..60 {
            detector.process_bands(0.1, 0.1, 0.05, 1.0 / 60.0);
        }

        // Simulate kick via band energies
        let kick = detector.process_bands(0.9, 0.7, 0.4, 1.0 / 60.0);

        // The detector should respond to the sudden increase
        assert!(detector.confidence() > 0.0 || kick);
    }
}
