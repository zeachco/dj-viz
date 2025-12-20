//! Viewport utility functions for calculating edge coordinates and crossing paths.

use nannou::prelude::*;
use rand::Rng;

/// Returns a random coordinate on the edge of the viewport with an offset
/// applied along the radial direction from the center.
///
/// The offset moves the point outward from the center along the line
/// connecting the center to the edge point.
///
/// # Arguments
/// * `bounds_width` - Width of the viewport
/// * `bounds_height` - Height of the viewport
/// * `pushback` - Distance to move the point outward from the center (in pixels)
///
/// # Example
/// ```
/// // Get a random point on the edge, 50px outside the viewport
/// let point = get_random_edge_coord(800.0, 600.0, 50.0);
/// ```
pub fn get_random_edge_coord(bounds_width: f32, bounds_height: f32, pushback: f32) -> Vec2 {
    let mut rng = rand::rng();

    // Pick a random edge (0=left, 1=right, 2=top, 3=bottom)
    let edge = rng.random_range(0..4);

    let half_width = bounds_width / 2.0;
    let half_height = bounds_height / 2.0;

    // Get point on edge
    let (x, y) = match edge {
        0 => {
            // Left edge
            let y = rng.random_range(-half_height..half_height);
            (-half_width, y)
        }
        1 => {
            // Right edge
            let y = rng.random_range(-half_height..half_height);
            (half_width, y)
        }
        2 => {
            // Top edge
            let x = rng.random_range(-half_width..half_width);
            (x, half_height)
        }
        _ => {
            // Bottom edge
            let x = rng.random_range(-half_width..half_width);
            (x, -half_height)
        }
    };

    let point = vec2(x, y);

    // Calculate direction from center (0, 0) to point
    let direction = point.normalize();

    // Apply offset along the radial direction (away from center)
    point + direction * pushback
}

/// Gets a start position (outside viewport) and calculates the crossing end position
/// (on the opposite side, also outside viewport).
///
/// The path crosses through the center of the viewport, ensuring diagonal traversal.
///
/// # Arguments
/// * `bounds_width` - Width of the viewport
/// * `bounds_height` - Height of the viewport
/// * `offset` - Distance outside viewport boundaries for start/end points
///
/// # Returns
/// A tuple of (start_position, end_position) both as Vec2
///
/// # Example
/// ```
/// // Get a crossing path with 50px offset
/// let (start, end) = get_crossing_path(800.0, 600.0, 50.0);
/// let velocity = (end - start).normalize() * speed;
/// ```
pub fn get_crossing_path(bounds_width: f32, bounds_height: f32, offset: f32) -> (Vec2, Vec2) {
    let start = get_random_edge_coord(bounds_width, bounds_height, offset);

    // Calculate end position: opposite side through center
    // The skeleton crosses from one edge to the opposite edge
    // We calculate where the line from start through (0,0) intersects the opposite edge

    let direction = -start.normalize(); // Direction toward and through center

    // Find intersection with bounds by checking which edge we hit first
    let half_width = bounds_width / 2.0;
    let half_height = bounds_height / 2.0;

    // Calculate intersection with each edge and find the closest valid one
    let mut end = vec2(0.0, 0.0);
    let mut min_t = f32::INFINITY;

    // Right edge (x = half_width)
    if direction.x > 0.001 {
        let t = half_width / direction.x;
        let y = t * direction.y;
        if y.abs() <= half_height && t > 0.0 && t < min_t {
            min_t = t;
            end = vec2(half_width, y);
        }
    }

    // Left edge (x = -half_width)
    if direction.x < -0.001 {
        let t = -half_width / direction.x;
        let y = t * direction.y;
        if y.abs() <= half_height && t > 0.0 && t < min_t {
            min_t = t;
            end = vec2(-half_width, y);
        }
    }

    // Top edge (y = half_height)
    if direction.y > 0.001 {
        let t = half_height / direction.y;
        let x = t * direction.x;
        if x.abs() <= half_width && t > 0.0 && t < min_t {
            min_t = t;
            end = vec2(x, half_height);
        }
    }

    // Bottom edge (y = -half_height)
    if direction.y < -0.001 {
        let t = -half_height / direction.y;
        let x = t * direction.x;
        if x.abs() <= half_width && t > 0.0 && t < min_t {
            end = vec2(x, -half_height);
        }
    }

    // Apply offset to end position (move it outside viewport)
    let end_with_offset = end + end.normalize() * offset;

    (start, end_with_offset)
}
