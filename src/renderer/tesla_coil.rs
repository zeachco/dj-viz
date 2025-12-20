use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

/// Maximum number of lightning bolts per frame
const MAX_BOLTS: usize = if cfg!(debug_assertions) { 4 } else { 8 };
/// Segments per lightning bolt
const SEGMENTS_PER_BOLT: usize = if cfg!(debug_assertions) { 12 } else { 20 };
/// Chance of branching at each segment (0-1)
const BRANCH_CHANCE: f32 = 0.25;
/// Number of radial gradient rings for the center orb
const GRADIENT_RINGS: usize = 20;

/// A single lightning bolt segment
#[derive(Clone)]
struct LightningSegment {
    start: Vec2,
    end: Vec2,
    brightness: f32,
    thickness: f32,
}

/// A complete lightning bolt with all its segments
struct LightningBolt {
    segments: Vec<LightningSegment>,
}

pub struct TeslaCoil {
    /// Current bass level for intensity
    bass: f32,
    /// Current mids level
    mids: f32,
    /// Bass velocity (rate of change) for hue
    bass_velocity: f32,
    /// Mids velocity for hue
    mids_velocity: f32,
    /// Previous bass for velocity calculation
    prev_bass: f32,
    /// Previous mids for velocity calculation
    prev_mids: f32,
    /// Accumulated hue offset
    hue_offset: f32,
    /// Current lightning bolts to draw
    bolts: Vec<LightningBolt>,
    /// Frame counter for timing
    frame_count: u32,
    /// Smoothed kick intensity
    kick_intensity: f32,
    /// Treble level for extra effects
    treble: f32,
}

impl TeslaCoil {
    pub fn new() -> Self {
        Self {
            bass: 0.0,
            mids: 0.0,
            bass_velocity: 0.0,
            mids_velocity: 0.0,
            prev_bass: 0.0,
            prev_mids: 0.0,
            hue_offset: 0.0,
            bolts: Vec::new(),
            frame_count: 0,
            kick_intensity: 0.0,
            treble: 0.0,
        }
    }

    /// Generate a lightning bolt from center toward an angle
    fn generate_bolt(&self, center: Vec2, max_radius: f32, angle: f32, intensity: f32) -> LightningBolt {
        let mut rng = rand::rng();
        let mut segments = Vec::new();

        // Main bolt
        self.generate_bolt_segments(
            &mut segments,
            &mut rng,
            center,
            angle,
            max_radius * (0.6 + intensity * 0.4),
            intensity,
            1.0, // full brightness for main bolt
            0,   // depth
        );

        LightningBolt { segments }
    }

    /// Recursively generate lightning segments with branching
    fn generate_bolt_segments(
        &self,
        segments: &mut Vec<LightningSegment>,
        rng: &mut impl Rng,
        start: Vec2,
        base_angle: f32,
        remaining_length: f32,
        intensity: f32,
        brightness_scale: f32,
        depth: usize,
    ) {
        if remaining_length < 5.0 || depth > 4 || segments.len() > 100 {
            return;
        }

        let segment_count = (SEGMENTS_PER_BOLT as f32 * (1.0 - depth as f32 * 0.2)).max(3.0) as usize;
        let segment_length = remaining_length / segment_count as f32;

        let mut current_pos = start;
        let mut current_angle = base_angle;

        for i in 0..segment_count {
            // Add jitter to angle (more for higher treble)
            let jitter = rng.random_range(-0.4..0.4) * (1.0 + self.treble);
            current_angle += jitter;

            // Calculate next position
            let next_pos = current_pos + Vec2::new(
                current_angle.cos() * segment_length,
                current_angle.sin() * segment_length,
            );

            // Brightness fades toward the end, boosted by treble
            let progress = i as f32 / segment_count as f32;
            let segment_brightness = brightness_scale * (1.0 - progress * 0.5) * (0.7 + self.treble * 0.3);

            // Thickness based on intensity and depth
            let thickness = (3.0 + intensity * 4.0) * (1.0 - depth as f32 * 0.2).max(0.3);

            segments.push(LightningSegment {
                start: current_pos,
                end: next_pos,
                brightness: segment_brightness,
                thickness,
            });

            // Maybe branch
            if depth < 3 && rng.random::<f32>() < BRANCH_CHANCE * (1.0 + intensity) {
                let branch_angle = current_angle + rng.random_range(-0.8..0.8);
                let branch_length = remaining_length * rng.random_range(0.3..0.5);
                self.generate_bolt_segments(
                    segments,
                    rng,
                    next_pos,
                    branch_angle,
                    branch_length,
                    intensity * 0.7,
                    brightness_scale * 0.6,
                    depth + 1,
                );
            }

            current_pos = next_pos;
        }
    }

    /// Convert hue (0-360), saturation (0-1), value (0-1) to RGB
    fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> (f32, f32, f32) {
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

        (r1 + m, g1 + m, b1 + m)
    }

