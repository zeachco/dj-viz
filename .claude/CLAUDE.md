# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build                # Debug build
cargo build --release      # Release build
cargo run                  # Run debug (400x300 window)
cargo run --release        # Run release (fullscreen)
cargo run -- --audio-info  # Print audio diagnostics
```

## Architecture

Real-time audio visualizer built with Rust and nannou. Captures audio from system devices and renders FFT-based visualizations.

### Module Organization by I/O Domain

```
src/
├── main.rs          # App lifecycle, input handling, wires layers together
├── audio/           # INPUT: audio capture layer
│   ├── mod.rs
│   └── source_pipe.rs
├── renderer/        # OUTPUT: visualization layer
│   ├── mod.rs
│   ├── spectro_road.rs
│   └── solar_beat.rs
└── utils/           # Shared utilities (config, diagnostics)
```

Future input layers (e.g., webcam) should follow the same pattern as `audio/`.

### Input Layer: `audio/`

- **source_pipe.rs** - Audio capture via cpal. Manages device enumeration, stream building, and a ring buffer (`BUFFER_SIZE=1024`) shared across threads via `Arc<Mutex<Vec<f32>>>`

### Output Layer: `renderer/`

- **mod.rs** - `Renderer` orchestrates visualizations, performs FFT for high-frequency detection, auto-cycles between visualizations when treble peaks
- `Visualization` trait (`renderer/mod.rs:15-21`): `update(&mut self, samples: &[f32])` + `draw(&self, draw: &Draw, bounds: Rect)`
- **spectro_road.rs** - scrolling frequency/time road-like heatmap with FFT history
- **solar_beat.rs** - radial frequency display with particle effects

### Configuration

- `~/.dj-viz.toml` - persists last selected audio device
- `Resolution::current()` uses `cfg!(debug_assertions)` to select window size/fullscreen

### Key Dependencies

- **nannou** - graphics/windowing
- **cpal** - cross-platform audio capture
- **rustfft** - FFT for frequency analysis

## Skills

- `/new-visualization <name>` - Generate a new visualization module (see `.claude/skills/new-visualization.md`)
- `/release` - Create a new release with automated version bumping and GitHub release creation (see `.claude/skills/release.md`)
