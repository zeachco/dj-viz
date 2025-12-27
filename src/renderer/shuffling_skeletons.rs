//! Shuffling skeletons visualization.
//!
//! Animated skeletons shuffle along screen edges with the following behaviors:
//! - Maximum of 6 skeletons on screen at any time
//! - Spawn at screen corners and move along edges (corner to adjacent corner)
//! - Removed when they exit the viewport
//! - Random scales (0.6-2.5x) for variety in size
//! - Facial features: single eye (side view), triangle nose, optional smile (50% chance)
//! - Bone color assigned based on dominant frequency band at spawn time:
//!   - Band 0-1 (Bass): Red/Orange
//!   - Band 2-4 (Mids): Yellow/Green
//!   - Band 5-7 (Treble): Cyan/Blue/Purple
//! - Two dance styles:
//!   - Shuffle: Side view with bass-driven foot steps (Band 0 or 1 > 0.9, 0.5s cooldown)
//!   - Shuffle Mirrored: Same as shuffle but horizontally flipped
//! - Dance style is randomly assigned at spawn
//! - Shuffle steps occur only on beat hits, creating rhythmic foot movements
//! - Edge shufflers: feet point towards edge, head points towards middle of screen
//! - Shuffler velocity modulated by energy (0.1x-1.1x speed based on music intensity)
//! - Hue-shifted outlines (180° shift) create complementary color pairs

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;

const MAX_SKELETONS: usize = 6;
const SPAWN_AREA_WIDTH: f32 = 800.0;
const SPAWN_AREA_HEIGHT: f32 = 600.0;
const BASE_EDGE_OFFSET: f32 = 20.0;
const SKELETON_HEIGHT_FACTOR: f32 = 80.0;
const SHUFFLE_COOLDOWN_FRAMES: u32 = 30; // 0.5 seconds at 60fps
const BASS_THRESHOLD: f32 = 0.9;

#[derive(Clone, Copy, Debug)]
enum DanceStyle {
    Shuffle,
    ShuffleMirrored,
}

struct Skeleton {
    position: Vec2,
    start_position: Vec2,
    velocity: Vec2,
    base_velocity: Vec2,
    scale: f32,
    rotation: f32,
    dance_style: DanceStyle,
    animation_phase: f32,
    has_smile: bool,
    bone_color: Rgb<u8>,
    shuffle_step: i32,
    shuffle_transition: f32,
    shuffle_cooldown: u32,
    trail_hue_rotation: f32,
}

impl Skeleton {
    fn new(
        position: Vec2,
        velocity: Vec2,
        scale: f32,
        dominant_band: usize,
        rotation: f32,
        dance_style: DanceStyle,
    ) -> Self {
        let mut rng = rand::rng();
        let has_smile = rng.random();
        let bone_color = Self::band_to_color(dominant_band);

        Self {
            position,
            start_position: position,
            velocity,
            base_velocity: velocity,
            scale,
            rotation,
            dance_style,
            animation_phase: rng.random_range(0.0..std::f32::consts::TAU),
            has_smile,
            bone_color,
            shuffle_step: 0,
            shuffle_transition: 1.0,
            shuffle_cooldown: 0,
            trail_hue_rotation: 0.0,
        }
    }

    fn band_to_color(band: usize) -> Rgb<u8> {
        match band {
            0 => rgb(255, 50, 50),
            1 => rgb(255, 120, 50),
            2 => rgb(255, 200, 50),
            3 => rgb(200, 255, 50),
            4 => rgb(50, 255, 100),
            5 => rgb(50, 200, 255),
            6 => rgb(100, 100, 255),
            7 => rgb(200, 100, 255),
            _ => rgb(255, 255, 255),
        }
    }

    fn shift_hue(color: Rgb<u8>, shift: f32) -> Rgb<u8> {
        let r = color.red as f32 / 255.0;
        let g = color.green as f32 / 255.0;
        let b = color.blue as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

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

        let s = if max == 0.0 { 0.0 } else { delta / max };
        let v = max;

        h = (h + shift * 360.0) % 360.0;

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
        Self::shift_hue(self.bone_color, 0.5)
    }