    /// Get lightning color based on current audio state
    fn get_lightning_color(&self, brightness: f32) -> Srgba<u8> {
        // Base hue cycles through electric blues/purples/cyans
        // Velocity shifts it toward warmer colors (more energy = more purple/pink)
        let base_hue = 200.0 + self.hue_offset; // Start at electric blue
        let velocity_shift = (self.bass_velocity + self.mids_velocity) * 60.0;
        let hue = (base_hue + velocity_shift) % 360.0;

        // Saturation decreases with brightness (white core)
        let saturation = (1.0 - brightness * 0.6).clamp(0.3, 1.0);

        // Value is the brightness
        let value = brightness.clamp(0.0, 1.0);

        let (r, g, b) = Self::hsv_to_rgb(hue, saturation, value);

        // Alpha for trail blending
        let alpha = (brightness * 200.0) as u8;

        srgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            alpha,
        )
    }

    /// Draw the center orb with radial gradient
    fn draw_center_orb(&self, draw: &Draw, center: Vec2, max_radius: f32) {
        let orb_radius = max_radius * 0.15;

        // Draw gradient rings from outside in
        for i in 0..GRADIENT_RINGS {
            let t = i as f32 / GRADIENT_RINGS as f32;
            let radius = orb_radius * (1.0 - t);

            // Gray gradient: outer is more transparent, inner is brighter
            let gray = 0.3 + t * 0.4 * (0.5 + self.kick_intensity * 0.5);
            let alpha = (0.1 + t * 0.4) * (0.5 + self.kick_intensity * 0.5);

            draw.ellipse()
                .xy(center)
                .radius(radius)
                .color(srgba(
                    (gray * 255.0) as u8,
                    (gray * 255.0) as u8,
                    (gray * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ));
        }

        // Bright core that pulses with kick
        let core_brightness = 0.6 + self.kick_intensity * 0.4;
        let (r, g, b) = Self::hsv_to_rgb(self.hue_offset, 0.3, core_brightness);
        draw.ellipse()
            .xy(center)
            .radius(orb_radius * 0.3)
            .color(srgba(
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                180,
            ));
    }
}

impl Visualization for TeslaCoil {
    fn update(&mut self, analysis: &AudioAnalysis) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Calculate velocities (rate of change)
        self.bass_velocity = (analysis.bass - self.prev_bass).abs();
        self.mids_velocity = (analysis.mids - self.prev_mids).abs();
        self.prev_bass = analysis.bass;
        self.prev_mids = analysis.mids;

        // Smooth tracking of audio levels
        let attack = 0.6;
        let decay = 0.15;

        if analysis.bass > self.bass {
            self.bass = self.bass * (1.0 - attack) + analysis.bass * attack;
        } else {
            self.bass = self.bass * (1.0 - decay) + analysis.bass * decay;
        }

        if analysis.mids > self.mids {
            self.mids = self.mids * (1.0 - attack) + analysis.mids * attack;
        } else {
            self.mids = self.mids * (1.0 - decay) + analysis.mids * decay;
        }

        self.treble = self.treble * 0.8 + analysis.treble * 0.2;

        // Kick intensity combines bass and low-mids
        let kick = (self.bass + analysis.bands[2] * 0.5) / 1.5;
        if kick > self.kick_intensity {
            self.kick_intensity = self.kick_intensity * 0.3 + kick * 0.7;
        } else {
            self.kick_intensity = self.kick_intensity * 0.9 + kick * 0.1;
        }

        // Hue slowly shifts, faster with more velocity
        let velocity_factor = 1.0 + (self.bass_velocity + self.mids_velocity) * 5.0;
        self.hue_offset += 0.5 * velocity_factor;
        if self.hue_offset > 360.0 {
            self.hue_offset -= 360.0;
        }

        // Generate new lightning bolts based on kick intensity
        self.bolts.clear();

        // Only spawn bolts if there's enough energy
        if self.kick_intensity > 0.1 {
            let mut rng = rand::rng();

            // Number of bolts based on intensity
            let num_bolts = ((self.kick_intensity * MAX_BOLTS as f32) as usize).max(1);

            // Use a placeholder for bounds - actual values come from draw
            // We'll regenerate in draw() with proper bounds
            for _ in 0..num_bolts {
                // Store random angles for later
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                // Temporarily store with unit values, regenerate in draw
                let bolt = LightningBolt {
                    segments: vec![LightningSegment {
                        start: Vec2::ZERO,
                        end: Vec2::new(angle, 0.0), // Store angle in x
                        brightness: self.kick_intensity,
                        thickness: 0.0,
                    }],
                };
                self.bolts.push(bolt);
            }
        }
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        let center = bounds.xy();
        let max_radius = bounds.w().min(bounds.h()) / 2.0;

        // Draw center orb first (behind lightning)
        self.draw_center_orb(draw, center, max_radius);

        // Generate and draw lightning bolts
        for bolt in &self.bolts {
            if let Some(first_seg) = bolt.segments.first() {
                // Extract stored angle and intensity
                let angle = first_seg.end.x;
                let intensity = first_seg.brightness;

                // Generate actual bolt with proper bounds
                let actual_bolt = self.generate_bolt(center, max_radius, angle, intensity);

                // Draw all segments
                for segment in &actual_bolt.segments {
                    let color = self.get_lightning_color(segment.brightness);

                    // Main bolt line
                    draw.line()
                        .start(segment.start)
                        .end(segment.end)
                        .weight(segment.thickness)
                        .color(color);

                    // Glow effect (wider, more transparent)
                    let glow_color = srgba(
                        color.red,
                        color.green,
                        color.blue,
                        (color.alpha as f32 * 0.3) as u8,
                    );
                    draw.line()
                        .start(segment.start)
                        .end(segment.end)
                        .weight(segment.thickness * 2.5)
                        .color(glow_color);
                }
            }
        }

        // Add extra bolts on peaks for dramatic effect
        if self.kick_intensity > 0.6 {
            let mut rng = rand::rng();
            let extra_bolts = ((self.kick_intensity - 0.6) * 10.0) as usize;

            for _ in 0..extra_bolts {
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                let bolt = self.generate_bolt(center, max_radius, angle, self.kick_intensity * 0.8);

                for segment in &bolt.segments {
                    let color = self.get_lightning_color(segment.brightness * 0.7);
                    draw.line()
                        .start(segment.start)
                        .end(segment.end)
                        .weight(segment.thickness * 0.7)
                        .color(color);
                }
            }
        }
    }
}
