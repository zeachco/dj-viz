// Debug Visualization - Shows all audio metrics
// Geometry patterns for bands, centroid slider, energy meter, diff hues, boolean states
// Info panel at top, visualization elements pushed down 30%
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

// === LAYOUT: Define visualization area below info panel ===
// Info panel uses roughly 180 pixels at top, so visualization area is reduced
let info_panel_height = 200.0;
let viz_area_top = bounds_top - info_panel_height;
let viz_area_bottom = bounds_bottom;
let viz_area_h = viz_area_top - viz_area_bottom;
let viz_area_center_y = (viz_area_top + viz_area_bottom) / 2.0;

// Layout constants
let margin = 20.0;
let side_col_width = 140.0;  // Fixed width for left/right columns
let meter_width = 30.0;
let slider_height = 25.0;

// === BACKGROUND: 40% black overlay ===
let bg_center_x = (bounds_left + bounds_right) / 2.0;
let bg_center_y = (bounds_top + bounds_bottom) / 2.0;
let bg_h = bounds_top - bounds_bottom;
rect(bg_center_x, bg_center_y, bounds_w, bg_h, 0.0, 0.0, 0.0, 0.9);

// === FPS Display (top left) ===
let fps_text = "FPS: " + floor(fps);
text(bounds_left + 60.0, bounds_top - 30.0, fps_text, 24, 0.0, 0.8, 0.0, 1.0);

// === TOP: Info Panel (from debug.rs) ===
let info_top = bounds_top - 50.0;
let row_spacing = 22.0;
let column_spacing = bounds_w / 4.0;  // 3 columns, evenly spaced
let indicator_width = 100.0;

// Helper function for value-based color (gray -> green -> yellow -> red)
fn value_color(val) {
    let v = clamp(val, 0.0, 1.0);
    if v < 0.25 {
        let t = v / 0.25;
        let gray = 0.4;
        [gray * (1.0 - t), gray * (1.0 - t) + 0.8 * t, gray * (1.0 - t), 1.0]
    } else if v < 0.5 {
        [0.0, 0.8, 0.0, 1.0]
    } else if v < 0.75 {
        let t = (v - 0.5) / 0.25;
        [0.9 * t, 0.8, 0.0, 1.0]
    } else {
        let t = (v - 0.75) / 0.25;
        [0.9, 0.8 * (1.0 - t), 0.0, 1.0]
    }
}

// Helper to format float to 2 decimal places
fn fmt2(v) {
    let scaled = floor(v * 100.0 + 0.5) / 100.0;
    "" + scaled
}

// Helper to draw a value with indicator bar
fn draw_value_row(x, y, label, value, show_bar, indicator_w) {
    let col = value_color(value);
    let display_text = label + ": " + fmt2(value);
    text(x, y, display_text, 14, col[0], col[1], col[2], col[3]);

    if show_bar {
        let bar_y = y - 12.0;
        let bar_left = x - indicator_w / 2.0;
        // Background track
        line(bar_left, bar_y, bar_left + indicator_w, bar_y, 1.0, 0.3, 0.3, 0.3, 0.5);
        // Value bar
        let bar_len = clamp(value, 0.0, 1.0) * indicator_w;
        line(bar_left, bar_y, bar_left + bar_len, bar_y, 2.5, col[0], col[1], col[2], col[3] * 0.9);
    }
}

// Helper for boolean text
fn bool_text(val) {
    if val { "TRUE" } else { "FALSE" }
}

// Column 1: Frequency bands (raw values) - at 25%
let info_col1_x = bounds_left + column_spacing * 1.0;
for i in 0..8 {
    let y = info_top - row_spacing * (i * 1.0);
    draw_value_row(info_col1_x, y, "Band " + i, bands[i], true, indicator_width);
}

// Column 2: Normalized bands - at 50%
let info_col2_x = bounds_left + column_spacing * 2.0;
for i in 0..8 {
    let y = info_top - row_spacing * (i * 1.0);
    draw_value_row(info_col2_x, y, "Norm " + i, bands_normalized[i], true, indicator_width);
}

// Column 3: Bass/Mids/Treble and other info - at 75%
let info_col3_x = bounds_left + column_spacing * 3.0;
draw_value_row(info_col3_x, info_top, "Bass", bass, true, indicator_width);
draw_value_row(info_col3_x, info_top - row_spacing, "Mids", mids, true, indicator_width);
draw_value_row(info_col3_x, info_top - row_spacing * 2.0, "Treble", treble, true, indicator_width);

// Dominant band
let dom_labels = ["SUB", "BASS", "LO-M", "MID", "UP-M", "PRES", "BRIL", "AIR"];

