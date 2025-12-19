# dj-viz

Real-time audio visualizer built with Rust and nannou.

## Download

Pre-built binaries are available from [GitHub Actions](../../actions/workflows/build.yml) - click on the latest successful run and download artifacts for your platform:

- **dj-viz-linux-x86_64** - Linux (x86_64)
- **dj-viz-linux-aarch64** - Raspberry Pi 4+ (64-bit)
- **dj-viz-linux-armv7** - Raspberry Pi 3/Zero (32-bit)

## Build from source

```bash
cargo build --release
```

## Usage

```bash
cargo run --release        # Run fullscreen
cargo run                  # Run in debug window (400x300)
cargo run -- --audio-info  # Print audio device info
```

## Configuration

Audio device selection is saved to `~/.dj-viz.toml`.
