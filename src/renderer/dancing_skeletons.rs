//! Dancing skeletons visualization.
//!
//! Animated skeletons traverse the screen diagonally with the following behaviors:
//! - Maximum of 6 skeletons on screen at any time
//! - Spawn at screen edges and move in random diagonal directions
//! - Removed when they exit the viewport
//! - Random scales (0.6-2.5x) for variety in size
//! - Random rotation (-30° to +30°) for tilted poses
//! - Facial features: two eyes, upside-down heart nose, optional smile (50% chance)
//! - Bone color assigned based on dominant frequency band at spawn time:
//!   - Band 0-1 (Bass): Red/Orange
//!   - Band 2-4 (Mids): Yellow/Green
//!   - Band 5-7 (Treble): Cyan/Blue/Purple
//! - Five dance styles:
//!   - Jumping Jacks: Front view with arms and legs swinging out
//!   - Jumping Jacks Overhead: Front view with arms going overhead
//!   - Dougie: Front view with side-to-side sway
//!   - Shuffle: Side view with bass-driven foot steps (Band 0 or 1 > 0.7, 0.4s cooldown)
//!   - Shuffle Mirrored: Same as shuffle but horizontally flipped
//! - Dance style is randomly assigned at spawn
//! - Animation speed is affected by energy_diff (higher energy = faster dancing)
//! - Shuffle steps occur only on beat hits, creating rhythmic foot movements
//! - Shuffle skeletons always move along screen edges (corner to adjacent corner)
//! - Edge shufflers: feet point towards edge, head points towards middle of screen
//! - Shuffler velocity modulated by energy (0.5x-2.5x speed based on music intensity)
//! - Hue-shifted outlines (90° shift) create complementary color pairs
//! - Radial shadow behind each skeleton for depth effect

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;
use crate::utils::get_crossing_path;

const MAX_SKELETONS: usize = 6;
const SPAWN_AREA_WIDTH: f32 = 800.0; // Assumed spawn area width
const SPAWN_AREA_HEIGHT: f32 = 600.0; // Assumed spawn area height
const BASE_EDGE_OFFSET: f32 = 20.0; // Base distance outside viewport
                                    // Skeleton height calculation:
                                    // Head top: ~33.0 * scale (25.0 head_y + 8.0 radius)
                                    // Legs bottom: ~-30.0 * scale (-10.0 pelvis + -20.0 legs)
                                    // Body animation adds up to ~15.0 * scale
                                    // Total: ~78.0 * scale (using 80.0 for safety)
const SKELETON_HEIGHT_FACTOR: f32 = 80.0;
// Shuffle dance constants
const SHUFFLE_COOLDOWN_FRAMES: u32 = 30; // 0.5 seconds at 60fps
const BASS_THRESHOLD: f32 = 0.9; // Band 0 or Band 1 must exceed this value (lowered for easier triggering)

#[derive(Clone, Copy, Debug)]
enum DanceStyle {
    JumpingJacks,
    JumpingJacksOverhead, // Arms go overhead
    Dougie,
    Shuffle,         // Side view shuffle/foot dance
    ShuffleMirrored, // Side view shuffle mirrored (y-axis flip)
}

struct Skeleton {
    position: Vec2,
    velocity: Vec2,      // Movement direction and speed
    base_velocity: Vec2, // Base velocity (for edge shufflers to modify with energy)
    scale: f32,
    rotation: f32, // Rotation in radians
    hp: i32,
    dance_style: DanceStyle,
    animation_phase: f32,
    has_smile: bool,
    bone_color: Rgb<u8>,    // Color based on dominant frequency band at spawn
    is_edge_shuffler: bool, // If true, shuffles along screen edges
    // Shuffle-specific state
    shuffle_step: i32, // Which foot is forward: -1 = left, 1 = right, 0 = center
    shuffle_transition: f32, // Smooth transition between steps (0.0 to 1.0)
    shuffle_cooldown: u32, // Frames until next step is allowed (0.4s = 24 frames at 60fps)
}

