//! AudioAnalysis exposure to Rhai scripts.
//!
//! Pushes audio analysis data as read-only constants into the Rhai scope.

use crate::audio::AudioAnalysis;
use nannou::geom::Rect;
use rhai::{Dynamic, Scope};

/// Push all AudioAnalysis fields as read-only constants to the scope.
/// Also pushes window bounds and frame counter.
pub fn push_audio_to_scope(scope: &mut Scope, analysis: &AudioAnalysis, bounds: Rect, frame: i64) {
    // Aggregate metrics (0-1 range)
    scope.push_constant("energy", analysis.energy as f64);
    scope.push_constant("bass", analysis.bass as f64);
    scope.push_constant("mids", analysis.mids as f64);
    scope.push_constant("treble", analysis.treble as f64);

    // Frequency bands as array
    let bands: rhai::Array = analysis
        .bands
        .iter()
        .map(|&b| Dynamic::from(b as f64))
        .collect();
    scope.push_constant("bands", bands);

    // Normalized bands (relative to tracked min/max)
    let bands_normalized: rhai::Array = analysis
        .bands_normalized
        .iter()
        .map(|&b| Dynamic::from(b as f64))
        .collect();
    scope.push_constant("bands_normalized", bands_normalized);

    // Temporal metrics
    scope.push_constant("bpm", analysis.bpm as f64);
    scope.push_constant("dominant_band", analysis.dominant_band as i64);
    scope.push_constant("energy_diff", analysis.energy_diff as f64);
    scope.push_constant("rise_rate", analysis.rise_rate as f64);
    scope.push_constant("spectral_centroid", analysis.spectral_centroid as f64);

    // Event flags
    scope.push_constant("transition_detected", analysis.transition_detected);
    scope.push_constant("punch_detected", analysis.punch_detected);
    scope.push_constant("break_detected", analysis.break_detected);
    scope.push_constant("instrument_added", analysis.instrument_added);
    scope.push_constant("instrument_removed", analysis.instrument_removed);
    scope.push_constant("viz_change_triggered", analysis.viz_change_triggered);

    // Window bounds
    scope.push_constant("bounds_w", bounds.w() as f64);
    scope.push_constant("bounds_h", bounds.h() as f64);
    scope.push_constant("bounds_left", bounds.left() as f64);
    scope.push_constant("bounds_right", bounds.right() as f64);
    scope.push_constant("bounds_top", bounds.top() as f64);
    scope.push_constant("bounds_bottom", bounds.bottom() as f64);

    // Frame counter
    scope.push_constant("frame", frame);
}
