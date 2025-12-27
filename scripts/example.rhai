// Example visualization demonstrating the Rhai scripting API
// Hot-reload: save this file and changes appear immediately!

// Initialize persistent state on first frame
if frame == 1 {
    let hue_offset = 0.0;
    let particles = [];
}

// Cycle hue on musical transitions
if transition_detected {
    hue_offset = (hue_offset + 0.15) % 1.0;
}

// Spawn particles on bass hits
if bass > 0.5 && rand() < bass * 0.3 {
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
particles = new_particles;

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