// Last mark
text(info_col3_x, info_top - row_spacing * 3.0, "Last Mark: " + last_mark, 14, 0.0, 0.8, 0.0, 1.0);

// === 3-COLUMN LAYOUT FOR BOTTOM SECTION ===
// Column 1 (left, 140px): Energy, Rise, E.Floor bars
// Column 2 (center, flexible): Spectrum + Centroid
// Column 3 (right, 140px): Boolean indicators

let col1_left = bounds_left + margin;
let col1_right = col1_left + side_col_width;
let col3_right = bounds_right - margin;
let col3_left = col3_right - side_col_width;
let col2_left = col1_right + margin;
let col2_right = col3_left - margin;
let col2_center_x = (col2_left + col2_right) / 2.0;
let col2_w = col2_right - col2_left;

// === COLUMN 1 (LEFT): Energy, Rise, E.Floor Vertical Meters ===
let meter_h = viz_area_h * 0.20;
let meter_y = viz_area_bottom + margin + slider_height + meter_h / 2.0;
let meter_spacing = (side_col_width - meter_width * 3.0) / 4.0;  // Spacing between 3 meters

// Helper function for drawing a vertical meter
fn draw_vertical_meter(cx, cy, w, h, value, label, hue_base, hue_range) {
    // Meter background
    rect(cx, cy, w, h, 0.15, 0.15, 0.2, 0.8);

    // Value fill
    let fill_h = clamp(value, 0.0, 1.0) * h;
    let fill_y = cy - h / 2.0 + fill_h / 2.0;
    let hue = hue_base - value * hue_range;
    let col = hsla(hue, 0.8, 0.5, 0.9);
    if fill_h > 1.0 {
        rect(cx, fill_y, w - 4.0, fill_h, col[0], col[1], col[2], col[3]);
    }

    // Label
    text(cx, cy + h / 2.0 + 12.0, label, 10, 0.7, 0.7, 0.7, 1.0);
}

// Energy meter
let meter1_x = col1_left + meter_spacing + meter_width / 2.0;
draw_vertical_meter(meter1_x, meter_y, meter_width, meter_h, energy, "ENERGY", 0.3, 0.3);

// Rise meter (map -1..1 to 0..1)
let rise_mapped = (rise_rate + 1.0) / 2.0;
let meter2_x = meter1_x + meter_width + meter_spacing;
draw_vertical_meter(meter2_x, meter_y, meter_width, meter_h, rise_mapped, "RISE", 0.6, 0.6);

// E.Floor meter
let meter3_x = meter2_x + meter_width + meter_spacing;
draw_vertical_meter(meter3_x, meter_y, meter_width, meter_h, energy_floor, "FLOOR", 0.15, 0.15);

// === COLUMN 2 (CENTER): Spectrum + Centroid ===
// Centroid slider at bottom
let slider_w = col2_w;
let slider_x = col2_center_x;
let slider_y = viz_area_bottom + margin;

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

// Frequency ticks
let freq_marks = [100.0, 500.0, 1000.0, 5000.0, 10000.0];
for f in freq_marks {
    let f_norm = (f - min_freq) / (max_freq - min_freq);
    let f_log = sqrt(f_norm);
    let tick_x = slider_x - slider_w / 2.0 + f_log * slider_w;
    line(tick_x, slider_y - slider_height / 2.0, tick_x, slider_y + slider_height / 2.0, 1.0, 0.4, 0.4, 0.4, 0.6);
}

// Centroid label
text(slider_x, slider_y + slider_height + 10.0, "CENTROID", 12, 0.7, 0.7, 0.7, 1.0);

// Spectrum visualization above centroid
let spec_height = viz_area_h * 0.15;
let spec_y_base = slider_y + slider_height + 30.0;
let spec_w = slider_w;
let spec_left = col2_left;

// Background for spectrum area
rect(slider_x, spec_y_base + spec_height / 2.0, spec_w, spec_height, 0.1, 0.1, 0.12, 0.7);

// Draw spectrum bars
let spec_len = spectrum.len();
if spec_len > 0 {
    let bar_w = spec_w / (spec_len * 1.0);
    for i in 0..spec_len {
        let mag = spectrum[i];
        let diff = spectrum_diff[i];
        let bar_h = mag * spec_height;
        let hue = clamp(0.3 - diff * 0.3, 0.0, 0.6);
        let sat = 0.7 + abs(diff) * 0.3;
        let light = 0.3 + mag * 0.4;
        let col_rgba = hsla(hue, sat, light, 0.9);
        let x = spec_left + (i * 1.0 + 0.5) * bar_w;
        if bar_h > 0.5 {
            rect(x, spec_y_base + bar_h / 2.0, bar_w, bar_h, col_rgba[0], col_rgba[1], col_rgba[2], col_rgba[3]);
        }
    }
}