impl Skeleton {
    fn new(
        position: Vec2,
        velocity: Vec2,
        scale: f32,
        dominant_band: usize,
        rotation: f32,
        is_edge_shuffler: bool,
    ) -> Self {
        let mut rng = rand::rng();

        // Random HP as power of 4: 4^1=4, 4^2=16, 4^3=64, 4^4=256
        let power = rng.random_range(1..=4);
        let hp = 4_i32.pow(power);

        // Random dance style (equal chance for all five)
        let dance_style = match rng.random_range(0..5) {
            0 => DanceStyle::JumpingJacks,
            1 => DanceStyle::JumpingJacksOverhead,
            2 => DanceStyle::Dougie,
            3 => DanceStyle::Shuffle,
            _ => DanceStyle::ShuffleMirrored,
        };

        // 50% chance to have a smile
        let has_smile = rng.random();

        // Assign color based on dominant frequency band
        let bone_color = Self::band_to_color(dominant_band);

        Self {
            position,
            velocity,
            base_velocity: velocity, // Store base velocity
            scale,
            rotation,
            hp,
            dance_style,
            animation_phase: rng.random_range(0.0..std::f32::consts::TAU),
            has_smile,
            bone_color,
            is_edge_shuffler,
            shuffle_step: 0,
            shuffle_transition: 1.0,
            shuffle_cooldown: 0,
        }
    }

    /// Maps frequency band index to a color
    /// Bands 0-7: Bass (red) -> Mids (green/yellow) -> Treble (blue/cyan)
    fn band_to_color(band: usize) -> Rgb<u8> {
        match band {
            0 => rgb(255, 50, 50),   // Deep bass - Red
            1 => rgb(255, 120, 50),  // Bass - Orange
            2 => rgb(255, 200, 50),  // Low mids - Yellow-Orange
            3 => rgb(200, 255, 50),  // Mids - Yellow-Green
            4 => rgb(50, 255, 100),  // High mids - Green
            5 => rgb(50, 200, 255),  // Low treble - Cyan
            6 => rgb(100, 100, 255), // Treble - Blue
            7 => rgb(200, 100, 255), // High treble - Purple
            _ => rgb(255, 255, 255), // Fallback - White
        }
    }

