# Kick Detector

Multi-band onset detection for kick drums that works across music genres.

## How It Works

Kicks vary widely across genres:
- Electronic music: clean sub-bass kicks
- Rock/metal: punchy mid-frequency kicks
- Jazz/acoustic: softer, more transient kicks
- Hip-hop: 808s with long sustain

The detector uses **coincident onset detection** across 3 frequency bands:

| Band | Frequency | Purpose |
|------|-----------|---------|
| Sub-bass | 20-80 Hz | The fundamental "thump" |
| Low-mid | 80-200 Hz | The "punch" and body |
| Attack | 2-5 kHz | The "click" of the beater/transient |

A kick is detected when **2+ bands** show simultaneous onsets.

## Algorithm

1. **Envelope following** - Track smoothed energy per band with fast attack, slower release
2. **Spectral flux** - Measure positive frame-to-frame energy change (onsets only)
3. **Adaptive threshold** - Moving average per band prevents constant triggering on loud songs
4. **Coincidence detection** - Require multiple bands to agree before triggering
5. **Cooldown** - Minimum interval between detections (default 120ms)

## Usage

```rust
use crate::audio::utils::KickDetector;

// Create detector
let mut kick_detector = KickDetector::new(44100.0, 2048);

// Option 1: Process full spectrum
let kick = kick_detector.process(&spectrum, 1.0 / 60.0);

// Option 2: Process pre-computed bands (faster)
let kick = kick_detector.process_bands(sub_bass, low_mid, attack, dt);

// Query state
kick_detector.kick_detected();   // bool
kick_detector.confidence();      // 0-1
kick_detector.time_since_kick(); // seconds
kick_detector.band_envelopes();  // [f32; 3]
kick_detector.band_flux();       // [f32; 3]
```

## Configuration

```rust
KickDetectorConfig {
    min_kick_interval: 0.12,  // Max ~500 BPM
    onset_threshold: 1.4,     // 40% above moving average
    min_coincident_bands: 2,  // Require 2 of 3 bands
    envelope_attack: 0.8,     // Fast attack
    envelope_release: 0.15,   // Moderate release
}
```

## Exposed to Rhai Scripts

| Variable | Type | Description |
|----------|------|-------------|
| `kick_detected` | bool | Kick detected this frame |
| `kick_confidence` | f64 | Detection confidence (0-1) |
| `kick_time_since` | f64 | Seconds since last kick |
| `kick_envelopes` | [f64; 3] | Band envelopes [sub, lo-mid, attack] |
| `kick_flux` | [f64; 3] | Spectral flux per band |

## Debug Visualization

In `scripts/debug.rhai`:
- **KICK** indicator in boolean column (right side)
- **Band meters** showing SUB/LO-M/ATK envelopes with flux overlay
- **Confidence bar** below the band meters
