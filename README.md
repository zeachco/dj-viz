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
| `/` | Open device search |
| `q` | Exit application |

### Device Search

Press `/` to open an interactive device search interface:

| Key | Action |
|-----|--------|
| Type | Filter devices by name |
| `Backspace` | Delete last character |
| `Up/Down` | Navigate results |
| `Enter` | Select device |
| `Escape` | Cancel search |

### macOS Audio Capture Setup

**⚠️ macOS Disclaimer:** Unlike Linux, macOS does not natively support capturing system audio (loopback). You must install a virtual audio device:

```bash
brew install blackhole-2ch
```

After installation, **restart the Core Audio service** to make BlackHole visible:

```bash
sudo killall coreaudiod
```

Core Audio will automatically restart in a few seconds. If BlackHole still doesn't appear, restart your Mac.

Then create a **Multi-Output Device** in Audio MIDI Setup:
1. Open `/Applications/Utilities/Audio MIDI Setup.app`
2. Click `+` → "Create Multi-Output Device"
3. Check both your speakers/headphones AND BlackHole 2ch
4. Set your speakers as primary (checkmark on the left)
5. Set this Multi-Output Device as your system default output

Now dj-viz can capture system audio by selecting BlackHole as the input device (press `/` and search for "blackhole").

### PipeWire Stream Search (Linux)

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
