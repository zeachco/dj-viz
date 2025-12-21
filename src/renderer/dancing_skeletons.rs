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
//! - Three dance styles:
//!   - Jumping Jacks: Front view with arms and legs swinging out
//!   - Jumping Jacks Overhead: Front view with arms going overhead
//!   - Dougie: Front view with side-to-side sway
//! - Dance style is randomly assigned at spawn
//! - Animation speed is affected by energy_diff (higher energy = faster dancing)
//! - Hue-shifted outlines (180° shift) create complementary color pairs

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;

use crate::audio::AudioAnalysis;
use crate::utils::get_crossing_path;

const MAX_SKELETONS: usize = 6;
const SPAWN_AREA_WIDTH: f32 = 800.0;
const SPAWN_AREA_HEIGHT: f32 = 600.0;
const BASE_EDGE_OFFSET: f32 = 20.0;
const SKELETON_HEIGHT_FACTOR: f32 = 80.0;

#[derive(Clone, Copy, Debug)]
enum DanceStyle {
    JumpingJacks,
    JumpingJacksOverhead,
    Dougie,
}

struct Skeleton {
    position: Vec2,
    velocity: Vec2,
    scale: f32,
    rotation: f32,
    dance_style: DanceStyle,
    animation_phase: f32,
    has_smile: bool,
    bone_color: Rgb<u8>,
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
            velocity,
            scale,
            rotation,
            dance_style,
            animation_phase: rng.random_range(0.0..std::f32::consts::TAU),
            has_smile,
            bone_color,
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
        self.position += self.velocity;

