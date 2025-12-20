# dj-viz

[![Build](https://github.com/zeachco/dj-viz/actions/workflows/build.yml/badge.svg)](https://github.com/zeachco/dj-viz/actions/workflows/build.yml)

Real-time audio visualizer built with Rust and nannou.

## Download

Pre-built binaries are available for download in [Releases](../../releases) - click on the latest successful run and download artifacts for your platform:

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

## Controls

| Key | Action |
|-----|--------|
| `Space` | Cycle visualization |
| `0-9` | Select audio device (Shift+0-9 for devices 10-19) |
| `/` | Open PipeWire stream search |
| `q` | Exit application |

### PipeWire Stream Search

Press `/` to open the stream search interface for capturing audio from specific applications (like Spotify):

| Key | Action |
|-----|--------|
| Type | Filter streams by name |
| `Backspace` | Delete last character |
| `Up/Down` | Navigate results (cycles) |
| `Enter` | Select and auto-connect |
| `Escape` | Cancel search |

The search shows both output and input PipeWire ports, with outputs listed first. Selected streams are auto-connected via `pw-link` and saved to config.

## Configuration

Audio device and PipeWire stream selections are saved to `~/.dj-viz.toml`.

## Screenshots

| | | |
|:---:|:---:|:---:|
| <img src="assets/preview_1.png" width="250"> | <img src="assets/preview_2.png" width="250"> | <img src="assets/preview_3.png" width="250"> |
| <img src="assets/preview_4.png" width="250"> | <img src="assets/preview_5.png" width="250"> | <img src="assets/preview_6.png" width="250"> |
