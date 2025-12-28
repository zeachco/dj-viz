//! AudioAnalysis exposure to Rhai scripts.
//!
//! Updates audio analysis data in the Rhai scope each frame.
//! Uses set_or_push to update existing vars or create them on first frame.

use crate::audio::AudioAnalysis;
use nannou::geom::Rect;
use rhai::{Dynamic, Scope};

/// Update all AudioAnalysis fields in the scope.
/// Uses set_or_push so variables are created on first frame, updated on subsequent frames.
/// This allows user-defined variables to persist between frames.
pub fn update_audio_in_scope(scope: &mut Scope, analysis: &AudioAnalysis, bounds: Rect, frame: i64) {
    // Aggregate metrics (0-1 range)
    scope.set_or_push("energy", analysis.energy as f64);
    scope.set_or_push("bass", analysis.bass as f64);
    scope.set_or_push("mids", analysis.mids as f64);
    scope.set_or_push("treble", analysis.treble as f64);

    // Frequency bands as array
    let bands: rhai::Array = analysis
        .bands
        .iter()
        .map(|&b| Dynamic::from(b as f64))
        .collect();
    scope.set_or_push("bands", bands);

    // Normalized bands (relative to tracked min/max)
    let bands_normalized: rhai::Array = analysis
        .bands_normalized
        .iter()
        .map(|&b| Dynamic::from(b as f64))
        .collect();
    scope.set_or_push("bands_normalized", bands_normalized);

    // Temporal metrics
    scope.set_or_push("bpm", analysis.bpm as f64);
    scope.set_or_push("dominant_band", analysis.dominant_band as i64);
    scope.set_or_push("energy_diff", analysis.energy_diff as f64);
    scope.set_or_push("rise_rate", analysis.rise_rate as f64);
    scope.set_or_push("spectral_centroid", analysis.spectral_centroid as f64);

    // Event flags
    scope.set_or_push("transition_detected", analysis.transition_detected);
    scope.set_or_push("punch_detected", analysis.punch_detected);
    scope.set_or_push("break_detected", analysis.break_detected);
    scope.set_or_push("instrument_added", analysis.instrument_added);
    scope.set_or_push("instrument_removed", analysis.instrument_removed);
    scope.set_or_push("viz_change_triggered", analysis.viz_change_triggered);

    // Window bounds
    scope.set_or_push("bounds_w", bounds.w() as f64);
    scope.set_or_push("bounds_h", bounds.h() as f64);
    scope.set_or_push("bounds_left", bounds.left() as f64);
    scope.set_or_push("bounds_right", bounds.right() as f64);
    scope.set_or_push("bounds_top", bounds.top() as f64);
    scope.set_or_push("bounds_bottom", bounds.bottom() as f64);

    // Frame counter
    scope.set_or_push("frame", frame);
}
