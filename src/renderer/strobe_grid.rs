//! Strobing grid visualization.
//!
//! Grid of cells that flash/pulse in sync with kick drums and hi-hats.
//! Disorienting and intense effect for high-energy techno.

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

/// Grid size
const GRID_SIZE: usize = if cfg!(debug_assertions) { 8 } else { 12 };

#[derive(Clone)]
struct Cell {
    /// Current brightness (0-1)
    brightness: f32,
    /// Target brightness
    target_brightness: f32,
    /// Base hue
    hue: f32,
    /// Which frequency band triggers this cell
    band_idx: usize,
    /// Flash cooldown
    cooldown: u32,
}

pub struct StrobeGrid {
    /// Grid of cells
    cells: Vec<Vec<Cell>>,
    /// Global strobe intensity from bass
    strobe_intensity: f32,
    /// Hi-hat flicker intensity
    flicker: f32,
    /// Hue offset for color cycling
    hue_offset: f32,
    /// Smoothed bass
    bass: f32,
    /// Smoothed treble
    treble: f32,
    /// Frame counter
    frame_count: u32,
    /// Wave phase for ripple effects
    wave_phase: f32,
}

impl StrobeGrid {
    pub fn new() -> Self {
        let mut rng = rand::rng();

        let cells: Vec<Vec<Cell>> = (0..GRID_SIZE)
            .map(|y| {
                (0..GRID_SIZE)
                    .map(|x| {
                        // Assign bands in a pattern (center = bass, edges = treble)
                        let dist_from_center = {
                            let cx = (GRID_SIZE / 2) as f32;
                            let cy = (GRID_SIZE / 2) as f32;
                            let dx = x as f32 - cx;
                            let dy = y as f32 - cy;
                            (dx * dx + dy * dy).sqrt() / cx
                        };
                        let band_idx = (dist_from_center * 7.0).min(7.0) as usize;

                        Cell {
                            brightness: 0.0,
                            target_brightness: 0.0,
                            hue: rng.random_range(0.0..360.0),
                            band_idx,
                            cooldown: 0,
                        }
                    })
                    .collect()
            })
            .collect();

        Self {
            cells,
            strobe_intensity: 0.0,
            flicker: 0.0,
            hue_offset: 0.0,
            bass: 0.0,
            treble: 0.0,
            frame_count: 0,
            wave_phase: 0.0,
        }
    }

    fn hsv_to_rgba(hue: f32, saturation: f32, value: f32, alpha: f32) -> Srgba<u8> {
        let hue = hue % 360.0;
        let c = value * saturation;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = value - c;

        let (r1, g1, b1) = if hue < 60.0 {
            (c, x, 0.0)
        } else if hue < 120.0 {
            (x, c, 0.0)
        } else if hue < 180.0 {
            (0.0, c, x)
        } else if hue < 240.0 {
            (0.0, x, c)
        } else if hue < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        srgba(
            ((r1 + m) * 255.0) as u8,
            ((g1 + m) * 255.0) as u8,
            ((b1 + m) * 255.0) as u8,
            (alpha * 255.0) as u8,
        )
    }
}

