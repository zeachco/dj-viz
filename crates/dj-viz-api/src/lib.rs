//! FFI-safe API for dj-viz plugins
//!
//! This crate provides ABI-stable types that can cross the FFI boundary
//! between the main application and plugin dynamic libraries.

pub mod audio;
pub mod draw;
pub mod plugin;
pub mod rect;

pub use audio::{AudioAnalysisFFI, NUM_BANDS, SPECTRUM_SIZE};
pub use draw::{ColorFFI, DrawFFI};
pub use plugin::{PluginMetadata, Visualization, Visualization_TO, VisLabel};
pub use rect::RectFFI;

pub const ABI_VERSION: u32 = 1;
