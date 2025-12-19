---
name: new-visualization
description: Generate a new visualization module for the dj-viz audio visualizer. Use when adding a new visual effect that responds to audio.
---

# New Visualization

## Instructions

Create a new visualization that implements the `Visualization` trait and integrates with the audio pipeline.

### 1. Create the module file

Create `src/renderer/<name>.rs` with this structure:

```rust
use nannou::prelude::*;
use super::Visualization;

pub struct <Name> {
    // State for rendering (history, computed values, etc.)
}

impl <Name> {
    pub fn new() -> Self {
        Self { /* init state */ }
    }
}

impl Visualization for <Name> {
    fn update(&mut self, samples: &[f32]) {
        // Process audio samples, update internal state
        // samples.len() == 1024 (BUFFER_SIZE)
        // Values typically in range [-1.0, 1.0]
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        // Use nannou's Draw API
        // bounds provides the window dimensions
    }
}
```

### 2. Export from renderer module

In `src/renderer/mod.rs`, add:

```rust
pub mod <name>;
pub use <name>::<Name>;
```

### 3. Add to cycling renderer (optional)

In `Renderer::with_cycling()` at `src/renderer/mod.rs:105`, add to the visualizations vec:

```rust
Box::new(<Name>::new()),
```

### Key Integration Points

- **Audio source** (`src/audio/source_pipe.rs`): `SourcePipe::stream()` returns `Vec<f32>` with 1024 samples
- **Visualization trait** (`src/renderer/mod.rs:15-21`): `update(&mut self, samples: &[f32])` + `draw(&self, draw: &Draw, bounds: Rect)`
- **Renderer** (`src/renderer/mod.rs`): Calls `update()` every frame, auto-cycles visualizations on treble peaks

### Common Patterns

**FFT for frequency analysis:**
```rust
use rustfft::FftPlanner;
use num_complex::Complex;

let mut planner = FftPlanner::new();
let fft = planner.plan_fft_forward(samples.len());
let mut buffer: Vec<Complex<f32>> = samples.iter()
    .map(|&s| Complex::new(s, 0.0))
    .collect();
fft.process(&mut buffer);
```

**Smoothing/decay:**
```rust
self.value = self.value * 0.9 + new_value * 0.1;
```

## Examples

### Basic waveform visualization

```
/new-visualization waveform
```

Creates a simple oscilloscope-style waveform that draws the raw audio samples as a line.

### Frequency bar visualization

```
/new-visualization freq-bars
```

Creates vertical bars representing frequency bins from FFT analysis.

### Reference implementations

- `src/renderer/spectrogram.rs` - FFT history as scrolling heatmap
- `src/renderer/solar_beat.rs` - Radial frequency display with particle effects
