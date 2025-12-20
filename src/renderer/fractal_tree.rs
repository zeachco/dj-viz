//! Growing tree visualization with colored leaves.
//!
//! Renders branches growing from edges towards center with eggshell-colored bark
//! and colored leaves (blood-red, orange, yellow).

use super::Visualization;
use nannou::prelude::*;
use rand::Rng;
use std::cell::Cell;

use crate::audio::AudioAnalysis;
use crate::utils::get_random_edge_coord;

const MAX_BRANCHES: usize = 5;
const MIN_BRANCHES: usize = 1;
const GROWTH_SPEED: f32 = 8.0; // Pixels per frame for main branches (lightning fast)
const FORK_GROWTH_SPEED: f32 = 5.0; // Pixels per frame for forks (also fast)
const CENTER_RADIUS: f32 = 50.0; // Distance from center where branch disappears
const TWIST_AMOUNT: f32 = 0.12; // Random angle change per frame (more chaotic)
const ENERGY_THRESHOLD: f32 = 0.15; // Energy diff needed to spawn new branch
const ENERGY_TURN_THRESHOLD: f32 = 0.1; // Energy diff needed to change angle dramatically
const FORK_CHANCE: f32 = 0.2; // Chance to create a fork per frame (more forks)
const FORK_MIN_TURNS: usize = 15; // Minimum segments for a fork (increased)
const FORK_MAX_TURNS: usize = 25; // Maximum segments for a fork (increased)
const SPAWN_OFFSET: f32 = 50.0; // How far outside viewport to spawn branches

#[derive(Clone)]
enum LeafColor {
    Red,
    Orange,
    Yellow,
}

impl LeafColor {
    fn random() -> Self {
        let roll = rand::rng().random_range(0.0..1.0);
        if roll < 0.04 {
            LeafColor::Yellow
        } else if roll < 0.19 { // 0.04 + 0.15 = 0.19
            LeafColor::Orange
        } else {
            LeafColor::Red
        }
    }

    fn to_rgb(&self) -> Rgb<u8> {
        match self {
            LeafColor::Red => rgb(153, 0, 13),      // Blood red
            LeafColor::Orange => rgb(255, 140, 0),  // Dark orange
            LeafColor::Yellow => rgb(255, 215, 0),  // Gold yellow
        }
    }
}

#[derive(Clone)]
struct Segment {
    position: Vec2,
    thickness: f32,
}

struct Branch {
    /// Path of the branch (list of positions)
    segments: Vec<Segment>,
    /// Current growth direction angle
    angle: f32,
    /// Current thickness
    thickness: f32,
    /// Is this branch still growing?
    alive: bool,
    /// Parent branch ID (None for main branches, Some for forks)
    parent_id: Option<usize>,
    /// Remaining turns for fork branches (None for main branches)
    remaining_turns: Option<usize>,
    /// Branch unique ID
    id: usize,
    /// Distance traveled since last major turn
    distance_since_turn: f32,
    /// Spawn radius (distance from center where branch started)
    spawn_radius: f32,
    /// Total length traveled by this branch
    total_length: f32,
    /// Maximum allowed length for this branch (5x screen width)
    max_length: f32,
    /// Branch color based on dominant frequency at spawn
    color: Rgba,
}

#[derive(Clone)]
struct Leaf {
    position: Vec2,
    angle: f32,
    size: f32,
    color: LeafColor,
    /// Which branch this leaf belongs to
    branch_id: usize,
}

pub struct FractalTree {
    branches: Vec<Branch>,
    leaves: Vec<Leaf>,
    last_energy_diff: f32,
    next_branch_id: usize,
    last_energy_turn_triggered: bool,
    bounds: Cell<Rect>,
}