    fn update(&mut self, analysis: &AudioAnalysis) {
        // Modulate velocity based on energy
        let energy_scale = 0.1 + analysis.energy * 1.0;
        self.velocity = self.base_velocity * energy_scale;

        self.position += self.velocity;

        // Update animation phase
        let speed = 0.05 + analysis.energy_diff.abs() * 0.2;
        self.animation_phase += speed;
        if self.animation_phase > std::f32::consts::TAU {
            self.animation_phase -= std::f32::consts::TAU;
        }

        // Beat detection and stepping
        if self.shuffle_cooldown > 0 {
            self.shuffle_cooldown -= 1;
        }

        let bass_hit = (analysis.bands_normalized[0] > BASS_THRESHOLD
            || analysis.bands_normalized[1] > BASS_THRESHOLD)
            && self.shuffle_cooldown == 0;

        if bass_hit {
            self.shuffle_step = match self.shuffle_step {
                -1 => 1,
                1 => -1,
                _ => 1,
            };
            self.shuffle_transition = 0.0;
            self.shuffle_cooldown = SHUFFLE_COOLDOWN_FRAMES;
            self.trail_hue_rotation += 100.0 / 360.0;
        }

        if self.shuffle_transition < 1.0 {
            self.shuffle_transition += 0.08;
            if self.shuffle_transition > 1.0 {
                self.shuffle_transition = 1.0;
            }
        }
    }

    fn is_in_bounds(&self, bounds: Rect) -> bool {
        let half_width = SPAWN_AREA_WIDTH / 2.0;
        let half_height = SPAWN_AREA_HEIGHT / 2.0;
        let margin = half_width + half_height;

        self.position.x > bounds.left() - margin
            && self.position.x < bounds.right() + margin
            && self.position.y > bounds.bottom() - margin
            && self.position.y < bounds.top() + margin
    }

    fn get_front_foot_world_position(&self) -> Vec2 {
        let scale = self.scale;
        let mirrored = matches!(self.dance_style, DanceStyle::ShuffleMirrored);
        let mirror = if mirrored { -1.0 } else { 1.0 };

        let bob_amount = if self.shuffle_transition < 0.5 {
            self.shuffle_transition * 2.0
        } else {
            2.0 - (self.shuffle_transition * 2.0)
        };
        let body_y = bob_amount * 3.0 * scale;
        let spine_bottom = body_y - 10.0 * scale;
        let pelvis_y = spine_bottom;
        let pelvis_depth = 5.0 * scale;
        let leg_length = 20.0 * scale;
        let max_step_offset = 12.0 * scale;

        let (left_foot_offset, right_foot_offset) = match self.shuffle_step {
            -1 => (max_step_offset * self.shuffle_transition, 0.0),
            1 => (0.0, max_step_offset * self.shuffle_transition),
            _ => (0.0, 0.0),
        };

        // Front foot is the one with the offset
        let local_foot_pos = if self.shuffle_step == 1 {
            vec2(
                (pelvis_depth + right_foot_offset) * mirror,
                pelvis_y - leg_length,
            )
        } else if self.shuffle_step == -1 {
            vec2(
                (-pelvis_depth + left_foot_offset) * mirror,
                pelvis_y - leg_length,
            )
        } else {
            vec2(pelvis_depth * mirror, pelvis_y - leg_length)
        };

        // Apply rotation and translate to world position
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let rotated = vec2(
            local_foot_pos.x * cos_r - local_foot_pos.y * sin_r,
            local_foot_pos.x * sin_r + local_foot_pos.y * cos_r,
        );
        self.position + rotated
    }

    fn draw(&self, draw: &Draw) {
        // Draw trail line first (behind skeleton)
        self.draw_trail(draw);

        let x = self.position.x;
        let y = self.position.y;
        let scale = self.scale;

        let draw = draw.x_y(x, y).rotate(self.rotation);
        let mirrored = matches!(self.dance_style, DanceStyle::ShuffleMirrored);

        self.draw_shuffle(&draw, 0.0, 0.0, scale, mirrored);
    }

