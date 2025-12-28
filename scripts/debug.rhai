// Debug Visualization - Shows all audio metrics
// Geometry patterns for bands, centroid slider, energy meter, diff hues, boolean states
//
// Persistent variable API:
//   init(name, value) - initialize only if not set, returns current value
//   get(name)         - get value, returns () if not set
//   set(name, value)  - set value

let dt = 1.0 / 60.0;  // Approx frame time
let fade_duration = 2.0;  // How long to keep ON color

// Get persistent trigger timers (initialized to 0.0 on first frame / hot-reload)
let tt_transition = init("tt_transition", 0.0);
let tt_punch = init("tt_punch", 0.0);
let tt_break = init("tt_break", 0.0);
let tt_inst_add = init("tt_inst_add", 0.0);
let tt_inst_rem = init("tt_inst_rem", 0.0);
let tt_viz_chg = init("tt_viz_chg", 0.0);

// Update trigger timers - set to fade_duration when triggered, count down otherwise
if transition_detected { tt_transition = fade_duration; }
else { tt_transition = max(0.0, tt_transition - dt); }
set("tt_transition", tt_transition);

if punch_detected { tt_punch = fade_duration; }
else { tt_punch = max(0.0, tt_punch - dt); }
set("tt_punch", tt_punch);

if break_detected { tt_break = fade_duration; }
else { tt_break = max(0.0, tt_break - dt); }
set("tt_break", tt_break);

if instrument_added { tt_inst_add = fade_duration; }
else { tt_inst_add = max(0.0, tt_inst_add - dt); }
set("tt_inst_add", tt_inst_add);

if instrument_removed { tt_inst_rem = fade_duration; }
else { tt_inst_rem = max(0.0, tt_inst_rem - dt); }
set("tt_inst_rem", tt_inst_rem);

if viz_change_triggered { tt_viz_chg = fade_duration; }
else { tt_viz_chg = max(0.0, tt_viz_chg - dt); }
set("tt_viz_chg", tt_viz_chg);

// Layout constants
let margin = 40.0;
let meter_width = 30.0;
let slider_height = 25.0;
let bool_panel_width = 120.0;

// === LEFT: Energy Floor Vertical Meter ===
let meter_x = bounds_left + margin + meter_width / 2.0;
let meter_h = bounds_h - margin * 3.0 - slider_height;
let meter_y = bounds_bottom + margin + slider_height + meter_h / 2.0;

// Meter background
rect(meter_x, meter_y, meter_width, meter_h, 0.15, 0.15, 0.2, 0.8);

// Energy fill
let fill_h = energy * meter_h;
let fill_y = bounds_bottom + margin + slider_height + fill_h / 2.0;
let energy_hue = 0.3 - energy * 0.3;  // Green to red
let e_col = hsla(energy_hue, 0.8, 0.5, 0.9);
rect(meter_x, fill_y, meter_width - 4.0, fill_h, e_col[0], e_col[1], e_col[2], e_col[3]);

// Energy label
text(meter_x, bounds_top - 15.0, "ENERGY", 12, 0.7, 0.7, 0.7, 1.0);

// === BOTTOM: Centroid Horizontal Slider ===
let slider_w = bounds_w - margin * 4.0 - meter_width - bool_panel_width;
let slider_x = bounds_left + margin * 2.0 + meter_width + slider_w / 2.0;
let slider_y = bounds_bottom + margin;

// Slider background
rect(slider_x, slider_y, slider_w, slider_height, 0.15, 0.15, 0.2, 0.8);

// Centroid position (normalize 20Hz-20000Hz to 0-1, log scale)
let min_freq = 20.0;
let max_freq = 20000.0;
let centroid_norm = clamp((spectral_centroid - min_freq) / (max_freq - min_freq), 0.0, 1.0);
let centroid_log = sqrt(centroid_norm);  // Approximate log scale
let indicator_x = slider_x - slider_w / 2.0 + centroid_log * slider_w;

// Centroid indicator
let c_col = hsla(centroid_log * 0.8, 0.9, 0.6, 1.0);
ellipse(indicator_x, slider_y, 20.0, slider_height - 4.0, c_col[0], c_col[1], c_col[2], c_col[3]);

// Frequency ticks and labels
let freq_marks = [100.0, 500.0, 1000.0, 5000.0, 10000.0];
for f in freq_marks {
    let f_norm = (f - min_freq) / (max_freq - min_freq);
    let f_log = sqrt(f_norm);
    let tick_x = slider_x - slider_w / 2.0 + f_log * slider_w;
    line(tick_x, slider_y - slider_height / 2.0, tick_x, slider_y + slider_height / 2.0, 1.0, 0.4, 0.4, 0.4, 0.6);
}

// Centroid label with frequency
text(slider_x, slider_y + slider_height + 10.0, "CENTROID", 12, 0.7, 0.7, 0.7, 1.0);