impl FractalTree {
    /// Get color based on the dominant frequency band
    /// Maps frequency bands to colors: low=warm, mid=green/yellow, high=cool
    fn color_from_bands(bands: &[f32; 8]) -> Rgba {
        // Find the dominant band (highest energy)
        let mut max_idx = 0;
        let mut max_val = bands[0];
        for (i, &val) in bands.iter().enumerate() {
            if val > max_val {
                max_val = val;
                max_idx = i;
            }
        }

        // Add some randomness to the color
        let mut rng = rand::rng();
        let variation = rng.random_range(-0.1..0.1);

        // Map bands to colors with 90% opacity
        match max_idx {
            0 => rgba(0.9 + variation, 0.1, 0.1, 0.9),      // Sub-bass: Deep red
            1 => rgba(1.0, 0.3 + variation, 0.0, 0.9),      // Bass: Orange
            2 => rgba(1.0, 0.7 + variation, 0.0, 0.9),      // Low-mid: Yellow-orange
            3 => rgba(0.9 + variation, 0.9, 0.2, 0.9),      // Mid: Yellow
            4 => rgba(0.3 + variation, 0.9, 0.3, 0.9),      // Upper-mid: Green
            5 => rgba(0.2, 0.7 + variation, 0.9, 0.9),      // Presence: Cyan
            6 => rgba(0.3, 0.3, 0.9 + variation, 0.9),      // Brilliance: Blue
            7 => rgba(0.7 + variation, 0.3, 0.9, 0.9),      // Air: Violet
            _ => rgba(0.95, 0.92, 0.85, 0.9),               // Fallback: Eggshell
        }
    }

    pub fn new() -> Self {
        // Default bounds (will be updated on first draw)
        let default_bounds = Rect::from_w_h(800.0, 600.0);

        let mut tree = Self {
            branches: Vec::new(),
            leaves: Vec::new(),
            last_energy_diff: 0.0,
            next_branch_id: 0,
            last_energy_turn_triggered: false,
            bounds: Cell::new(default_bounds),
        };

        // Start with one branch from random edge (default color)
        tree.spawn_main_branch_with_color(rgba(0.95, 0.92, 0.85, 0.9));

        tree
    }

    /// Calculate spawn radius (corner-to-center distance) from bounds
    fn calculate_spawn_radius(&self) -> f32 {
        let bounds = self.bounds.get();
        let half_width = bounds.w() / 2.0;
        let half_height = bounds.h() / 2.0;
        (half_width * half_width + half_height * half_height).sqrt()
    }

    /// Spawn a new main branch from a random edge with a specific color
    fn spawn_main_branch_with_color(&mut self, color: Rgba) {
        // Count only main branches (not forks)
        let main_branch_count = self.branches.iter().filter(|b| b.parent_id.is_none()).count();
        if main_branch_count >= MAX_BRANCHES {
            return;
        }

        let bounds = self.bounds.get();

        // Use viewport utility to get random edge position outside bounds
        let start_pos = get_random_edge_coord(
            bounds.w(),
            bounds.h(),
            SPAWN_OFFSET,
        );

        let center = vec2(0.0, 0.0);

        // Calculate initial angle towards center with some randomness
        let to_center = (center - start_pos).normalize();
        let mut rng = rand::rng();
        let angle_variation = rng.random_range(-0.5..0.5);
        let initial_angle = to_center.y.atan2(to_center.x) + angle_variation;

        let thickness = rng.random_range(8.0..15.0);
        let branch_id = self.next_branch_id;
        self.next_branch_id += 1;

        let spawn_radius = self.calculate_spawn_radius();
        let max_length = bounds.w() * 5.0; // 5 times screen width

        let branch = Branch {
            segments: vec![Segment {
                position: start_pos,
                thickness,
            }],
            angle: initial_angle,
            thickness,
            alive: true,
            parent_id: None,
            remaining_turns: None,
            id: branch_id,
            distance_since_turn: 0.0,
            spawn_radius,
            total_length: 0.0,
            max_length,
            color,
        };

        self.branches.push(branch);
    }