    fn draw_trail(&self, draw: &Draw) {
        let front_foot = self.get_front_foot_world_position();
        let start = self.start_position;

        // Get trail color with hue rotation
        let trail_color = Self::shift_hue(self.bone_color, self.trail_hue_rotation);

        // Draw tapered line from start (1px) to front foot (5px)
        // Use multiple line segments to create taper effect
        let segments = 20;
        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;

            let p0 = start.lerp(front_foot, t0);
            let p1 = start.lerp(front_foot, t1);

            // Weight goes from 1.0 to 5.0
            let weight = 1.0 + (t0 + t1) / 2.0 * 4.0;

            draw.line()
                .start(pt2(p0.x, p0.y))
                .end(pt2(p1.x, p1.y))
                .weight(weight)
                .color(trail_color);
        }
    }

    fn draw_shuffle(&self, draw: &Draw, x: f32, y: f32, scale: f32, mirrored: bool) {
        let bone_color = self.bone_color;
        let outline_color = self.get_outline_color();
        let thickness = 3.0 * scale;
        let outline_thickness = thickness + 2.0 * scale;

        let bob_amount = if self.shuffle_transition < 0.5 {
            self.shuffle_transition * 2.0
        } else {
            2.0 - (self.shuffle_transition * 2.0)
        };
        let body_y = y + bob_amount * 3.0 * scale;

        let head_y = body_y + 25.0 * scale;
        let head_radius = 8.0 * scale;

        let spine_top = head_y - head_radius;
        let spine_bottom = body_y - 10.0 * scale;
        let spine_tilt = if mirrored { -3.0 * scale } else { 3.0 * scale };

        let shoulder_y = spine_top - 5.0 * scale;
        let shoulder_depth = 4.0 * scale;

        let pelvis_y = spine_bottom;
        let pelvis_depth = 5.0 * scale;

        let arm_swing = self.shuffle_step as f32 * 15.0 * scale * self.shuffle_transition;
        let arm_length = 18.0 * scale;

        let max_step_offset = 12.0 * scale;

        let (left_foot_offset, right_foot_offset) = match self.shuffle_step {
            -1 => (max_step_offset * self.shuffle_transition, 0.0),
            1 => (0.0, max_step_offset * self.shuffle_transition),
            _ => (0.0, 0.0),
        };

        let leg_length = 20.0 * scale;
        let mirror = if mirrored { -1.0 } else { 1.0 };

        // Draw outlines first
        draw.ellipse()
            .x_y(x, head_y)
            .radius(head_radius + 1.5 * scale)
            .color(outline_color);

        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x + spine_tilt * mirror, spine_bottom))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x + shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (shoulder_depth - arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (-shoulder_depth + arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x + pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (pelvis_depth + right_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (-pelvis_depth + left_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(x + pelvis_depth * mirror, pelvis_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Draw bones on top
        draw.ellipse()
            .x_y(x, head_y)
            .radius(head_radius)
            .color(bone_color);

        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x + spine_tilt * mirror, spine_bottom))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x + shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (shoulder_depth - arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - shoulder_depth * mirror, shoulder_y))
            .end(pt2(
                x + (-shoulder_depth + arm_swing) * mirror,
                shoulder_y - arm_length,
            ))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x + pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (pelvis_depth + right_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(
                x + (-pelvis_depth + left_foot_offset) * mirror,
                pelvis_y - leg_length,
            ))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - pelvis_depth * mirror, pelvis_y))
            .end(pt2(x + pelvis_depth * mirror, pelvis_y))
            .weight(thickness)
            .color(bone_color);

        self.draw_side_face(draw, x, head_y, scale, mirrored);
    }

    fn draw_side_face(&self, draw: &Draw, x: f32, head_y: f32, scale: f32, mirrored: bool) {
        let eye_size = 2.0 * scale;
        let eye_offset = 4.0 * scale;
        let mirror = if mirrored { -1.0 } else { 1.0 };

        draw.ellipse()
            .x_y(x + eye_offset * mirror, head_y + 2.0 * scale)
            .radius(eye_size)
            .color(BLACK);

        let nose_x = x + 6.0 * scale * mirror;
        let nose_y = head_y;
        let nose_points = vec![
            pt2(nose_x - 1.5 * scale * mirror, nose_y + 1.5 * scale),
            pt2(nose_x + 1.5 * scale * mirror, nose_y),
            pt2(nose_x - 1.5 * scale * mirror, nose_y - 1.5 * scale),
        ];
        draw.polygon().points(nose_points).color(BLACK);

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

pub struct ShufflingSkeletons {
    skeletons: Vec<Skeleton>,
}

impl Default for ShufflingSkeletons {
    fn default() -> Self {
        Self {
            skeletons: Vec::new(),
        }
    }
}

impl ShufflingSkeletons {
    fn try_spawn_skeleton(&mut self, analysis: &AudioAnalysis) {
        if self.skeletons.len() >= MAX_SKELETONS {
            return;
        }

        let mut rng = rand::rng();
        let scale = rng.random_range(0.6..2.5);
        let skeleton_offset = BASE_EDGE_OFFSET + (SKELETON_HEIGHT_FACTOR * scale);

        let dance_style = if rng.random() {
            DanceStyle::Shuffle
        } else {
            DanceStyle::ShuffleMirrored
        };

        let (start_pos, end_pos, rotation) =
            self.calculate_edge_shuffle_path(skeleton_offset, &mut rng);

        let crossing_distance = start_pos.distance(end_pos);
        let crossing_frames = rng.random_range(600.0..1200.0);
        let speed = crossing_distance / crossing_frames;
        let velocity = (end_pos - start_pos).normalize() * speed;

        let dominant_band = analysis
            .bands
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        let skeleton = Skeleton::new(
            start_pos,
            velocity,
            scale,
            dominant_band,
            rotation,
            dance_style,
        );
        self.skeletons.push(skeleton);
    }

    fn calculate_edge_shuffle_path(
        &self,
        offset: f32,
        rng: &mut impl rand::Rng,
    ) -> (Vec2, Vec2, f32) {
        let half_width = SPAWN_AREA_WIDTH / 2.0;
        let half_height = SPAWN_AREA_HEIGHT / 2.0;

        let start_corner = rng.random_range(0..4);

        let end_corner = match start_corner {
            0 => {
                if rng.random() {
                    1
                } else {
                    2
                }
            }
            1 => {
                if rng.random() {
                    0
                } else {
                    3
                }
            }
            2 => {
                if rng.random() {
                    0
                } else {
                    3
                }
            }
            _ => {
                if rng.random() {
                    1
                } else {
                    2
                }
            }
        };

        let get_corner_pos = |corner: i32| -> Vec2 {
            match corner {
                0 => vec2(-half_width - offset, half_height + offset),
                1 => vec2(half_width + offset, half_height + offset),
                2 => vec2(-half_width - offset, -half_height - offset),
                _ => vec2(half_width + offset, -half_height - offset),
            }
        };

        let start_pos = get_corner_pos(start_corner);
        let end_pos = get_corner_pos(end_corner);

        let rotation =
            if start_corner == 0 && end_corner == 1 || start_corner == 1 && end_corner == 0 {
                std::f32::consts::PI
            } else if start_corner == 2 && end_corner == 3 || start_corner == 3 && end_corner == 2 {
                0.0
            } else if start_corner == 0 && end_corner == 2 || start_corner == 2 && end_corner == 0 {
                -std::f32::consts::FRAC_PI_2
            } else {
                std::f32::consts::FRAC_PI_2
            };

        (start_pos, end_pos, rotation)
    }
}

impl Visualization for ShufflingSkeletons {
    fn update(&mut self, analysis: &AudioAnalysis) {
        for skeleton in &mut self.skeletons {
            skeleton.update(analysis);
        }

        let bounds = Rect::from_w_h(SPAWN_AREA_WIDTH, SPAWN_AREA_HEIGHT);
        self.skeletons.retain(|s| s.is_in_bounds(bounds));

        let mut rng = rand::rng();
        if self.skeletons.len() < MAX_SKELETONS && rng.random::<f32>() < 0.05 {
            self.try_spawn_skeleton(analysis);
        }
    }

    fn draw(&self, draw: &Draw, _bounds: Rect) {
        for skeleton in &self.skeletons {
            skeleton.draw(draw);
        }
    }
}
