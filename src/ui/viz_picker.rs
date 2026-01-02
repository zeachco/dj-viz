//! Visualization picker overlay for selecting and toggling visualizations.
//!
//! Provides a UI for browsing all available visualizations, selecting the primary
//! visualization, and toggling overlays on/off.

use crate::renderer::VIZ_NAMES;
use crate::ui::text_picker::{PickerItem, TextPickerState};
use nannou::prelude::*;

/// Entry representing a visualization in the picker
#[derive(Clone, Debug)]
pub struct VizEntry {
    pub index: usize,
    pub name: &'static str,
    /// Whether this viz is currently active (primary or overlay)
    pub active: bool,
}

impl PickerItem for VizEntry {
    fn display(&self) -> String {
        let status = if self.active { "[*]" } else { "[ ]" };
        format!("{} {:2} {}", status, self.index, self.name)
    }
}

/// Manages visualization picker state
pub struct VizPicker {
    pub active: bool,
    pub entries: Vec<VizEntry>,
    pub selected_idx: usize,
}

impl VizPicker {
    pub fn new() -> Self {
        let entries = VIZ_NAMES
            .iter()
            .enumerate()
            .map(|(i, &name)| VizEntry {
                index: i,
                name,
                active: false,
            })
            .collect();

        Self {
            active: false,
            entries,
            selected_idx: 0,
        }
    }

    /// Show the picker
    pub fn show(&mut self) {
        self.active = true;
    }

    /// Hide the picker
    pub fn hide(&mut self) {
        self.active = false;
    }

    /// Update active states based on current primary and overlay indices
    pub fn update_active_states(&mut self, primary_idx: usize, overlay_indices: &[usize]) {
        for entry in &mut self.entries {
            entry.active = entry.index == primary_idx || overlay_indices.contains(&entry.index);
        }
    }

    /// Move selection up (cycles)
    pub fn move_up(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected_idx == 0 {
            self.selected_idx = self.entries.len() - 1;
        } else {
            self.selected_idx -= 1;
        }
    }

    /// Move selection down (cycles)
    pub fn move_down(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % self.entries.len();
    }

    /// Get the currently selected visualization index
    pub fn selected_viz_index(&self) -> Option<usize> {
        self.entries.get(self.selected_idx).map(|e| e.index)
    }
}

impl Default for VizPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl TextPickerState for VizPicker {
    type Item = VizEntry;

    fn is_active(&self) -> bool {
        self.active
    }

    fn query(&self) -> &str {
        "" // No search query for viz picker
    }

    fn filtered_items(&self) -> &[Self::Item] {
        &self.entries
    }

    fn selected_index(&self) -> usize {
        self.selected_idx
    }
}

/// Draw the visualization picker overlay
pub fn draw_viz_picker(draw: &Draw, bounds: Rect, picker: &VizPicker) {
    let padding = 20.0;
    let line_height = 22.0;
    let font_size = 18;
    let max_visible = 18; // Show all visualizations

    // Calculate overlay dimensions
    let overlay_width = 350.0;
    let visible_count = picker.entries.len().min(max_visible);
    let overlay_height = line_height * (visible_count as f32 + 2.0) + padding * 2.0;

    // Position at top-right
    let overlay_x = bounds.right() - overlay_width / 2.0 - padding;
    let overlay_y = bounds.top() - overlay_height / 2.0 - padding;

    // Semi-transparent background
    draw.rect()
        .x_y(overlay_x, overlay_y)
        .w_h(overlay_width, overlay_height)
        .color(rgba(0.0, 0.0, 0.0, 0.85));

    // Border
    draw.rect()
        .x_y(overlay_x, overlay_y)
        .w_h(overlay_width, overlay_height)
        .stroke(rgba(1.0, 1.0, 1.0, 0.3))
        .stroke_weight(1.0)
        .no_fill();

    let text_left = overlay_x - overlay_width / 2.0 + padding;

    // Title
    let title_y = overlay_y + overlay_height / 2.0 - padding - line_height / 2.0;
    draw.text("Visualizations")
        .xy(pt2(overlay_x, title_y))
        .wh(pt2(overlay_width - padding * 2.0, line_height))
        .center_justify()
        .color(rgba(0.5, 0.8, 1.0, 0.9))
        .font_size(font_size);

    // Separator line
    let sep_y = title_y - line_height * 0.7;
    draw.line()
        .start(pt2(overlay_x - overlay_width / 2.0 + padding, sep_y))
        .end(pt2(overlay_x + overlay_width / 2.0 - padding, sep_y))
        .color(rgba(1.0, 1.0, 1.0, 0.3))
        .weight(1.0);

    // List visualizations
    for (i, entry) in picker.entries.iter().take(max_visible).enumerate() {
        let item_y = sep_y - line_height * (i as f32 + 1.0);
        let is_selected = i == picker.selected_idx;

        // Status indicator
        let status = if entry.active { "[*]" } else { "[ ]" };

        // Selection indicator
        let prefix = if is_selected { "> " } else { "  " };

        let text = format!("{}{} {:2} {}", prefix, status, entry.index, entry.name);

        let color = if is_selected {
            rgb(0.3, 0.8, 1.0) // Highlight color
        } else if entry.active {
            rgb(0.6, 1.0, 0.6) // Active (green tint)
        } else {
            rgb(1.0, 1.0, 1.0) // White
        };

        draw.text(&text)
            .xy(pt2(text_left + (overlay_width - padding * 2.0) / 2.0, item_y))
            .wh(pt2(overlay_width - padding * 2.0, line_height))
            .left_justify()
            .no_line_wrap()
            .color(color)
            .font_size(font_size);
    }

    // Help text at bottom
    let help_y = sep_y - line_height * (max_visible as f32 + 1.5);
    draw.text("Enter: select | t/Right-click: toggle overlay | Esc: close")
        .xy(pt2(overlay_x, help_y))
        .wh(pt2(overlay_width - padding * 2.0, line_height))
        .center_justify()
        .color(rgba(1.0, 1.0, 1.0, 0.5))
        .font_size(14);
}
