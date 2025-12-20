---
name: new-visualization
description: Generate hypnotic, fast-responsive visualizations for live electronic music (110-160 BPM techno/psytrance). Leverage pre-computed audio analysis, GPU shaders, particle systems, and trippy effects.
---

# New Visualization for Live Electronic Music

## Design Philosophy

Create **weird, responsive, hypnotic** visualizations optimized for 110-160 BPM electronic music (afro, techno, psytrance, hypnotic techno). Target:
- **Ultra-responsive**: Fast attack (0.6-0.7), slow decay (0.15) to match rapid kick drums
- **Hypnotic patterns**: Spirals, rotations, symmetry, endless loops, tunnel effects
- **Driving energy**: Emphasize bass/kick (bands 0-1) and hi-hat/cymbals (treble)
- **Psychedelic**: Color cycling, optical illusions, fractals, feedback loops

## Audio Analysis Structure

Your visualization receives pre-computed `AudioAnalysis` (NO manual FFT needed):

```rust
pub struct AudioAnalysis {
    pub bands: [f32; 8],           // 8 frequency bands (0-1, smoothed)
                                   // [0-1]: Bass/sub-bass (kick drums)
                                   // [2-4]: Mids (synths, vocals)
                                   // [5-7]: Treble (hi-hats, cymbals)
    pub energy: f32,               // Overall energy (0-1)
    pub transition_detected: bool, // Musical transitions (genre changes, drops)
    pub bass: f32,                 // Combined bass energy (bands 0-1)
    pub mids: f32,                 // Combined mid energy (bands 2-4)
    pub treble: f32,               // Combined treble energy (bands 5-7)
}
```

## Implementation Template

### 1. Create the module file

Create `src/renderer/<name>.rs`:

```rust
//! Brief description of the visual effect
//!
//! More details about what makes it unique

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;
use crate::audio::AudioAnalysis;

// Use cfg! for debug/release performance tuning
const NUM_PARTICLES: usize = if cfg!(debug_assertions) { 200 } else { 500 };

pub struct <Name> {
    // State: particles, rotations, smoothed values, frame counters
    // Keep state minimal for performance
    bass: f32,              // Smoothed bass for kick response
    treble: f32,            // Smoothed treble for hi-hat response
    rotation: f32,          // Rotation accumulator for spinning effects
    hue_offset: f32,        // Color cycling (0-360 degrees)
    particles: Vec<Particle>, // Particle systems work great for techno
    frame_count: u32,       // For timing/pulsing effects
}

#[derive(Clone)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
    age: f32,
    // Add properties: hue, size, band_idx, etc.
}

impl <Name> {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        // Initialize particles/state
        Self {
            bass: 0.0,
            treble: 0.0,
            rotation: 0.0,
            hue_offset: 0.0,
            particles: Vec::new(),
            frame_count: 0,
        }
    }
}

impl Visualization for <Name> {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count += 1;

        // CRITICAL: Fast attack, slow decay for techno responsiveness
        let attack = 0.7;  // Snap to peaks quickly
        let decay = 0.15;  // Hold energy longer

        if analysis.bass > self.bass {
            self.bass = self.bass * (1.0 - attack) + analysis.bass * attack;
        } else {
            self.bass = self.bass * (1.0 - decay) + analysis.bass * decay;
        }

        // Similar smoothing for treble (hi-hats/cymbals)
        self.treble = self.treble * 0.8 + analysis.treble * 0.2;

        // Rotation drives hypnotic spinning (faster with energy)
        self.rotation += 0.01 + analysis.energy * 0.05;

        // Color cycling for psychedelic effect
        self.hue_offset += 0.5 + analysis.energy * 2.0;
        if self.hue_offset > 360.0 { self.hue_offset -= 360.0; }

        // Update particles, spawn based on energy, etc.
        // Map particle behaviors to specific frequency bands
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0;

        // Draw geometry: radial patterns, tunnels, spirals
        // Use center + polar coordinates (radius, angle) for hypnotic effects
        // Alpha blending + feedback creates trails (handled by FeedbackRenderer)
    }
}
```

### 2. Export from renderer module

In `src/renderer/mod.rs` (around line 6-17):

```rust
pub mod <name>;
pub use <name>::<Name>;
```

### 3. Add to auto-cycling (optional)

In `Renderer::with_cycling()` at `src/renderer/mod.rs:99-111`:

```rust
Box::new(<Name>::new()),
```

Also update `visualization_name()` function (line 218-233) to include your viz name.

## Creative Patterns for Techno/Psytrance

