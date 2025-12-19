# DJ-Viz Runbook

## Quick Start

```bash
cargo run           # Debug mode (400x300 window)
cargo run --release # Release mode (fullscreen)
```

## Controls

| Key | Action |
|-----|--------|
| `<` (comma) | Previous audio device |
| `>` (period) | Next audio device |
| `0-9` | Select device by index |

## Audio Device Setup

### Finding the Right Device

The visualizer needs an **input** source. To capture system audio (like Spotify), you need a **monitor** source.

### PipeWire / PulseAudio (Linux)

1. **List current audio routing:**
   ```bash
   pactl list sink-inputs short
   ```

2. **List available sources (inputs):**
   ```bash
   pactl list sources short
   ```

3. **Find monitor sources:**
   Look for sources ending in `.monitor` - these capture output audio.

4. **Create a virtual sink for capturing:**
   ```bash
   pactl load-module module-null-sink sink_name=visualizer sink_properties=device.description=Visualizer
   pactl load-module module-loopback source=visualizer.monitor sink=@DEFAULT_SINK@
   ```
   Then select "visualizer" as input in dj-viz.

### JACK (Linux/macOS)

Connect your audio source to dj-viz input using QjackCtl or similar.

### macOS

Use BlackHole or Loopback to create a virtual audio device for capturing system audio.

### Windows

Use VB-Cable or similar virtual audio cable software.

## Troubleshooting

### "Max sample: 0.000000"

- Wrong device selected - cycle through with `>` until you see values
- No audio playing on that device
- Need a monitor/loopback source for output capture

### No devices listed

- Check audio server is running: `systemctl --user status pipewire` or `pulseaudio --check`
- Check permissions for audio devices

### Laggy visualization

- Run in release mode: `cargo run --release`
- Close other GPU-intensive applications

## Configuration

Config is stored in `~/.dj-viz.toml`:

```toml
last_device = "pipewire"
last_device_is_input = false
```

## Debug Commands

Run with audio system diagnostics:
```bash
cargo run -- --audio-info
```
