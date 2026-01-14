//! Plugin trait and metadata types

use abi_stable::{sabi_trait, std_types::{RString, RVec}, StableAbi};

use crate::{AudioAnalysisFFI, DrawFFI, RectFFI};

/// Visualization categories for smart selection
#[repr(C)]
#[derive(StableAbi, Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisLabel {
    Organic,    // Flowing, natural forms
    Geometric,  // Precise, angular shapes
    Cartoon,    // Animated characters
    Glitchy,    // Digital artifacts
    Intense,    // High energy
    Retro,      // Vintage aesthetic
}

/// Plugin metadata exported by each plugin
#[repr(C)]
#[derive(StableAbi, Clone, Debug)]
pub struct PluginMetadata {
    pub name: RString,
    pub version: RString,
    pub labels: RVec<VisLabel>,
}

impl PluginMetadata {
    pub fn new(name: impl Into<RString>, version: impl Into<RString>, labels: Vec<VisLabel>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            labels: labels.into(),
        }
    }
}

/// Core visualization trait that all plugins must implement
///
/// This trait uses abi_stable's sabi_trait to ensure ABI stability
/// across compilation boundaries.
#[sabi_trait]
pub trait Visualization {
    /// Update visualization state based on audio analysis
    fn update(&mut self, analysis: &AudioAnalysisFFI);

    /// Draw the visualization
    fn draw(&self, draw: &DrawFFI, bounds: RectFFI);
}

// Re-export the generated trait object type
pub use self::Visualization_TO;