// === RIGHT: Boolean State Indicators ===
let bool_x = bounds_right - margin - bool_panel_width / 2.0;
let bool_start_y = bounds_top - margin;
let bool_spacing = 35.0;
let indicator_size = 20.0;

// Helper function for drawing boolean indicator
fn draw_bool_indicator(x, y, label, timer, fade_dur) {
    let progress = timer / fade_dur;
    let is_on = progress > 0.0;

    // Background circle
    let bg_col = if is_on {
        hsla(0.3, 0.8, 0.2 + progress * 0.4, 0.9)  // Green, brighter when fresh
    } else {
        [0.2, 0.15, 0.15, 0.7]  // Dark red-ish off state
    };

    ellipse(x - 45.0, y, indicator_size, indicator_size, bg_col[0], bg_col[1], bg_col[2], bg_col[3]);

    // Glow when on
    if is_on {
        let glow = hsla(0.3, 0.6, 0.5, progress * 0.3);
        ellipse(x - 45.0, y, indicator_size + 10.0, indicator_size + 10.0, glow[0], glow[1], glow[2], glow[3]);
    }

    // Label
    text(x, y, label, 11, 0.7, 0.7, 0.7, 1.0);
}

draw_bool_indicator(bool_x, bool_start_y, "TRANSITION", tt_transition, fade_duration);
draw_bool_indicator(bool_x, bool_start_y - bool_spacing, "PUNCH", tt_punch, fade_duration);
draw_bool_indicator(bool_x, bool_start_y - bool_spacing * 2.0, "BREAK", tt_break, fade_duration);
draw_bool_indicator(bool_x, bool_start_y - bool_spacing * 3.0, "INST+", tt_inst_add, fade_duration);
draw_bool_indicator(bool_x, bool_start_y - bool_spacing * 4.0, "INST-", tt_inst_rem, fade_duration);
draw_bool_indicator(bool_x, bool_start_y - bool_spacing * 5.0, "VIZ CHG", tt_viz_chg, fade_duration);

// === CENTER: Geometry Patterns for All Bands ===
let center_area_left = bounds_left + margin * 2.0 + meter_width + 20.0;
let center_area_right = bounds_right - margin - bool_panel_width - 20.0;
let center_area_bottom = bounds_bottom + margin * 2.0 + slider_height + 20.0;
let center_area_top = bounds_top - margin - 20.0;
let center_w = center_area_right - center_area_left;
let center_h = center_area_top - center_area_bottom;
let center_x = (center_area_left + center_area_right) / 2.0;
let center_y = (center_area_bottom + center_area_top) / 2.0;

// Draw 8 geometric patterns arranged in a circle
let pattern_radius = min(center_w, center_h) * 0.35;
let shape_max_size = pattern_radius * 0.4;