// Spectrum label
text(slider_x, spec_y_base + spec_height + 12.0, "SPECTRUM", 12, 0.7, 0.7, 0.7, 1.0);

// === COLUMN 3 (RIGHT): Boolean State Indicators ===
let bool_area_center_x = (col3_left + col3_right) / 2.0;
let bool_row_spacing = 24.0;
let indicator_size = 14.0;

// Helper function for drawing boolean indicator
fn draw_bool_indicator(x, y, label, timer, fade_dur, indicator_size) {
    let progress = timer / fade_dur;
    let is_on = progress > 0.0;

    // Background circle
    let bg_col = if is_on {
        hsla(0.3, 0.8, 0.2 + progress * 0.4, 0.9)
    } else {
        [0.2, 0.15, 0.15, 0.7]
    };

    ellipse(x - 40.0, y, indicator_size, indicator_size, bg_col[0], bg_col[1], bg_col[2], bg_col[3]);

    // Glow when on
    if is_on {
        let glow = hsla(0.3, 0.6, 0.5, progress * 0.3);
        ellipse(x - 40.0, y, indicator_size + 8.0, indicator_size + 8.0, glow[0], glow[1], glow[2], glow[3]);
    }

    // Label
    text(x, y, label, 10, 0.7, 0.7, 0.7, 1.0);
}

// Stack indicators vertically, aligned with spectrum height
let bool_base_y = spec_y_base + spec_height - 10.0;
draw_bool_indicator(bool_area_center_x, bool_base_y, "TRANSITION", tt_transition, fade_duration, indicator_size);
draw_bool_indicator(bool_area_center_x, bool_base_y - bool_row_spacing, "PUNCH", tt_punch, fade_duration, indicator_size);
draw_bool_indicator(bool_area_center_x, bool_base_y - bool_row_spacing * 2.0, "BREAK", tt_break, fade_duration, indicator_size);
draw_bool_indicator(bool_area_center_x, bool_base_y - bool_row_spacing * 3.0, "INST+", tt_inst_add, fade_duration, indicator_size);
draw_bool_indicator(bool_area_center_x, bool_base_y - bool_row_spacing * 4.0, "INST-", tt_inst_rem, fade_duration, indicator_size);
draw_bool_indicator(bool_area_center_x, bool_base_y - bool_row_spacing * 5.0, "VIZ CHG", tt_viz_chg, fade_duration, indicator_size);

// === CENTER: Geometry Patterns for All Bands ===
let center_area_left = col1_right + margin;
let center_area_right = col3_left - margin;
// Account for centroid slider + spectrum panel + labels
let center_area_bottom = spec_y_base + spec_height + 30.0;
let center_area_top = viz_area_top - 10.0;  // Use viz area boundary
let center_w = center_area_right - center_area_left;
let center_h = center_area_top - center_area_bottom;
let center_x = (center_area_left + center_area_right) / 2.0;
let center_y = (center_area_bottom + center_area_top) / 2.0;

// Draw 8 geometric patterns arranged in a circle
let pattern_radius = min(center_w, center_h) * 0.30;
let shape_max_size = pattern_radius * 0.35;

let radial_size = min(center_w, center_h);
ellipse(center_x, center_y, radial_size, radial_size, 0.0, 0.0, 0.0, 0.6);

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

    let size = 12.0 + band_val * shape_max_size;
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
    text(px, py - size - 12.0, labels[i], 10, 0.6, 0.6, 0.6, 0.8);
}

// === CENTER: Additional info display ===
// BPM display
let bpm_display = "BPM: " + floor(bpm);
text(center_x, center_y + 15.0, bpm_display, 14, 0.9, 0.9, 0.9, 1.0);

// Dominant band indicator
let dom_text2 = "DOM: " + dom_labels[dominant_band];
text(center_x, center_y, dom_text2, 12, 0.8, 0.8, 0.8, 1.0);

// Rise/Fall indicator
let rise_text = if rise_rate > 0.05 { "RISING" } else if rise_rate < -0.05 { "FALLING" } else { "STABLE" };
let rise_col = if rise_rate > 0.05 { [0.3, 0.9, 0.3, 1.0] } else if rise_rate < -0.05 { [0.9, 0.3, 0.3, 1.0] } else { [0.7, 0.7, 0.7, 1.0] };
text(center_x, center_y - 15.0, rise_text, 11, rise_col[0], rise_col[1], rise_col[2], rise_col[3]);