    /// Spawn a fork branch from an existing branch
    fn spawn_fork(&mut self, parent_idx: usize) {
        let parent = &self.branches[parent_idx];
        if !parent.alive || parent.segments.is_empty() {
            return;
        }

        let fork_pos = parent.segments.last().unwrap().position;
        let parent_spawn_radius = parent.spawn_radius;
        let parent_id_val = parent.id;
        let parent_angle = parent.angle;
        let parent_thickness = parent.thickness;
        let parent_max_length = parent.max_length;
        let parent_color = parent.color;

        let mut rng = rand::rng();

        // Fork angle diverges from parent by 30-60 degrees
        let angle_divergence = rng.random_range(0.5..1.0);
        let fork_angle = if rng.random_range(0.0..1.0) < 0.5 {
            parent_angle + angle_divergence
        } else {
            parent_angle - angle_divergence
        };

        let fork_thickness = parent_thickness * 0.6;
        let turns = rng.random_range(FORK_MIN_TURNS..=FORK_MAX_TURNS);
        let branch_id = self.next_branch_id;
        self.next_branch_id += 1;

        let fork = Branch {
            segments: vec![Segment {
                position: fork_pos,
                thickness: fork_thickness,
            }],
            angle: fork_angle,
            thickness: fork_thickness,
            alive: true,
            parent_id: Some(parent_id_val),
            remaining_turns: Some(turns),
            id: branch_id,
            distance_since_turn: 0.0,
            spawn_radius: parent_spawn_radius,
            total_length: 0.0,
            max_length: parent_max_length,
            color: parent_color, // Inherit parent's color
        };

        self.branches.push(fork);
    }

    /// Spawn a leaf at a given position
    fn spawn_leaf(&mut self, position: Vec2, angle: f32, branch_id: usize) {
        if self.leaves.len() < 300 {
            let mut rng = rand::rng();
            self.leaves.push(Leaf {
                position,
                angle: angle + rng.random_range(-0.5..0.5),
                size: rng.random_range(0.8..1.2),
                color: LeafColor::random(),
                branch_id,
            });
        }
    }

    /// Draw a branch with its color
    fn draw_branch(&self, draw: &Draw, branch: &Branch) {
        // Use branch's color
        let base_color = branch.color;

        // Draw branch as connected line segments
        for i in 0..branch.segments.len().saturating_sub(1) {
            let start = branch.segments[i].position;
            let end = branch.segments[i + 1].position;
            let thickness = (branch.segments[i].thickness + branch.segments[i + 1].thickness) / 2.0;

            draw.line()
                .start(start)
                .end(end)
                .weight(thickness)
                .color(base_color);
        }

        // Optional: Add subtle glow effect for lightning
        for i in 0..branch.segments.len().saturating_sub(1) {
            let start = branch.segments[i].position;
            let end = branch.segments[i + 1].position;
            let thickness = (branch.segments[i].thickness + branch.segments[i + 1].thickness) / 2.0;

            // Outer glow with lower opacity
            draw.line()
                .start(start)
                .end(end)
                .weight(thickness * 1.5)
                .color(rgba(base_color.red, base_color.green, base_color.blue, 0.3));
        }
    }

    /// Draw a pointy leaf
    fn draw_leaf(&self, draw: &Draw, leaf: &Leaf) {
        let rgb_color = leaf.color.to_rgb();
        // Convert to rgba with 90% opacity
        let color = rgba(
            rgb_color.red as f32 / 255.0,
            rgb_color.green as f32 / 255.0,
            rgb_color.blue as f32 / 255.0,
            0.9
        );
        let base_size = 4.0 * leaf.size;

        let cos_a = leaf.angle.cos();
        let sin_a = leaf.angle.sin();

        // Define leaf points (pointy diamond shape)
        let pos = leaf.position;
        let tip = pos + vec2(cos_a, sin_a) * base_size * 1.5;
        let left = pos + vec2(-sin_a, cos_a) * base_size * 0.5;
        let bottom = pos - vec2(cos_a, sin_a) * base_size * 0.5;
        let right = pos + vec2(sin_a, -cos_a) * base_size * 0.5;

        // Draw the pointy leaf as triangles
        draw.tri()
            .points(tip, left, pos)
            .color(color);
        draw.tri()
            .points(tip, pos, right)
            .color(color);
        draw.tri()
            .points(left, bottom, pos)
            .color(color);
        draw.tri()
            .points(pos, bottom, right)
            .color(color);
    }
}