impl Visualization for StrobeGrid {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);
        let mut rng = rand::rng();

        // Fast attack, moderate decay
        let attack = 0.8;
        let decay = 0.2;

        if analysis.bass > self.bass {
            self.bass = self.bass * (1.0 - attack) + analysis.bass * attack;
        } else {
            self.bass = self.bass * (1.0 - decay) + analysis.bass * decay;
        }

        self.treble = self.treble * 0.7 + analysis.treble * 0.3;

        // Strobe effect on bass hits
        if analysis.bass > 0.5 {
            self.strobe_intensity = 1.0;
        } else {
            self.strobe_intensity *= 0.85;
        }

        // Hi-hat flicker
        self.flicker = self.treble;

        // Wave phase for ripple
        self.wave_phase += 0.1 + analysis.energy * 0.1;

        // Color cycling
        self.hue_offset += 0.3 + analysis.energy * 1.5;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }

        // Update cells
        for (y, row) in self.cells.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                // Decrease cooldown
                if cell.cooldown > 0 {
                    cell.cooldown -= 1;
                }

                // Calculate distance for wave effect
                let cx = GRID_SIZE as f32 / 2.0;
                let cy = GRID_SIZE as f32 / 2.0;
                let dist = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                let wave = (dist * 0.5 - self.wave_phase).sin() * 0.5 + 0.5;

                // Get band energy for this cell
                let band_energy = analysis.bands_normalized[cell.band_idx];

                // Trigger flash on high energy for matching band
                if band_energy > 0.5 && cell.cooldown == 0 {
                    cell.target_brightness = band_energy;
                    cell.cooldown = rng.random_range(3..8);
                    cell.hue = (cell.band_idx as f32 / 8.0 * 360.0 + self.hue_offset) % 360.0;
                }

                // Add strobe effect from bass
                let strobe_contribution = self.strobe_intensity * wave * 0.5;

                // Add flicker from treble
                let flicker_contribution = if cell.band_idx >= 5 {
                    self.flicker * rng.random_range(0.0..1.0) * 0.3
                } else {
                    0.0
                };

                cell.target_brightness = (cell.target_brightness + strobe_contribution + flicker_contribution).min(1.0);

                // Smooth brightness transition
                cell.brightness = cell.brightness * 0.8 + cell.target_brightness * 0.2;
                cell.target_brightness *= 0.9; // Decay target
            }
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let cell_width = bounds.w() / GRID_SIZE as f32;
        let cell_height = bounds.h() / GRID_SIZE as f32;
        let gap = 2.0;

        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let px = bounds.left() + x as f32 * cell_width + cell_width / 2.0;
                let py = bounds.bottom() + y as f32 * cell_height + cell_height / 2.0;

                if cell.brightness > 0.01 {
                    let hue = (cell.hue + self.hue_offset * 0.2) % 360.0;
                    let saturation = 0.7 + cell.brightness * 0.3;
                    let value = cell.brightness;
                    let alpha = 0.3 + cell.brightness * 0.7;

                    // Draw glow
                    if cell.brightness > 0.3 {
                        let glow_size = cell_width.min(cell_height) - gap + cell.brightness * 10.0;
                        let glow_alpha = (cell.brightness - 0.3) * 0.3;
                        let glow_color = Self::hsv_to_rgba(hue, saturation * 0.5, value, glow_alpha);

                        draw.rect()
                            .x_y(px, py)
                            .w_h(glow_size, glow_size)
                            .color(glow_color);
                    }

                    // Draw cell
                    let color = Self::hsv_to_rgba(hue, saturation, value, alpha);
                    draw.rect()
                        .x_y(px, py)
                        .w_h(cell_width - gap, cell_height - gap)
                        .color(color);

                    // Bright highlight on high intensity
                    if cell.brightness > 0.7 {
                        let highlight_alpha = (cell.brightness - 0.7) * 1.5;
                        draw.rect()
                            .x_y(px, py)
                            .w_h(cell_width - gap - 4.0, cell_height - gap - 4.0)
                            .color(srgba(255, 255, 255, (highlight_alpha * 100.0) as u8));
                    }
                } else {
                    // Dim cell outline
                    draw.rect()
                        .x_y(px, py)
                        .w_h(cell_width - gap, cell_height - gap)
                        .no_fill()
                        .stroke_weight(0.5)
                        .stroke(srgba(50u8, 50u8, 60u8, 100u8));
                }
            }
        }

        // Border flash on bass
        if self.strobe_intensity > 0.3 {
            let border_alpha = (self.strobe_intensity - 0.3) * 0.5;
            let hue = self.hue_offset;
            let color = Self::hsv_to_rgba(hue, 0.8, 0.9, border_alpha);

            draw.rect()
                .xy(bounds.xy())
                .wh(bounds.wh())
                .no_fill()
                .stroke_weight(4.0 + self.strobe_intensity * 6.0)
                .stroke(color);
        }
    }
}