### Kick Response (Bass Bands 0-1)
```rust
// Pulsing center orb
let pulse_radius = base_radius * (1.0 + self.bass * 0.5);

// Outward burst particles on kicks
if analysis.bass > 0.6 {
    self.spawn_particles(center, analysis.bass);
}

// Tunnel zoom effect
let zoom = 1.0 + self.bass * 0.3;
```

### Hi-Hat Response (Treble Bands 5-7)
```rust
// Sparkle/flicker effects
let brightness = 0.7 + self.treble * 0.3;

// Rapid rotation changes
self.rotation += self.treble * 0.1;

// Particle jitter/chaos
let jitter = rng.random_range(-self.treble..self.treble);
```

### Hypnotic Geometry
```rust
// Spirals (tunnel effect)
for i in 0..NUM_RINGS {
    let t = i as f32 / NUM_RINGS as f32;
    let radius = max_radius * t * zoom;
    let angle = self.rotation + t * TAU * 3.0; // 3 spirals
    let x = center.x + radius * angle.cos();
    let y = center.y + radius * angle.sin();
}

// Kaleidoscope symmetry (2, 3, 6, 8 segments)
for seg in 0..6 {
    let base_angle = seg as f32 * TAU / 6.0 + self.rotation;
    // Mirror/reflect geometry
}

// Infinite zoom/tunnel (use rotation + scale based on energy)
```

### Psychedelic Colors
```rust
// HSV color cycling (0-360 hue)
fn hsv_to_rgb(hue: f32, sat: f32, val: f32) -> (f32, f32, f32) {
    // Convert HSV to RGB (see tesla_coil.rs:165-186)
}

// Map frequency bands to rainbow
let band_hue = (band_idx as f32 / 8.0) * 360.0 + self.hue_offset;

// Energy-reactive saturation/brightness
let saturation = 0.6 + analysis.energy * 0.4;
let brightness = 0.5 + self.bass * 0.5;
```

### GPU Shader Integration (Advanced)

Visualizations are composited with GPU shaders for feedback/burn effects:
- `src/renderer/shaders/feedback.wgsl`: Trail effects via ping-pong buffers
- `src/renderer/shaders/burn_blend.wgsl`: Screen-blend overlays
- See `src/renderer/feedback.rs` for shader pipeline

To use custom shaders, follow the pattern in `FeedbackRenderer`. WGSL shaders can do:
- Distortion fields, chromatic aberration
- Reaction-diffusion patterns
- Fractal zoom effects
- Ray marching for 3D tunnels

## Performance Optimization

```rust
// Debug vs Release particle counts
const NUM_PARTICLES: usize = if cfg!(debug_assertions) { 100 } else { 400 };

// Early culling (skip offscreen rendering)
if x < bounds.left() || x > bounds.right() { continue; }

// Pre-allocate vectors
Vec::with_capacity(NUM_PARTICLES)

// Reuse particle pools instead of spawning
if particle.age > MAX_AGE {
    particle.respawn(); // Reset instead of remove/add
}
```

## Examples & Inspiration

### Minimal spiral tunnel
```
/new-visualization spiral-tunnel
```
Rotating spiral rings that zoom/pulse with bass kicks. Simple but hypnotic.

### Particle nebula
```
/new-visualization particle-nebula
```
Thousands of particles in orbital motion, colored by frequency band, swirling with mid energy.

### Frequency mandala
```
/new-visualization freq-mandala
```
8-fold symmetry where each segment represents a frequency band. Rotates with energy.

### Strobing grid
```
/new-visualization strobe-grid
```
Grid of cells that flash/pulse in sync with kick drums and hi-hats. Disorienting and intense.

## Reference Implementations

Study these for patterns:
- `black_hole.rs` - Particle systems, polar coords, gravitational effects
- `tesla_coil.rs` - HSV colors, velocity tracking, recursive branching
- `kaleidoscope.rs` - Radial symmetry, particle mirroring
- `gravity_flames.rs` - Particle spawning/aging, band-mapped directions
- `solar_beat.rs` - Radial frequency mapping, rotation effects
- `feedback.rs` - GPU shader integration, ping-pong buffers

## Music-Specific Tips

**110-160 BPM**: 60fps means ~24-36 frames per kick at 120 BPM. Your smoothing attack/decay determines responsiveness.

**Afro/Techno**: Emphasize steady 4/4 kick pattern (bass), repetitive patterns, subtle variations
**Psytrance**: Rapid bass line (130-150 BPM), high treble activity, chaotic/organic visuals
**Hypnotic Techno**: Minimal, looping, trance-inducing geometry, slow color shifts

Use `transition_detected` flag to trigger major visual changes on drops/breakdowns!