impl Visualization for FractalTree {
    fn update(&mut self, analysis: &AudioAnalysis) {
        // Detect energy diff peak for dramatic angle changes
        let energy_turn_triggered = analysis.energy_diff.abs() >= ENERGY_TURN_THRESHOLD;
        let should_change_angles = energy_turn_triggered && !self.last_energy_turn_triggered;

        // Grow existing branches
        let mut branches_to_remove = Vec::new();
        let mut leaves_to_spawn = Vec::new();
        let mut forks_to_spawn = Vec::new();

        for (idx, branch) in self.branches.iter_mut().enumerate() {
            if !branch.alive {
                continue;
            }

            let last_pos = branch.segments.last().unwrap().position;

            // Check if branch exceeded maximum length
            if branch.total_length >= branch.max_length {
                branches_to_remove.push((idx, branch.id));
                leaves_to_spawn.push((last_pos, branch.angle, branch.id));
                continue;
            }

            // Check if branch left viewport (use spawn radius as boundary)
            if last_pos.length() > branch.spawn_radius {
                branches_to_remove.push((idx, branch.id));
                continue;
            }

            // Check if branch reached center
            if last_pos.length() < CENTER_RADIUS {
                branches_to_remove.push((idx, branch.id));
                // Mark leaf to spawn where the branch disappeared
                leaves_to_spawn.push((last_pos, branch.angle, branch.id));
                continue;
            }

            // Check if fork has run out of turns
            if let Some(remaining) = branch.remaining_turns {
                if remaining == 0 {
                    branches_to_remove.push((idx, branch.id));
                    leaves_to_spawn.push((last_pos, branch.angle, branch.id));
                    continue;
                }
                branch.remaining_turns = Some(remaining - 1);
            }

            // Calculate proximity to center (0.0 at spawn, 1.0 at center)
            let distance_from_center = last_pos.length();
            let proximity = 1.0 - (distance_from_center / branch.spawn_radius).clamp(0.0, 1.0);

            // Calculate minimum turn distance based on proximity
            // At spawn (proximity=0): 10% of spawn radius
            // At center (proximity=1): 1% of spawn radius
            let min_turn_percentage = 0.10 - (proximity * 0.09);
            let min_turn_distance = branch.spawn_radius * min_turn_percentage;

            // Calculate twist amount based on branch length (segment count) and proximity
            // More segments = more twist, closer to center = more twist
            let segment_count = branch.segments.len();
            let length_multiplier = 1.0 + (segment_count as f32 * 0.02).min(2.0);
            let proximity_multiplier = 1.0 + proximity * 3.0; // Up to 4x twist near center
            let base_twist = TWIST_AMOUNT * length_multiplier * proximity_multiplier;

            let mut rng = rand::rng();

            // Check if we can make a major turn
            if branch.distance_since_turn >= min_turn_distance {
                // Make a major turn - more dramatic closer to center
                let turn_magnitude = rng.random_range(0.3..0.8) * (1.0 + proximity * 2.0);
                let turn = if rng.random_range(0.0..1.0) < 0.5 {
                    turn_magnitude
                } else {
                    -turn_magnitude
                };
                branch.angle += turn;
                branch.distance_since_turn = 0.0;
            }

            // Apply dramatic angle change on energy peaks
            if should_change_angles {
                let dramatic_turn = rng.random_range(-0.6..0.6) * (1.0 + proximity); // Larger near center
                branch.angle += dramatic_turn;
                branch.distance_since_turn = 0.0; // Reset turn counter
            }

            // Random angle twist (continuous, increases with length and proximity)
            let twist = rng.random_range(-base_twist..base_twist);
            branch.angle += twist;

            // Slightly bias towards center (only for main branches)
            if branch.parent_id.is_none() {
                let to_center = -last_pos.normalize();
                let center_angle = to_center.y.atan2(to_center.x);
                let angle_diff = center_angle - branch.angle;
                branch.angle += angle_diff * 0.05;
            }

            // Grow the branch (forks grow slower)
            let base_speed = if branch.parent_id.is_some() {
                FORK_GROWTH_SPEED
            } else {
                GROWTH_SPEED
            };
            let growth_speed = base_speed * (1.0 + analysis.energy * 0.5);
            let new_pos = last_pos + vec2(branch.angle.cos(), branch.angle.sin()) * growth_speed;

            // Track distance traveled since last turn
            branch.distance_since_turn += growth_speed;

            // Track total length traveled
            branch.total_length += growth_speed;

            // Gradually thin the branch
            branch.thickness *= 0.998;

            branch.segments.push(Segment {
                position: new_pos,
                thickness: branch.thickness,
            });

            // Occasionally spawn a leaf along the branch
            if rng.random_range(0.0..1.0) < 0.05 {
                leaves_to_spawn.push((new_pos, branch.angle, branch.id));
            }

            // Randomly spawn fork branches (only from main branches)
            if branch.parent_id.is_none() && rng.random_range(0.0..1.0) < FORK_CHANCE {
                forks_to_spawn.push(idx);
            }
        }

        // Spawn leaves
        for (pos, angle, branch_id) in leaves_to_spawn {
            self.spawn_leaf(pos, angle, branch_id);
        }

        // Spawn forks
        for parent_idx in forks_to_spawn {
            self.spawn_fork(parent_idx);
        }

        // Collect branch IDs to remove (including their children)
        let mut branch_ids_to_remove = Vec::new();
        for &(_, branch_id) in &branches_to_remove {
            branch_ids_to_remove.push(branch_id);
            // Also collect fork branches that belong to this parent
            for branch in &self.branches {
                if let Some(parent_id) = branch.parent_id {
                    if parent_id == branch_id {
                        branch_ids_to_remove.push(branch.id);
                    }
                }
            }
        }

        // Remove leaves associated with removed branches
        self.leaves.retain(|leaf| !branch_ids_to_remove.contains(&leaf.branch_id));

        // Remove branches (in reverse order to maintain indices)
        for &(idx, _) in branches_to_remove.iter().rev() {
            self.branches.remove(idx);
        }

        // Remove forks whose parent was removed
        self.branches.retain(|b| {
            if let Some(parent_id) = b.parent_id {
                !branch_ids_to_remove.contains(&parent_id)
            } else {
                true
            }
        });

        // Spawn new main branch if energy diff threshold reached and there's room
        let main_branch_count = self.branches.iter().filter(|b| b.parent_id.is_none()).count();
        if analysis.energy_diff.abs() >= ENERGY_THRESHOLD && self.last_energy_diff.abs() < ENERGY_THRESHOLD {
            if main_branch_count < MAX_BRANCHES {
                let color = Self::color_from_bands(&analysis.bands);
                self.spawn_main_branch_with_color(color);
            }
        }

        // Ensure minimum main branches
        if main_branch_count < MIN_BRANCHES {
            let color = Self::color_from_bands(&analysis.bands);
            self.spawn_main_branch_with_color(color);
        }

        self.last_energy_diff = analysis.energy_diff;
        self.last_energy_turn_triggered = energy_turn_triggered;
    }

    fn draw(&self, draw: &Draw, bounds: Rect) {
        // Update bounds for correct spawn positions
        self.bounds.set(bounds);

        // Draw branches
        for branch in &self.branches {
            self.draw_branch(draw, branch);
        }

        // Draw leaves on top
        for leaf in &self.leaves {
            self.draw_leaf(draw, leaf);
        }
    }
}
