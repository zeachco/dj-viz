//! ABI-stable rectangle type

use abi_stable::StableAbi;

/// FFI-safe rectangle (window bounds)
#[repr(C)]
#[derive(StableAbi, Copy, Clone, Debug)]
pub struct RectFFI {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectFFI {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_w_h(w: f32, h: f32) -> Self {
        Self { x: 0.0, y: 0.0, w, h }
    }

    pub fn left(&self) -> f32 {
        self.x - self.w * 0.5
    }

    pub fn right(&self) -> f32 {
        self.x + self.w * 0.5
    }

    pub fn top(&self) -> f32 {
        self.y + self.h * 0.5
    }

    pub fn bottom(&self) -> f32 {
        self.y - self.h * 0.5
    }
}
