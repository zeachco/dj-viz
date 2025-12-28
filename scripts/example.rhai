// Example visualization demonstrating the Rhai scripting API
// Hot-reload: save this file and changes appear immediately!
//
// ============================================================================
// AVAILABLE VARIABLES (read-only)
// ============================================================================
//
// --- Audio Metrics (0.0-1.0 range) ---
// energy              : Overall audio energy level
// bass                : Low frequency energy
// mids                : Mid frequency energy
// treble              : High frequency energy
// bands               : Array of 8 frequency band values
// bands_normalized    : Array of 8 normalized frequency bands (relative to tracked min/max)
//
// --- Temporal Metrics ---
// bpm                 : Detected beats per minute
// dominant_band       : Index of the loudest frequency band (0-7)
// energy_diff         : Change in energy from previous frame
// rise_rate           : Rate of energy increase
// spectral_centroid   : Center of mass of the frequency spectrum
//
// --- Event Flags (boolean) ---
// transition_detected : Musical transition occurred (e.g., verse to chorus)
// punch_detected      : Sudden energy spike detected
// break_detected      : Quiet moment / breakdown detected
// instrument_added    : New frequency content appeared
// instrument_removed  : Frequency content disappeared
// viz_change_triggered: Visualization change was triggered
//
// --- Window Bounds ---
// bounds_w            : Window width in pixels
// bounds_h            : Window height in pixels
// bounds_left         : Left edge x-coordinate (negative)
// bounds_right        : Right edge x-coordinate (positive)
// bounds_top          : Top edge y-coordinate (positive)
// bounds_bottom       : Bottom edge y-coordinate (negative)
//
// --- Frame Counter ---
// frame               : Current frame number (starts at 1)
//
// ============================================================================
// AVAILABLE FUNCTIONS
// ============================================================================
//
// --- Drawing ---
// rect(x, y, w, h, r, g, b, a)              : Draw rectangle at (x,y) with size (w,h)
// ellipse(x, y, w, h, r, g, b, a)           : Draw ellipse at (x,y) with size (w,h)
// line(x1, y1, x2, y2, stroke, r, g, b, a)  : Draw line from (x1,y1) to (x2,y2)
// text(x, y, content, size, r, g, b, a)     : Draw text at (x,y) with font size
//
// --- Color ---
// hsla(h, s, l, a) -> [r, g, b, a]          : Convert HSL to RGBA array
//
// --- Math ---
// sin(x), cos(x), tan(x)                    : Trigonometric functions
// abs(x)                                    : Absolute value
// sqrt(x)                                   : Square root
// pow(x, y)                                 : x raised to power y
// floor(x), ceil(x)                         : Round down/up
// min(x, y), max(x, y)                      : Minimum/maximum
// clamp(x, min, max)                        : Clamp value to range
//
// --- Random ---
// rand()                                    : Random float 0.0-1.0
// rand_range(min, max)                      : Random float in range
//
// --- Constants ---
// pi()                                      : 3.14159...
// tau()                                     : 6.28318... (2*pi)
//
// --- Persistent Variables (survive between frames, cleared on hot-reload) ---
// init(name, value)                         : Set only if not set, returns current value
// get(name)                                 : Get value, returns () if not set
// set(name, value)                          : Set value
// get_or(name, default)                     : Get value or return default (doesn't store)
//
// ============================================================================

// Initialize persistent state (survives between frames, reset on hot-reload)
let hue_offset = init("hue_offset", 0.0);
let particles = init("particles", []);

// Cycle hue on musical transitions
if transition_detected {
    hue_offset = (hue_offset + 0.15) % 1.0;
}
set("hue_offset", hue_offset);

// Spawn particles on bass hits
if bass > 0.5 {
    for i in 0..5 {
        let p = #{
            x: rand_range(-bounds_w / 2.0, bounds_w / 2.0),
            y: bounds_bottom,
            vx: rand_range(-3.0, 3.0),
            vy: rand_range(3.0, 8.0) * (0.5 + bass),
            size: 15.0 + bass * 40.0,
            life: 1.0,
            hue: (rand() + hue_offset) % 1.0
        };
        particles.push(p);
    }
}

// Update and draw particles
let new_particles = [];

for p in particles {
    // Apply gravity and movement
    p.vy -= 0.15;
    p.x += p.vx;
    p.y += p.vy;
    p.life -= 0.008;

    // Pulse size with energy
    let size = p.size * (0.7 + energy * 0.5) * p.life;

    if p.life > 0.0 && p.y > bounds_bottom - 50.0 {
        let color = hsla(p.hue, 0.8, 0.5, p.life * 0.8);
        ellipse(p.x, p.y, size, size, color[0], color[1], color[2], color[3]);
        new_particles.push(p);
    }
}
set("particles", new_particles);

// Draw frequency bars at the bottom
let bar_width = bounds_w / 8.0;
for i in 0..8 {
    let x = bounds_left + (i + 0.5) * bar_width;
    let h = bands[i] * bounds_h * 0.25;
    let y = bounds_bottom + h / 2.0;

    let hue = (i / 8.0 + hue_offset) % 1.0;
    let color = hsla(hue, 0.7, 0.4 + bands[i] * 0.2, 0.9);

    rect(x, y, bar_width - 6.0, h, color[0], color[1], color[2], color[3]);
}

// Draw center circle that pulses with energy
let center_size = 50.0 + energy * 150.0;
let center_color = hsla(hue_offset, 0.6, 0.3 + energy * 0.3, 0.7);
ellipse(0.0, 0.0, center_size, center_size,
        center_color[0], center_color[1], center_color[2], center_color[3]);

// Draw radiating lines on punch
if punch_detected {
    for i in 0..12 {
        let angle = i * tau() / 12.0;
        let len = 100.0 + energy * 200.0;
        let x2 = cos(angle) * len;
        let y2 = sin(angle) * len;
        let color = hsla((hue_offset + 0.5) % 1.0, 1.0, 0.6, 0.8);
        line(0.0, 0.0, x2, y2, 3.0, color[0], color[1], color[2], color[3]);
    }
}