    /// Shifts the hue of an RGB color by a given amount (0.0 to 1.0 = full circle)
    fn shift_hue(color: Rgb<u8>, shift: f32) -> Rgb<u8> {
        // Convert RGB to HSV
        let r = color.red as f32 / 255.0;
        let g = color.green as f32 / 255.0;
        let b = color.blue as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        // Calculate hue
        let mut h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        if h < 0.0 {
            h += 360.0;
        }

        // Calculate saturation
        let s = if max == 0.0 { 0.0 } else { delta / max };

        // Value
        let v = max;

        // Apply hue shift (0.4 of full circle = 144 degrees)
        h = (h + shift * 360.0) % 360.0;

        // Convert HSV back to RGB
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r_prime, g_prime, b_prime) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        rgb(
            ((r_prime + m) * 255.0) as u8,
            ((g_prime + m) * 255.0) as u8,
            ((b_prime + m) * 255.0) as u8,
        )
    }

    fn get_outline_color(&self) -> Rgb<u8> {
        Self::shift_hue(self.bone_color, 0.5) // 90 degrees = 0.25 of full circle
    }

    fn update(&mut self, analysis: &AudioAnalysis) {
        // Lose HP when energy spikes (kept for potential future use)
        if analysis.energy_diff > 0.2 {
            self.hp -= 1;
        }

        // For edge shufflers, modulate velocity based on energy
        if self.is_edge_shuffler {
            // Scale velocity by energy: 0.5x to 2.5x based on energy (0.0 to 1.0)
            let energy_scale = 0.1 + analysis.energy * 1.0;
            self.velocity = self.base_velocity * energy_scale;
        }

        // Update position based on velocity
        self.position += self.velocity;

        // Update animation phase based on energy_diff
        // Higher energy_diff = faster dancing
        let speed = 0.05 + analysis.energy_diff.abs() * 0.2;
        self.animation_phase += speed;
        if self.animation_phase > std::f32::consts::TAU {
            self.animation_phase -= std::f32::consts::TAU;
        }

        // Shuffle-specific beat detection and stepping
        if matches!(self.dance_style, DanceStyle::Shuffle) {
            // Decrement cooldown
            if self.shuffle_cooldown > 0 {
                self.shuffle_cooldown -= 1;
            }

            // Detect bass hit (Band 0 or Band 1 exceeds threshold)
            let bass_hit = (analysis.bands[0] > BASS_THRESHOLD
                || analysis.bands[1] > BASS_THRESHOLD)
                && self.shuffle_cooldown == 0;

            if bass_hit {
                // Alternate steps on each bass hit
                self.shuffle_step = match self.shuffle_step {
                    -1 => 1, // Left -> Right
                    1 => -1, // Right -> Left
                    _ => 1,  // Center -> Right (start)
                };
                self.shuffle_transition = 0.0; // Reset transition
                self.shuffle_cooldown = SHUFFLE_COOLDOWN_FRAMES; // Set cooldown
            }

            // Smooth transition between steps
            if self.shuffle_transition < 1.0 {
                self.shuffle_transition += 0.08; // Transition speed (slower for smoother movement)
                if self.shuffle_transition > 1.0 {
                    self.shuffle_transition = 1.0;
                }
            }
        }
    }

    fn is_in_bounds(&self, bounds: Rect) -> bool {
        // Edge shufflers need much larger margin since they move along corners
        let margin = if self.is_edge_shuffler {
            // Need to account for full diagonal distance from corner
            let half_width = SPAWN_AREA_WIDTH / 2.0;
            let half_height = SPAWN_AREA_HEIGHT / 2.0;
            half_width + half_height // Large enough to contain corner-to-corner movement
        } else {
            BASE_EDGE_OFFSET + (SKELETON_HEIGHT_FACTOR * self.scale)
        };

        self.position.x > bounds.left() - margin
            && self.position.x < bounds.right() + margin
            && self.position.y > bounds.bottom() - margin
            && self.position.y < bounds.top() + margin
    }

    fn draw(&self, draw: &Draw) {
        let x = self.position.x;
        let y = self.position.y;
        let scale = self.scale;

        // For edge shufflers, rotate around the skeleton's position (not origin)
        // For others, use the old behavior (rotate around origin for the tilted effect)
        let draw = if self.is_edge_shuffler {
            // Translate to position, then rotate, so rotation happens around skeleton's center
            draw.x_y(x, y).rotate(self.rotation)
        } else {
            // Non-edge shufflers: apply rotation around origin first
            draw.rotate(self.rotation)
        };

        // For edge shufflers, we've already translated to (x, y), so draw at origin
        // For others, use the original position
        let (draw_x, draw_y) = if self.is_edge_shuffler {
            (0.0, 0.0)
        } else {
            (x, y)
        };

        // Calculate animation offsets based on dance style
        match self.dance_style {
            DanceStyle::JumpingJacks => {
                let (arm_offset, leg_offset, body_offset) = self.jumping_jacks_offsets();
                self.draw_skeleton_parts(
                    &draw,
                    draw_x,
                    draw_y,
                    scale,
                    arm_offset,
                    leg_offset,
                    body_offset,
                );
                let head_y = draw_y + body_offset + 25.0 * scale;
                self.draw_face(&draw, draw_x, head_y, scale);
            }
            DanceStyle::JumpingJacksOverhead => {
                let (arm_offset, leg_offset, body_offset) = self.jumping_jacks_overhead_offsets();
                self.draw_skeleton_parts_overhead(
                    &draw,
                    draw_x,
                    draw_y,
                    scale,
                    arm_offset,
                    leg_offset,
                    body_offset,
                );
                let head_y = draw_y + body_offset + 25.0 * scale;
                self.draw_face(&draw, draw_x, head_y, scale);
            }
            DanceStyle::Dougie => {
                let (arm_offset, leg_offset, body_offset) = self.dougie_offsets();
                self.draw_skeleton_parts(
                    &draw,
                    draw_x,
                    draw_y,
                    scale,
                    arm_offset,
                    leg_offset,
                    body_offset,
                );
                let head_y = draw_y + body_offset + 25.0 * scale;
                self.draw_face(&draw, draw_x, head_y, scale);
            }
            DanceStyle::Shuffle => {
                // Shuffle is a side view, so we use a different drawing approach
                self.draw_shuffle(&draw, draw_x, draw_y, scale, false);
            }
            DanceStyle::ShuffleMirrored => {
                // Mirrored shuffle (y-axis flip)
                self.draw_shuffle(&draw, draw_x, draw_y, scale, true);
            }
        }
    }

    fn jumping_jacks_offsets(&self) -> (f32, f32, f32) {
        let t = self.animation_phase;

        // Arms swing out and in
        let arm_angle = (t.sin() * 0.5 + 0.5) * 60.0;

        // Legs swing out and in (opposite to arms)
        let leg_angle = ((t + std::f32::consts::PI).sin() * 0.5 + 0.5) * 30.0;

        // Body bobs up and down
        let body_y = t.sin().abs() * 10.0;

        (arm_angle, leg_angle, body_y)
    }

    fn jumping_jacks_overhead_offsets(&self) -> (f32, f32, f32) {
        let t = self.animation_phase;

        // Arms swing up and down (overhead variation)
        // 0 = arms at sides, 90 = arms overhead
        let arm_angle = (t.sin() * 0.5 + 0.5) * 90.0;

        // Legs swing out and in (opposite to arms)
        let leg_angle = ((t + std::f32::consts::PI).sin() * 0.5 + 0.5) * 30.0;

        // Body bobs up and down
        let body_y = t.sin().abs() * 10.0;

        (arm_angle, leg_angle, body_y)
    }

    fn dougie_offsets(&self) -> (f32, f32, f32) {
        let t = self.animation_phase;

        // Arms sway side to side with offset
        let arm_angle = (t * 1.5).sin() * 40.0;

        // Legs do a subtle step
        let leg_angle = ((t * 2.0).sin() * 0.5 + 0.5) * 15.0;

        // Body leans side to side
        let body_y = (t * 0.5).sin() * 15.0;

        (arm_angle, leg_angle, body_y)
    }

    fn draw_shuffle(&self, draw: &Draw, x: f32, y: f32, scale: f32, mirrored: bool) {
        let bone_color = self.bone_color;
        let outline_color = self.get_outline_color();
        let thickness = 3.0 * scale;
        let outline_thickness = thickness + 2.0 * scale;

        // Body bob on step transitions (subtle)
        let bob_amount = if self.shuffle_transition < 0.5 {
            self.shuffle_transition * 2.0 // Going up
        } else {
            2.0 - (self.shuffle_transition * 2.0) // Coming down
        };
        let body_y = y + bob_amount * 3.0 * scale;

        // Head position (side view - circular)
        let head_y = body_y + 25.0 * scale;
        let head_radius = 8.0 * scale;

        // Spine (slightly tilted forward for shuffle posture)
        let spine_top = head_y - head_radius;
        let spine_bottom = body_y - 10.0 * scale;
        let spine_tilt = if mirrored { -3.0 * scale } else { 3.0 * scale };

        // Shoulders (side view - just a line depth)
        let shoulder_y = spine_top - 5.0 * scale;
        let shoulder_depth = 4.0 * scale;

        // Pelvis
        let pelvis_y = spine_bottom;
        let pelvis_depth = 5.0 * scale;

        // Arm swing based on which foot is forward (slower, more deliberate)
        let arm_swing = self.shuffle_step as f32 * 15.0 * scale * self.shuffle_transition;
        let arm_length = 18.0 * scale;

        // Leg shuffle motion - discrete steps on beats
        let max_step_offset = 12.0 * scale;

        // Calculate foot positions based on current step with smooth transition
        let (left_foot_offset, right_foot_offset) = match self.shuffle_step {
            -1 => {
                // Left foot forward
                let left_offset = max_step_offset * self.shuffle_transition;
                let right_offset = 0.0;
                (left_offset, right_offset)
            }
            1 => {
                // Right foot forward
                let left_offset = 0.0;
                let right_offset = max_step_offset * self.shuffle_transition;
                (left_offset, right_offset)
            }
            _ => {
                // Centered (both feet back)
                (0.0, 0.0)
            }
        };

        let leg_length = 20.0 * scale;

        // Apply mirror multiplier for x-coordinates
        let mirror = if mirrored { -1.0 } else { 1.0 };

        // Draw black outlines first

        // Head outline
        draw.ellipse()
            .x_y(x, head_y)
            .radius(head_radius + 1.5 * scale)
            .color(outline_color);

        // Spine outline
        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x + spine_tilt * mirror, spine_bottom))
            .weight(outline_thickness)
            .color(outline_color);

        // Back arm outline (behind body)
        draw.line()
            .start(pt2(x + shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (shoulder_depth - arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        // Front arm outline (in front of body)
        draw.line()
            .start(pt2(x - shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (-shoulder_depth + arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        // Back leg outline
        draw.line()
            .start(pt2(x + pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (pelvis_depth + right_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        // Front leg outline
        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (-pelvis_depth + left_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        // Pelvis outline (side view - horizontal line)
        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(x + pelvis_depth * mirror, pelvis_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Now draw bones on top

        // Head
        draw.ellipse()
            .x_y(x, head_y)
            .radius(head_radius)
            .color(bone_color);

        // Spine
        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x + spine_tilt * mirror, spine_bottom))
            .weight(thickness)
            .color(bone_color);

        // Back arm (behind body)
        draw.line()
            .start(pt2(x + shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (shoulder_depth - arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(thickness)
            .color(bone_color);

        // Front arm (in front of body)
        draw.line()
            .start(pt2(x - shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (-shoulder_depth + arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(thickness)
            .color(bone_color);

        // Back leg
        draw.line()
            .start(pt2(x + pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (pelvis_depth + right_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(thickness)
            .color(bone_color);

        // Front leg
        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (-pelvis_depth + left_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(thickness)
            .color(bone_color);

        // Pelvis (side view - horizontal line)
        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(x + pelvis_depth * mirror, pelvis_y))
            .weight(thickness)
            .color(bone_color);

        // Draw side-view face features
        self.draw_side_face(draw, x, head_y, scale, mirrored);
    }

    fn draw_skeleton_parts_overhead(
        &self,
        draw: &Draw,
        x: f32,
        y: f32,
        scale: f32,
        arm_angle: f32,
        leg_angle: f32,
        body_y: f32,
    ) {
        let bone_color = self.bone_color;
        let outline_color = self.get_outline_color();
        let thickness = 3.0 * scale;
        let outline_thickness = thickness + 2.0 * scale;

        // Calculate all positions first
        let head_y = y + body_y + 25.0 * scale;
        let spine_top = head_y - 8.0 * scale;
        let spine_bottom = y + body_y - 10.0 * scale;
        let shoulder_y = spine_top - 5.0 * scale;
        let shoulder_width = 15.0 * scale;
        let pelvis_y = spine_bottom;
        let pelvis_width = 12.0 * scale;

        // Overhead arms calculation
        // arm_angle: 0 = arms at sides, 90 = arms overhead
        let arm_rad = arm_angle.to_radians();
        let arm_length = 18.0 * scale;

        // Arms rotate upward from shoulders
        // At 0°: horizontal (out to sides)
        // At 90°: vertical (straight up overhead)
        let left_arm_end_x = x - shoulder_width + (arm_rad.cos() - 1.0) * shoulder_width;
        let left_arm_end_y = shoulder_y + arm_rad.sin() * arm_length;
        let right_arm_end_x = x + shoulder_width - (arm_rad.cos() - 1.0) * shoulder_width;
        let right_arm_end_y = shoulder_y + arm_rad.sin() * arm_length;

        let leg_rad = leg_angle.to_radians();
        let leg_length = 20.0 * scale;
        let left_leg_end_x = x - pelvis_width - leg_rad.sin() * leg_length;
        let left_leg_end_y = pelvis_y - leg_length;
        let right_leg_end_x = x + pelvis_width + leg_rad.sin() * leg_length;
        let right_leg_end_y = pelvis_y - leg_length;

        // Draw black outlines first

        // Head outline
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale + 1.5 * scale)
            .color(outline_color);

        // Spine outline
        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(outline_thickness)
            .color(outline_color);

        // Shoulders outline
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Left arm outline
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Right arm outline
        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Pelvis outline
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Left leg outline
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Right leg outline
        draw.line()
            .start(pt2(x + pelvis_width, pelvis_y))
            .end(pt2(right_leg_end_x, right_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Now draw white bones on top

        // Head
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale)
            .color(bone_color);

        // Spine
        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(thickness)
            .color(bone_color);

        // Shoulders
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(thickness)
            .color(bone_color);

        // Left arm
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        // Right arm
        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        // Pelvis
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(thickness)
            .color(bone_color);

        // Left leg
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(thickness)
            .color(bone_color);

        // Right leg
        draw.line()
            .start(pt2(x + pelvis_width, pelvis_y))
            .end(pt2(right_leg_end_x, right_leg_end_y))
            .weight(thickness)
            .color(bone_color);
    }

    fn draw_skeleton_parts(
        &self,
        draw: &Draw,
        x: f32,
        y: f32,
        scale: f32,
        arm_angle: f32,
        leg_angle: f32,
        body_y: f32,
    ) {
        let bone_color = self.bone_color;
        let outline_color = self.get_outline_color();
        let thickness = 3.0 * scale;
        let outline_thickness = thickness + 2.0 * scale;

        // Calculate all positions first
        let head_y = y + body_y + 25.0 * scale;
        let spine_top = head_y - 8.0 * scale;
        let spine_bottom = y + body_y - 10.0 * scale;
        let shoulder_y = spine_top - 5.0 * scale;
        let shoulder_width = 15.0 * scale;
        let pelvis_y = spine_bottom;
        let pelvis_width = 12.0 * scale;

        let arm_rad = arm_angle.to_radians();
        let arm_length = 18.0 * scale;
        let left_arm_end_x = x - shoulder_width - arm_rad.sin() * arm_length;
        let left_arm_end_y = shoulder_y - arm_rad.cos() * arm_length;
        let right_arm_end_x = x + shoulder_width + arm_rad.sin() * arm_length;
        let right_arm_end_y = shoulder_y - arm_rad.cos() * arm_length;

        let leg_rad = leg_angle.to_radians();
        let leg_length = 20.0 * scale;
        let left_leg_end_x = x - pelvis_width - leg_rad.sin() * leg_length;
        let left_leg_end_y = pelvis_y - leg_length;
        let right_leg_end_x = x + pelvis_width + leg_rad.sin() * leg_length;
        let right_leg_end_y = pelvis_y - leg_length;

        // Draw black outlines first

        // Head outline
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale + 1.5 * scale)
            .color(outline_color);

        // Spine outline
        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(outline_thickness)
            .color(outline_color);

        // Shoulders outline
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Left arm outline
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Right arm outline
        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Pelvis outline
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Left leg outline
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Right leg outline
        draw.line()
            .start(pt2(x + pelvis_width, pelvis_y))
            .end(pt2(right_leg_end_x, right_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Now draw white bones on top

        // Head
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale)
            .color(bone_color);

        // Spine
        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(thickness)
            .color(bone_color);

        // Shoulders
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(thickness)
            .color(bone_color);

        // Left arm
        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        // Right arm
        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        // Pelvis
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(thickness)
            .color(bone_color);

        // Left leg
        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(thickness)
            .color(bone_color);

        // Right leg
        draw.line()
            .start(pt2(x + pelvis_width, pelvis_y))
            .end(pt2(right_leg_end_x, right_leg_end_y))
            .weight(thickness)
            .color(bone_color);
    }

    fn draw_face(&self, draw: &Draw, x: f32, head_y: f32, scale: f32) {
        let eye_size = 2.0 * scale;
        let eye_spacing = 3.5 * scale;

        // Draw eyes (two black circles)
        // Left eye
        draw.ellipse()
            .x_y(x - eye_spacing, head_y + 2.0 * scale)
            .radius(eye_size)
            .color(BLACK);

        // Right eye
        draw.ellipse()
            .x_y(x + eye_spacing, head_y + 2.0 * scale)
            .radius(eye_size)
            .color(BLACK);

        // Draw upside-down heart nose
        self.draw_upside_down_heart_nose(draw, x, head_y, scale);

        // Draw smile if this skeleton has one
        if self.has_smile {
            self.draw_smile(draw, x, head_y, scale);
        }
    }

    fn draw_upside_down_heart_nose(&self, draw: &Draw, x: f32, head_y: f32, scale: f32) {
        let nose_size = 2.5 * scale;
        let nose_y = head_y - 1.0 * scale;

        // Upside-down heart: point at top, bumps at bottom
        let points = vec![
            // Top point
            pt2(x, nose_y + nose_size),
            // Left side going down
            pt2(x - nose_size * 0.5, nose_y + nose_size * 0.3),
            // Left bump
            pt2(x - nose_size * 0.7, nose_y - nose_size * 0.3),
            pt2(x - nose_size * 0.5, nose_y - nose_size * 0.6),
            // Bottom center
            pt2(x, nose_y - nose_size * 0.5),
            // Right bump
            pt2(x + nose_size * 0.5, nose_y - nose_size * 0.6),
            pt2(x + nose_size * 0.7, nose_y - nose_size * 0.3),
            // Right side going up
            pt2(x + nose_size * 0.5, nose_y + nose_size * 0.3),
            // Close to top point
            pt2(x, nose_y + nose_size),
        ];

        // Draw outline
        draw.polyline()
            .weight(2.0 * scale)
            .points(points.clone())
            .color(BLACK);

        // Fill
        draw.polygon().points(points).color(BLACK);
    }

    fn draw_smile(&self, draw: &Draw, x: f32, head_y: f32, scale: f32) {
        let smile_width = 6.0 * scale;
        let smile_y = head_y - 5.0 * scale;
        let smile_curve = 2.0 * scale;

        // Draw smile as a curved arc using points
        let num_points = 10;
        let mut points = Vec::new();

        for i in 0..=num_points {
            let t = i as f32 / num_points as f32;
            let smile_x = x + (t - 0.5) * smile_width * 2.0;
            // Parabolic curve for smile
            let curve_y = smile_curve * (1.0 - (2.0 * t - 1.0).powi(2));
            points.push(pt2(smile_x, smile_y - curve_y));
        }

        draw.polyline()
            .weight(1.5 * scale)
            .points(points)
            .color(BLACK);
    }

    fn draw_side_face(&self, draw: &Draw, x: f32, head_y: f32, scale: f32, mirrored: bool) {
        // Profile view of skeleton face
        let eye_size = 2.0 * scale;
        let eye_offset = 4.0 * scale; // Forward from center
        let mirror = if mirrored { -1.0 } else { 1.0 };

        // Single eye visible from side
        draw.ellipse()
            .x_y(x + eye_offset * mirror, head_y + 2.0 * scale)
            .radius(eye_size)
            .color(BLACK);

        // Side profile nose (just a small triangle)
        let nose_x = x + 6.0 * scale * mirror;
        let nose_y = head_y;
        let nose_points = vec![
            pt2(nose_x - 1.5 * scale * mirror, nose_y + 1.5 * scale),
            pt2(nose_x + 1.5 * scale * mirror, nose_y),
            pt2(nose_x - 1.5 * scale * mirror, nose_y - 1.5 * scale),
        ];
        draw.polygon().points(nose_points).color(BLACK);

        // Side smile if this skeleton has one
        if self.has_smile {
            let smile_start_x = x + 3.0 * scale * mirror;
            let smile_y = head_y - 4.0 * scale;
            let smile_width = 5.0 * scale;
            let smile_curve = 2.0 * scale;

            let mut smile_points = Vec::new();
            for i in 0..=8 {
                let t = i as f32 / 8.0;
                let sx = smile_start_x + t * smile_width * mirror;
                let sy = smile_y - smile_curve * (t * (1.0 - t)) * 4.0;
                smile_points.push(pt2(sx, sy));
            }

            draw.polyline()
                .weight(1.5 * scale)
                .points(smile_points)
                .color(BLACK);
        }
    }
}

pub struct DancingSkeletons {
    skeletons: Vec<Skeleton>,
}

impl DancingSkeletons {
    pub fn new() -> Self {
        Self {
            skeletons: Vec::new(),
        }
    }

    fn try_spawn_skeleton(&mut self, analysis: &AudioAnalysis) {
        if self.skeletons.len() >= MAX_SKELETONS {
            return;
        }

        let mut rng = rand::rng();

        // Random scale between 0.6 and 2.5 (much larger range)
        let scale = rng.random_range(0.6..2.5);

        // Calculate offset based on skeleton height to ensure it starts fully outside viewport
        let skeleton_offset = BASE_EDGE_OFFSET + (SKELETON_HEIGHT_FACTOR * scale);

        // Determine dance style first (needed for edge shuffle decision)
        let dance_style = match rng.random_range(0..5) {
            0 => DanceStyle::JumpingJacks,
            1 => DanceStyle::JumpingJacksOverhead,
            2 => DanceStyle::Dougie,
            3 => DanceStyle::Shuffle,
            _ => DanceStyle::ShuffleMirrored,
        };

        // Check if this is a shuffle dance that should go along edges (always true for shuffles)
        let is_shuffle = matches!(
            dance_style,
            DanceStyle::Shuffle | DanceStyle::ShuffleMirrored
        );
        let is_edge_shuffler = is_shuffle;

        let (start_pos, end_pos, rotation) = if is_edge_shuffler {
            self.calculate_edge_shuffle_path(skeleton_offset, &mut rng)
        } else {
            // Normal crossing path through center
            let (start, end) =
                get_crossing_path(SPAWN_AREA_WIDTH, SPAWN_AREA_HEIGHT, skeleton_offset);
            // Random rotation for non-edge skeletons
            let rot = rng.random_range(-30.0_f32..30.0_f32).to_radians();
            (start, end, rot)
        };

        // Calculate velocity to cross from start to end
        let crossing_distance = start_pos.distance(end_pos);
        let crossing_frames = if is_shuffle {
            rng.random_range(600.0..1200.0)
        } else {
            rng.random_range(300.0..600.0)
        }; // Random crossing time
        let speed = crossing_distance / crossing_frames;

        let velocity = (end_pos - start_pos).normalize() * speed;

        // Find dominant frequency band (highest value)
        let dominant_band = analysis
            .bands
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        // Create skeleton with calculated dance style and edge behavior
        let mut skeleton = Skeleton::new(
            start_pos,
            velocity,
            scale,
            dominant_band,
            rotation,
            is_edge_shuffler,
        );
        skeleton.dance_style = dance_style; // Override with pre-determined dance style
        self.skeletons.push(skeleton);
    }

    /// Calculates corner-to-corner edge path for edge shufflers
    fn calculate_edge_shuffle_path(
        &self,
        offset: f32,
        rng: &mut impl rand::Rng,
    ) -> (Vec2, Vec2, f32) {
        let half_width = SPAWN_AREA_WIDTH / 2.0;
        let half_height = SPAWN_AREA_HEIGHT / 2.0;

        // Pick a random corner to start from (0=TL, 1=TR, 2=BL, 3=BR)
        let start_corner = rng.random_range(0..4);

        // Pick one of two adjacent corners (not diagonal)
        let end_corner = match start_corner {
            0 => {
                if rng.random() {
                    1
                } else {
                    2
                }
            } // Top-left -> Top-right or Bottom-left
            1 => {
                if rng.random() {
                    0
                } else {
                    3
                }
            } // Top-right -> Top-left or Bottom-right
            2 => {
                if rng.random() {
                    0
                } else {
                    3
                }
            } // Bottom-left -> Top-left or Bottom-right
            _ => {
                if rng.random() {
                    1
                } else {
                    2
                }
            } // Bottom-right -> Top-right or Bottom-left
        };

        // Convert corner indices to positions (with offset applied outward)
        let get_corner_pos = |corner: i32| -> Vec2 {
            match corner {
                0 => vec2(-half_width - offset, half_height + offset), // Top-left
                1 => vec2(half_width + offset, half_height + offset),  // Top-right
                2 => vec2(-half_width - offset, -half_height - offset), // Bottom-left
                _ => vec2(half_width + offset, -half_height - offset), // Bottom-right
            }
        };

        let start_pos = get_corner_pos(start_corner);
        let end_pos = get_corner_pos(end_corner);

        // Determine which edge we're shuffling along and set rotation
        // For shuffle side view: feet should point TOWARDS edge, head TOWARDS middle
        let rotation =
            if start_corner == 0 && end_corner == 1 || start_corner == 1 && end_corner == 0 {
                // Top edge (left-right): feet point up (towards top), head down (towards middle) = 180°
                std::f32::consts::PI
            } else if start_corner == 2 && end_corner == 3 || start_corner == 3 && end_corner == 2 {
                // Bottom edge (left-right): feet point down (towards bottom), head up (towards middle) = 0°
                0.0
            } else if start_corner == 0 && end_corner == 2 || start_corner == 2 && end_corner == 0 {
                // Left edge (up-down): feet point left (towards left), head right (towards middle) = 90° (counter-clockwise)
                -std::f32::consts::FRAC_PI_2
            } else {
                // Right edge (up-down): feet point right (towards right), head left (towards middle) = -90° (clockwise)
                std::f32::consts::FRAC_PI_2
            };

        (start_pos, end_pos, rotation)
    }
}

impl Visualization for DancingSkeletons {
    fn update(&mut self, analysis: &AudioAnalysis) {
        // Update existing skeletons
        for skeleton in &mut self.skeletons {
            skeleton.update(analysis);
        }

        // Remove skeletons that left the viewport
        // Use rough bounds estimate (will be refined in draw)
        let bounds = Rect::from_w_h(SPAWN_AREA_WIDTH, SPAWN_AREA_HEIGHT);
        self.skeletons.retain(|s| s.is_in_bounds(bounds));

        // Try to spawn new skeletons if below max
        let mut rng = rand::rng();
        if self.skeletons.len() < MAX_SKELETONS && rng.random::<f32>() < 0.05 {
            self.try_spawn_skeleton(analysis);
        }
    }

    fn draw(&self, draw: &Draw, _bounds: Rect) {
        // Draw all skeletons
        for skeleton in &self.skeletons {
            skeleton.draw(draw);
        }
    }
}
