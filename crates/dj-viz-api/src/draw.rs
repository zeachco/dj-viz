//! FFI-safe drawing API wrapper for nannou::Draw

use abi_stable::StableAbi;
use std::ffi::c_void;

/// FFI-safe color type (RGBA, 0.0-1.0 range)
#[repr(C)]
#[derive(StableAbi, Copy, Clone, Debug)]
pub struct ColorFFI {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorFFI {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const BLACK: ColorFFI = ColorFFI::rgb(0.0, 0.0, 0.0);
    pub const WHITE: ColorFFI = ColorFFI::rgb(1.0, 1.0, 1.0);
}

/// FFI-safe wrapper around nannou::Draw
///
/// Uses opaque pointer to wrap nannou::Draw and exposes immediate-mode
/// drawing methods that execute complete draw operations.
#[repr(C)]
#[derive(StableAbi)]
pub struct DrawFFI {
    /// Opaque pointer to nannou::Draw
    ptr: *mut c_void,
}

impl DrawFFI {
    /// Create from nannou::Draw reference (UNSAFE: lifetime must outlive usage)
    pub unsafe fn from_draw(draw: &nannou::Draw) -> Self {
        Self {
            ptr: draw as *const nannou::Draw as *mut c_void,
        }
    }

    /// Get reference to nannou::Draw (UNSAFE: caller must ensure validity)
    unsafe fn draw(&self) -> &nannou::Draw {
        &*(self.ptr as *const nannou::Draw)
    }

    /// Draw a filled rectangle
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, color: ColorFFI) {
        unsafe {
            self.draw()
                .rect()
                .x_y(x, y)
                .w_h(w, h)
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }

    /// Draw a filled ellipse
    pub fn ellipse(&self, x: f32, y: f32, w: f32, h: f32, color: ColorFFI) {
        unsafe {
            self.draw()
                .ellipse()
                .x_y(x, y)
                .w_h(w, h)
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }

    /// Draw a line
    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, weight: f32, color: ColorFFI) {
        unsafe {
            use nannou::prelude::*;
            self.draw()
                .line()
                .start(pt2(x1, y1))
                .end(pt2(x2, y2))
                .weight(weight)
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }

    /// Set background color
    pub fn background(&self, color: ColorFFI) {
        unsafe {
            self.draw()
                .background()
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }

    /// Draw a triangle
    pub fn tri(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: ColorFFI) {
        unsafe {
            use nannou::prelude::*;
            self.draw()
                .tri()
                .points(pt2(x1, y1), pt2(x2, y2), pt2(x3, y3))
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }

    /// Draw a quadrilateral
    pub fn quad(
        &self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        x4: f32,
        y4: f32,
        color: ColorFFI,
    ) {
        unsafe {
            use nannou::prelude::*;
            self.draw()
                .quad()
                .points(pt2(x1, y1), pt2(x2, y2), pt2(x3, y3), pt2(x4, y4))
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }

    /// Draw a polyline (connected line segments)
    pub fn polyline(&self, points: &[(f32, f32)], weight: f32, color: ColorFFI) {
        unsafe {
            use nannou::prelude::*;
            let pts: Vec<Point2> = points.iter().map(|(x, y)| pt2(*x, *y)).collect();
            self.draw()
                .polyline()
                .weight(weight)
                .points(pts)
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }

    /// Draw a filled polygon
    pub fn polygon(&self, points: &[(f32, f32)], color: ColorFFI) {
        unsafe {
            use nannou::prelude::*;
            let pts: Vec<Point2> = points.iter().map(|(x, y)| pt2(*x, *y)).collect();
            self.draw()
                .polygon()
                .points(pts)
                .color(nannou::color::rgba(color.r, color.g, color.b, color.a));
        }
    }
}
