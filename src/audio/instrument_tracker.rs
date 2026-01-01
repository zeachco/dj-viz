//! Instrument detection and tracking via spectral pattern analysis.
//!
//! Detects instruments by analyzing frequency patterns over time. Once a pattern
//! is established, it's tracked as a distinct instrument. Instruments that aren't
//! active for a period decay and get replaced by new detections.

use super::analyzer::{NUM_BANDS, SPECTRUM_SIZE};

/// Maximum number of instruments to track simultaneously
pub const MAX_INSTRUMENTS: usize = 6;

/// Minimum frames to establish a pattern (quick probing phase)
const PROBE_FRAMES: usize = 8;

/// Frames to fully establish an instrument pattern
const ESTABLISH_FRAMES: usize = 30;

/// Frames of inactivity before decay starts
const DECAY_START_FRAMES: u32 = 90; // ~1.5 seconds at 60fps

/// Rate at which inactive instruments decay (per frame)
const DECAY_RATE: f32 = 0.02;

/// Minimum confidence to keep tracking an instrument
const MIN_CONFIDENCE: f32 = 0.1;

/// Correlation threshold to match an existing instrument
const MATCH_THRESHOLD: f32 = 0.7;

/// Minimum energy to consider a frequency region active
const ENERGY_THRESHOLD: f32 = 0.15;

/// Detected instrument with spectral signature and tracking state
#[derive(Clone, Debug)]
pub struct DetectedInstrument {
    /// Unique ID for this instrument slot
    pub id: usize,
    /// Spectral signature - relative energy distribution across bands
    pub signature: [f32; NUM_BANDS],
    /// Fine-grained spectral signature (subset of full spectrum for key regions)
    pub fine_signature: Vec<f32>,
    /// Center frequency in Hz (weighted average of active frequencies)
    pub center_freq: f32,
    /// Frequency range (low, high) in Hz
    pub freq_range: (f32, f32),
    /// Dominant band index (0-7)
    pub dominant_band: usize,
    /// Confidence level (0-1), increases with consistent matches, decreases with decay
    pub confidence: f32,
    /// Current energy level (0-1)
    pub energy: f32,
    /// Frames since last activity
    pub inactive_frames: u32,
    /// Whether pattern is fully established (past probing phase)
    pub established: bool,
    /// Frames of pattern history collected
    pub pattern_frames: usize,
    /// Rolling pattern history for refinement
    pattern_history: Vec<[f32; NUM_BANDS]>,
}

impl Default for DetectedInstrument {
    fn default() -> Self {
        Self {
            id: 0,
            signature: [0.0; NUM_BANDS],
            fine_signature: Vec::new(),
            center_freq: 0.0,
            freq_range: (0.0, 0.0),
            dominant_band: 0,
            confidence: 0.0,
            energy: 0.0,
            inactive_frames: 0,
            established: false,
            pattern_frames: 0,
            pattern_history: Vec::with_capacity(ESTABLISH_FRAMES),
        }
    }
}

impl DetectedInstrument {
    /// Check if this instrument slot is active (has a tracked pattern)
    pub fn is_active(&self) -> bool {
        self.confidence > MIN_CONFIDENCE && self.pattern_frames >= PROBE_FRAMES
    }

    /// Update signature from pattern history (averaging collected samples)
    fn refine_signature(&mut self) {
        if self.pattern_history.is_empty() {
            return;
        }

        // Average all patterns in history
        let mut avg = [0.0f32; NUM_BANDS];
        for pattern in &self.pattern_history {
            for (i, &val) in pattern.iter().enumerate() {
                avg[i] += val;
            }
        }
        let count = self.pattern_history.len() as f32;
        for val in &mut avg {
            *val /= count;
        }

        // Normalize to relative strengths (sum to 1)
        let total: f32 = avg.iter().sum();
        if total > 0.01 {
            for val in &mut avg {
                *val /= total;
            }
        }

        self.signature = avg;
    }