for i in 0..8 {
    let angle = (i * tau() / 8.0) - tau() / 4.0;  // Start from top
    let px = center_x + cos(angle) * pattern_radius;
    let py = center_y + sin(angle) * pattern_radius;

    let band_val = bands[i];
    let band_norm = bands_normalized[i];

    // Use energy_diff and rise_rate to influence hue
    // Normalize diffs: energy_diff roughly -0.5 to 0.5, rise_rate similar
    let diff_hue = (energy_diff + 0.5) * 0.5;  // Map to 0-0.5 range
    let rise_hue = (rise_rate + 0.5) * 0.3;   // Additional hue shift

    // Base hue from band index, modified by diffs
    let base_hue = i / 8.0;
    let final_hue = (base_hue + diff_hue + rise_hue) % 1.0;

    let size = 15.0 + band_val * shape_max_size;
    let sat = 0.6 + band_val * 0.4;
    let light = 0.3 + band_val * 0.3;
    let col = hsla(final_hue, sat, light, 0.85);

    // Different geometry for each band
    if i == 0 {
        // Sub-bass: Large pulsing circle
        ellipse(px, py, size * 1.5, size * 1.5, col[0], col[1], col[2], col[3]);
    } else if i == 1 {
        // Bass: Square
        rect(px, py, size * 1.3, size * 1.3, col[0], col[1], col[2], col[3]);
    } else if i == 2 {
        // Low-mid: Diamond (rotated square via 4 triangles approximated with lines)
        let d = size * 0.9;
        line(px, py - d, px + d, py, 3.0, col[0], col[1], col[2], col[3]);
        line(px + d, py, px, py + d, 3.0, col[0], col[1], col[2], col[3]);
        line(px, py + d, px - d, py, 3.0, col[0], col[1], col[2], col[3]);
        line(px - d, py, px, py - d, 3.0, col[0], col[1], col[2], col[3]);
        ellipse(px, py, size * 0.5, size * 0.5, col[0], col[1], col[2], col[3]);
    } else if i == 3 {
        // Mid: Hexagon approximation with lines
        for j in 0..6 {
            let a1 = j * tau() / 6.0;
            let a2 = (j + 1) * tau() / 6.0;
            let r = size * 0.8;
            line(px + cos(a1) * r, py + sin(a1) * r, px + cos(a2) * r, py + sin(a2) * r, 3.0, col[0], col[1], col[2], col[3]);
        }
    } else if i == 4 {
        // Upper-mid: Triangle pointing up
        let t = size * 0.9;
        line(px, py - t, px - t * 0.866, py + t * 0.5, 3.0, col[0], col[1], col[2], col[3]);
        line(px - t * 0.866, py + t * 0.5, px + t * 0.866, py + t * 0.5, 3.0, col[0], col[1], col[2], col[3]);
        line(px + t * 0.866, py + t * 0.5, px, py - t, 3.0, col[0], col[1], col[2], col[3]);
    } else if i == 5 {
        // Presence: Star pattern (crossing lines)
        let s = size * 0.8;
        for j in 0..4 {
            let a = j * tau() / 8.0;
            line(px - cos(a) * s, py - sin(a) * s, px + cos(a) * s, py + sin(a) * s, 2.0, col[0], col[1], col[2], col[3]);
        }
    } else if i == 6 {
        // Brilliance: Octagon
        for j in 0..8 {
            let a1 = j * tau() / 8.0;
            let a2 = (j + 1) * tau() / 8.0;
            let r = size * 0.7;
            line(px + cos(a1) * r, py + sin(a1) * r, px + cos(a2) * r, py + sin(a2) * r, 2.0, col[0], col[1], col[2], col[3]);
        }
    } else {
        // Air: Small ellipse with radiating lines
        ellipse(px, py, size * 0.4, size * 0.4, col[0], col[1], col[2], col[3]);
        for j in 0..6 {
            let a = j * tau() / 6.0;
            let r = size * 0.7;
            line(px, py, px + cos(a) * r, py + sin(a) * r, 1.5, col[0], col[1], col[2], col[3] * 0.5);
        }
    }

    // Band label
    let labels = ["SUB", "BASS", "LO-M", "MID", "UP-M", "PRES", "BRIL", "AIR"];
    text(px, py - size - 15.0, labels[i], 10, 0.6, 0.6, 0.6, 0.8);
}

// === CENTER: Additional info display ===
// BPM display
let bpm_text = "BPM: " + floor(bpm);
text(center_x, center_y + 20.0, bpm_text, 16, 0.9, 0.9, 0.9, 1.0);

// Dominant band indicator
let dom_labels = ["SUB", "BASS", "LO-M", "MID", "UP-M", "PRES", "BRIL", "AIR"];
let dom_text = "DOM: " + dom_labels[dominant_band];
text(center_x, center_y, dom_text, 14, 0.8, 0.8, 0.8, 1.0);

// Rise/Fall indicator
let rise_text = if rise_rate > 0.05 { "RISING" } else if rise_rate < -0.05 { "FALLING" } else { "STABLE" };
let rise_col = if rise_rate > 0.05 { [0.3, 0.9, 0.3, 1.0] } else if rise_rate < -0.05 { [0.9, 0.3, 0.3, 1.0] } else { [0.7, 0.7, 0.7, 1.0] };
text(center_x, center_y - 20.0, rise_text, 12, rise_col[0], rise_col[1], rise_col[2], rise_col[3]);

// === Bass/Mids/Treble bars at bottom of center area ===
let bar_y = center_area_bottom + 30.0;
let bar_h = 15.0;
let bar_spacing = 80.0;

// Bass bar
let bass_w = bass * 60.0 + 10.0;
let bass_col = hsla(0.0, 0.8, 0.5, 0.9);
rect(center_x - bar_spacing, bar_y, bass_w, bar_h, bass_col[0], bass_col[1], bass_col[2], bass_col[3]);
text(center_x - bar_spacing, bar_y + 20.0, "BASS", 10, 0.6, 0.6, 0.6, 0.8);

// Mids bar
let mids_w = mids * 60.0 + 10.0;
let mids_col = hsla(0.3, 0.8, 0.5, 0.9);
rect(center_x, bar_y, mids_w, bar_h, mids_col[0], mids_col[1], mids_col[2], mids_col[3]);
text(center_x, bar_y + 20.0, "MIDS", 10, 0.6, 0.6, 0.6, 0.8);

// Treble bar
let treble_w = treble * 60.0 + 10.0;
let treble_col = hsla(0.6, 0.8, 0.5, 0.9);
rect(center_x + bar_spacing, bar_y, treble_w, bar_h, treble_col[0], treble_col[1], treble_col[2], treble_col[3]);
text(center_x + bar_spacing, bar_y + 20.0, "TREBLE", 10, 0.6, 0.6, 0.6, 0.8);