        let speed = 0.05 + analysis.energy_diff.abs() * 0.2;
        self.animation_phase += speed;
        if self.animation_phase > std::f32::consts::TAU {
            self.animation_phase -= std::f32::consts::TAU;
        }
    }

    fn is_in_bounds(&self, bounds: Rect) -> bool {
        let margin = BASE_EDGE_OFFSET + (SKELETON_HEIGHT_FACTOR * self.scale);

        self.position.x > bounds.left() - margin
            && self.position.x < bounds.right() + margin
            && self.position.y > bounds.bottom() - margin
            && self.position.y < bounds.top() + margin
    }

    fn draw(&self, draw: &Draw) {
        let x = self.position.x;
        let y = self.position.y;
        let scale = self.scale;

        let draw = draw.rotate(self.rotation);

        match self.dance_style {
            DanceStyle::JumpingJacks => {
                let (arm_offset, leg_offset, body_offset) = self.jumping_jacks_offsets();
                self.draw_skeleton_parts(&draw, x, y, scale, arm_offset, leg_offset, body_offset);
                let head_y = y + body_offset + 25.0 * scale;
                self.draw_face(&draw, x, head_y, scale);
            }
            DanceStyle::JumpingJacksOverhead => {
                let (arm_offset, leg_offset, body_offset) = self.jumping_jacks_overhead_offsets();
                self.draw_skeleton_parts_overhead(
                    &draw,
                    x,
                    y,
                    scale,
                    arm_offset,
                    leg_offset,
                    body_offset,
                );
                let head_y = y + body_offset + 25.0 * scale;
                self.draw_face(&draw, x, head_y, scale);
            }
            DanceStyle::Dougie => {
                let (arm_offset, leg_offset, body_offset) = self.dougie_offsets();
                self.draw_skeleton_parts(&draw, x, y, scale, arm_offset, leg_offset, body_offset);
                let head_y = y + body_offset + 25.0 * scale;
                self.draw_face(&draw, x, head_y, scale);
            }
        }
    }

    fn jumping_jacks_offsets(&self) -> (f32, f32, f32) {
        let t = self.animation_phase;
        let arm_angle = (t.sin() * 0.5 + 0.5) * 60.0;
        let leg_angle = ((t + std::f32::consts::PI).sin() * 0.5 + 0.5) * 30.0;
        let body_y = t.sin().abs() * 10.0;
        (arm_angle, leg_angle, body_y)
    }

    fn jumping_jacks_overhead_offsets(&self) -> (f32, f32, f32) {
        let t = self.animation_phase;
        let arm_angle = (t.sin() * 0.5 + 0.5) * 90.0;
        let leg_angle = ((t + std::f32::consts::PI).sin() * 0.5 + 0.5) * 30.0;
        let body_y = t.sin().abs() * 10.0;
        (arm_angle, leg_angle, body_y)
    }

    fn dougie_offsets(&self) -> (f32, f32, f32) {
        let t = self.animation_phase;
        let arm_angle = (t * 1.5).sin() * 40.0;
        let leg_angle = ((t * 2.0).sin() * 0.5 + 0.5) * 15.0;
        let body_y = (t * 0.5).sin() * 15.0;
        (arm_angle, leg_angle, body_y)
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

        let head_y = y + body_y + 25.0 * scale;
        let spine_top = head_y - 8.0 * scale;
        let spine_bottom = y + body_y - 10.0 * scale;
        let shoulder_y = spine_top - 5.0 * scale;
        let shoulder_width = 15.0 * scale;
        let pelvis_y = spine_bottom;
        let pelvis_width = 12.0 * scale;

        let arm_rad = arm_angle.to_radians();
        let arm_length = 18.0 * scale;

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

        // Draw outlines first
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale + 1.5 * scale)
            .color(outline_color);

        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x + pelvis_width, pelvis_y))
            .end(pt2(right_leg_end_x, right_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Draw bones on top
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale)
            .color(bone_color);

        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(thickness)
            .color(bone_color);

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

        // Draw outlines first
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale + 1.5 * scale)
            .color(outline_color);

        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        draw.line()
            .start(pt2(x + pelvis_width, pelvis_y))
            .end(pt2(right_leg_end_x, right_leg_end_y))
            .weight(outline_thickness)
            .color(outline_color);

        // Draw bones on top
        draw.ellipse()
            .x_y(x, head_y)
            .radius(8.0 * scale)
            .color(bone_color);

        draw.line()
            .start(pt2(x, spine_top))
            .end(pt2(x, spine_bottom))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(x + shoulder_width, shoulder_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - shoulder_width, shoulder_y))
            .end(pt2(left_arm_end_x, left_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x + shoulder_width, shoulder_y))
            .end(pt2(right_arm_end_x, right_arm_end_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(x + pelvis_width, pelvis_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x - pelvis_width, pelvis_y))
            .end(pt2(left_leg_end_x, left_leg_end_y))
            .weight(thickness)
            .color(bone_color);

        draw.line()
            .start(pt2(x + pelvis_width, pelvis_y))
            .end(pt2(right_leg_end_x, right_leg_end_y))
            .weight(thickness)
            .color(bone_color);
    }

    fn draw_face(&self, draw: &Draw, x: f32, head_y: f32, scale: f32) {
        let eye_size = 2.0 * scale;
        let eye_spacing = 3.5 * scale;

        draw.ellipse()
            .x_y(x - eye_spacing, head_y + 2.0 * scale)
            .radius(eye_size)
            .color(BLACK);

        draw.ellipse()
            .x_y(x + eye_spacing, head_y + 2.0 * scale)
            .radius(eye_size)
            .color(BLACK);

        self.draw_upside_down_heart_nose(draw, x, head_y, scale);

        if self.has_smile {
            self.draw_smile(draw, x, head_y, scale);
        }
    }

    fn draw_upside_down_heart_nose(&self, draw: &Draw, x: f32, head_y: f32, scale: f32) {
        let nose_size = 2.5 * scale;
        let nose_y = head_y - 1.0 * scale;

        let points = vec![
            pt2(x, nose_y + nose_size),
            pt2(x - nose_size * 0.5, nose_y + nose_size * 0.3),
            pt2(x - nose_size * 0.7, nose_y - nose_size * 0.3),
            pt2(x - nose_size * 0.5, nose_y - nose_size * 0.6),
            pt2(x, nose_y - nose_size * 0.5),
            pt2(x + nose_size * 0.5, nose_y - nose_size * 0.6),
            pt2(x + nose_size * 0.7, nose_y - nose_size * 0.3),
            pt2(x + nose_size * 0.5, nose_y + nose_size * 0.3),
            pt2(x, nose_y + nose_size),
        ];

        draw.polyline()
            .weight(2.0 * scale)
            .points(points.clone())
            .color(BLACK);

        draw.polygon().points(points).color(BLACK);
    }

    fn draw_smile(&self, draw: &Draw, x: f32, head_y: f32, scale: f32) {
        let smile_width = 6.0 * scale;
        let smile_y = head_y - 5.0 * scale;
        let smile_curve = 2.0 * scale;

        let num_points = 10;
        let mut points = Vec::new();

        for i in 0..=num_points {
            let t = i as f32 / num_points as f32;
            let smile_x = x + (t - 0.5) * smile_width * 2.0;
            let curve_y = smile_curve * (1.0 - (2.0 * t - 1.0).powi(2));
            points.push(pt2(smile_x, smile_y - curve_y));
        }

        draw.polyline()
            .weight(1.5 * scale)
            .points(points)
            .color(BLACK);
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
        let scale = rng.random_range(0.6..2.5);
        let skeleton_offset = BASE_EDGE_OFFSET + (SKELETON_HEIGHT_FACTOR * scale);

        let dance_style = match rng.random_range(0..3) {
            0 => DanceStyle::JumpingJacks,
            1 => DanceStyle::JumpingJacksOverhead,
            _ => DanceStyle::Dougie,
        };

        let (start_pos, end_pos) =
            get_crossing_path(SPAWN_AREA_WIDTH, SPAWN_AREA_HEIGHT, skeleton_offset);
        let rotation = rng.random_range(-30.0_f32..30.0_f32).to_radians();

        let crossing_distance = start_pos.distance(end_pos);
        let crossing_frames = rng.random_range(300.0..600.0);
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
}

impl Visualization for DancingSkeletons {
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