    /// Calculate correlation with a given band pattern
    fn correlate(&self, bands: &[f32; NUM_BANDS]) -> f32 {
        // Normalize input to relative strengths
        let total: f32 = bands.iter().sum();
        if total < 0.01 {
            return 0.0;
        }

        let normalized: Vec<f32> = bands.iter().map(|&b| b / total).collect();

        // Pearson correlation coefficient
        let mean_sig: f32 = self.signature.iter().sum::<f32>() / NUM_BANDS as f32;
        let mean_input: f32 = normalized.iter().sum::<f32>() / NUM_BANDS as f32;

        let mut cov = 0.0f32;
        let mut var_sig = 0.0f32;
        let mut var_input = 0.0f32;

        for i in 0..NUM_BANDS {
            let diff_sig = self.signature[i] - mean_sig;
            let diff_input = normalized[i] - mean_input;
            cov += diff_sig * diff_input;
            var_sig += diff_sig * diff_sig;
            var_input += diff_input * diff_input;
        }

        let denom = (var_sig * var_input).sqrt();
        if denom > 0.001 {
            (cov / denom).clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Frequency band edges for reference
const BAND_EDGES: [f32; NUM_BANDS + 1] = [
    20.0, 60.0, 250.0, 500.0, 2000.0, 4000.0, 6000.0, 12000.0, 20000.0,
];

/// Spectral peak detected during analysis
#[derive(Clone, Debug)]
struct SpectralPeak {
    /// Band energies at this peak
    bands: [f32; NUM_BANDS],
    /// Center frequency (Hz)
    center_freq: f32,
    /// Frequency range (low, high) in Hz
    freq_range: (f32, f32),
    /// Dominant band index
    dominant_band: usize,
    /// Peak energy level
    energy: f32,
}

/// Tracks multiple instruments via spectral pattern analysis
pub struct InstrumentTracker {
    /// Currently tracked instruments
    instruments: [DetectedInstrument; MAX_INSTRUMENTS],
    /// Next instrument ID to assign
    next_id: usize,
    /// Frame counter for timing
    frame_count: u32,
    /// Previous frame's bands for change detection
    prev_bands: [f32; NUM_BANDS],
    /// Sample rate for frequency calculations
    sample_rate: f32,
}

impl InstrumentTracker {
    pub fn new(sample_rate: f32) -> Self {
        let instruments = std::array::from_fn(|i| {
            let mut inst = DetectedInstrument::default();
            inst.id = i;
            inst
        });

        Self {
            instruments,
            next_id: MAX_INSTRUMENTS,
            frame_count: 0,
            prev_bands: [0.0; NUM_BANDS],
            sample_rate,
        }
    }

    /// Analyze current audio frame and update instrument tracking.
    /// Returns list of currently active instruments.
    pub fn update(
        &mut self,
        bands: &[f32; NUM_BANDS],
        spectrum: &[f32],
    ) -> Vec<DetectedInstrument> {
        self.frame_count += 1;

        // Detect spectral peaks in current frame
        let peaks = self.detect_peaks(bands, spectrum);

        // Try to match peaks to existing instruments
        let mut matched_indices = Vec::new();

        for peak in &peaks {
            if let Some(idx) = self.match_to_existing(&peak.bands) {
                // Update existing instrument
                self.update_instrument(idx, peak);
                matched_indices.push(idx);
            } else {
                // Try to start tracking new instrument
                if let Some(idx) = self.find_empty_or_weakest_slot() {
                    if !matched_indices.contains(&idx) {
                        self.start_new_instrument(idx, peak);
                        matched_indices.push(idx);
                    }
                }
            }
        }

        // Decay unmatched instruments
        for i in 0..MAX_INSTRUMENTS {
            if !matched_indices.contains(&i) {
                self.decay_instrument(i);
            }
        }

        self.prev_bands = *bands;

        // Return active instruments sorted by energy
        let mut active: Vec<DetectedInstrument> = self
            .instruments
            .iter()
            .filter(|inst| inst.is_active())
            .cloned()
            .collect();
        active.sort_by(|a, b| {
            b.energy
                .partial_cmp(&a.energy)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        active
    }

    /// Get all currently tracked instruments (including inactive)
    pub fn all_instruments(&self) -> &[DetectedInstrument; MAX_INSTRUMENTS] {
        &self.instruments
    }

    /// Detect spectral peaks from band energies
    fn detect_peaks(&self, bands: &[f32; NUM_BANDS], spectrum: &[f32]) -> Vec<SpectralPeak> {
        let mut peaks = Vec::new();

        // Find dominant bands with significant energy
        let total_energy: f32 = bands.iter().sum();
        if total_energy < 0.1 {
            return peaks;
        }

        // Look for distinct frequency regions
        let mut i = 0;
        while i < NUM_BANDS {
            if bands[i] > ENERGY_THRESHOLD {
                // Found start of an active region
                let region_start = i;
                let mut region_energy = 0.0f32;
                let mut dominant_band = i;
                let mut max_energy = bands[i];

                // Extend region while energy stays significant
                while i < NUM_BANDS && bands[i] > ENERGY_THRESHOLD * 0.5 {
                    region_energy += bands[i];
                    if bands[i] > max_energy {
                        max_energy = bands[i];
                        dominant_band = i;
                    }
                    i += 1;
                }
                let region_end = i;

                // Create peak from this region
                let mut peak_bands = [0.0f32; NUM_BANDS];
                for j in region_start..region_end {
                    peak_bands[j] = bands[j];
                }

                // Calculate center frequency using spectrum
                let center_freq = self.calculate_center_freq(dominant_band, &peak_bands, spectrum);
                let freq_range = (
                    BAND_EDGES[region_start],
                    BAND_EDGES[region_end.min(NUM_BANDS)],
                );

                peaks.push(SpectralPeak {
                    bands: peak_bands,
                    center_freq,
                    freq_range,
                    dominant_band,
                    energy: region_energy / (region_end - region_start) as f32,
                });
            } else {
                i += 1;
            }
        }

        peaks
    }

    /// Calculate center frequency using fine spectrum data
    fn calculate_center_freq(
        &self,
        dominant_band: usize,
        bands: &[f32; NUM_BANDS],
        spectrum: &[f32],
    ) -> f32 {
        // Use spectrum bins within the dominant band for precise frequency
        let bin_width = self.sample_rate / (SPECTRUM_SIZE as f32 * 2.0);
        let low_freq = BAND_EDGES[dominant_band];
        let high_freq = BAND_EDGES[(dominant_band + 1).min(NUM_BANDS)];

        let low_bin = (low_freq / bin_width).floor() as usize;
        let high_bin = ((high_freq / bin_width).ceil() as usize).min(spectrum.len());

        if high_bin <= low_bin {
            return (low_freq + high_freq) / 2.0;
        }

        // Weighted average frequency
        let mut weighted_sum = 0.0f32;
        let mut total_weight = 0.0f32;

        for bin in low_bin..high_bin {
            if bin < spectrum.len() {
                let freq = bin as f32 * bin_width;
                let weight = spectrum[bin];
                weighted_sum += freq * weight;
                total_weight += weight;
            }
        }

        if total_weight > 0.01 {
            weighted_sum / total_weight
        } else {
            // Fallback to band-based calculation
            let mut band_weighted = 0.0f32;
            let mut band_total = 0.0f32;
            for i in 0..NUM_BANDS {
                let center = (BAND_EDGES[i] + BAND_EDGES[i + 1]) / 2.0;
                band_weighted += center * bands[i];
                band_total += bands[i];
            }
            if band_total > 0.01 {
                band_weighted / band_total
            } else {
                1000.0
            }
        }
    }

    /// Try to match a peak to an existing tracked instrument
    fn match_to_existing(&self, bands: &[f32; NUM_BANDS]) -> Option<usize> {
        let mut best_match: Option<(usize, f32)> = None;

        for (i, inst) in self.instruments.iter().enumerate() {
            if inst.pattern_frames < PROBE_FRAMES {
                continue; // Skip instruments still in probe phase
            }

            let correlation = inst.correlate(bands);
            if correlation > MATCH_THRESHOLD {
                if best_match.is_none() || correlation > best_match.unwrap().1 {
                    best_match = Some((i, correlation));
                }
            }
        }

        best_match.map(|(idx, _)| idx)
    }

    /// Find an empty slot or the weakest instrument to replace
    fn find_empty_or_weakest_slot(&self) -> Option<usize> {
        // First try to find an empty slot
        for (i, inst) in self.instruments.iter().enumerate() {
            if inst.confidence < MIN_CONFIDENCE {
                return Some(i);
            }
        }

        // Find the weakest established instrument
        let mut weakest_idx = 0;
        let mut weakest_score = f32::MAX;

        for (i, inst) in self.instruments.iter().enumerate() {
            // Score based on confidence and recent activity
            let activity_bonus = if inst.inactive_frames < 30 { 0.5 } else { 0.0 };
            let score = inst.confidence + activity_bonus;
            if score < weakest_score {
                weakest_score = score;
                weakest_idx = i;
            }
        }

        Some(weakest_idx)
    }

    /// Start tracking a new instrument in the given slot
    fn start_new_instrument(&mut self, idx: usize, peak: &SpectralPeak) {
        let inst = &mut self.instruments[idx];

        inst.id = self.next_id;
        self.next_id += 1;

        inst.signature = [0.0; NUM_BANDS];
        inst.fine_signature.clear();
        inst.center_freq = peak.center_freq;
        inst.freq_range = peak.freq_range;
        inst.dominant_band = peak.dominant_band;
        inst.confidence = 0.3; // Start with some confidence
        inst.energy = peak.energy;
        inst.inactive_frames = 0;
        inst.established = false;
        inst.pattern_frames = 1;
        inst.pattern_history.clear();
        inst.pattern_history.push(peak.bands);
    }

    /// Update an existing instrument with new matching data
    fn update_instrument(&mut self, idx: usize, peak: &SpectralPeak) {
        let inst = &mut self.instruments[idx];

        // Add to pattern history (up to ESTABLISH_FRAMES)
        if inst.pattern_history.len() < ESTABLISH_FRAMES {
            inst.pattern_history.push(peak.bands);
        } else {
            // Rolling window - remove oldest, add newest
            inst.pattern_history.remove(0);
            inst.pattern_history.push(peak.bands);
        }

        inst.pattern_frames += 1;

        // Refine signature periodically
        if inst.pattern_frames % 5 == 0 {
            inst.refine_signature();
        }

        // Update tracking state
        inst.center_freq = inst.center_freq * 0.9 + peak.center_freq * 0.1;
        inst.energy = inst.energy * 0.7 + peak.energy * 0.3;
        inst.inactive_frames = 0;

        // Increase confidence (fast attack)
        let confidence_boost = if inst.established { 0.05 } else { 0.1 };
        inst.confidence = (inst.confidence + confidence_boost).min(1.0);

        // Mark as established once we have enough samples
        if inst.pattern_frames >= ESTABLISH_FRAMES && !inst.established {
            inst.established = true;
            inst.refine_signature();
        }
    }

    /// Decay an instrument that wasn't matched this frame
    fn decay_instrument(&mut self, idx: usize) {
        let inst = &mut self.instruments[idx];

        if inst.confidence < MIN_CONFIDENCE {
            return; // Already inactive
        }

        inst.inactive_frames += 1;
        inst.energy *= 0.95; // Energy decays faster than confidence

        // Start confidence decay after grace period
        if inst.inactive_frames > DECAY_START_FRAMES {
            inst.confidence -= DECAY_RATE;

            // Also decay pattern reliability
            if !inst.pattern_history.is_empty() && inst.inactive_frames % 30 == 0 {
                inst.pattern_history.pop();
            }
        }

        // Clear slot when fully decayed
        if inst.confidence < MIN_CONFIDENCE {
            inst.pattern_frames = 0;
            inst.established = false;
            inst.pattern_history.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_correlation() {
        let mut inst = DetectedInstrument::default();
        inst.signature = [0.0, 0.5, 0.3, 0.1, 0.1, 0.0, 0.0, 0.0];
        inst.pattern_frames = PROBE_FRAMES;

        // Similar pattern should have high correlation
        let similar = [0.0, 0.6, 0.25, 0.1, 0.05, 0.0, 0.0, 0.0];
        assert!(inst.correlate(&similar) > 0.9);

        // Very different pattern should have low correlation
        let different = [0.0, 0.0, 0.0, 0.0, 0.0, 0.3, 0.5, 0.2];
        assert!(inst.correlate(&different) < 0.3);
    }

    #[test]
    fn test_tracker_detects_new_instrument() {
        let mut tracker = InstrumentTracker::new(44100.0);

        // Simulate a bass-heavy signal
        let bands = [0.8, 0.6, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0];
        let spectrum = vec![0.5; SPECTRUM_SIZE];

        // First few frames start tracking
        for _ in 0..PROBE_FRAMES {
            tracker.update(&bands, &spectrum);
        }

        let active = tracker.update(&bands, &spectrum);
        assert!(!active.is_empty(), "Should detect at least one instrument");
        assert!(
            active[0].dominant_band <= 1,
            "Should detect bass instrument"
        );
    }

    #[test]
    fn test_instrument_decay() {
        let mut tracker = InstrumentTracker::new(44100.0);

        // Establish an instrument
        let bands = [0.8, 0.6, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0];
        let spectrum = vec![0.5; SPECTRUM_SIZE];
        for _ in 0..ESTABLISH_FRAMES {
            tracker.update(&bands, &spectrum);
        }

        // Now send silence - instrument should decay
        let silence = [0.0; NUM_BANDS];
        for _ in 0..(DECAY_START_FRAMES + 50) {
            tracker.update(&silence, &spectrum);
        }

        let active = tracker.update(&silence, &spectrum);
        // After decay, instrument should be weakened or gone
        if !active.is_empty() {
            assert!(active[0].confidence < 0.5, "Confidence should have decayed");
        }
    }
}
