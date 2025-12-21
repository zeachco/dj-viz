//! Generic text-based picker overlay for selecting items with fuzzy search.
//!
//! Provides a reusable UI component for displaying a searchable list of items
//! with keyboard navigation.

use nannou::prelude::*;

/// Trait for items that can be displayed in the text picker
pub trait PickerItem {
    fn display(&self) -> String;
}

/// State interface required for rendering the text picker overlay
pub trait TextPickerState {
    type Item: PickerItem;

    fn is_active(&self) -> bool;
    fn query(&self) -> &str;
    fn filtered_items(&self) -> &[Self::Item];
    fn selected_index(&self) -> usize;
}

/// Draw a text picker overlay on screen
///
/// # Parameters
/// - `draw`: Nannou draw context
/// - `bounds`: Window bounds for positioning
/// - `state`: State implementing TextPickerState
pub fn draw_text_picker<T: TextPickerState>(draw: &Draw, bounds: Rect, state: &T) {
    let padding = 20.0;
    let line_height = 22.0;
    let font_size = 18;
    let max_visible = 15;

    // Calculate overlay dimensions - almost full screen width
    let overlay_width = bounds.w() - padding * 2.0;
    let visible_count = state.filtered_items().len().min(max_visible);
    let overlay_height = line_height * (visible_count as f32 + 2.0) + padding * 2.0;

    // Position at top (centered horizontally)
    let overlay_x = 0.0;
    let overlay_y = bounds.top() - overlay_height / 2.0 - padding;

    // Semi-transparent background
    draw.rect()
        .x_y(overlay_x, overlay_y)
        .w_h(overlay_width, overlay_height)
        .color(rgba(0.0, 0.0, 0.0, 0.85));

    // Text box centered on screen, text left-justified within it
    let text_box_width = bounds.w() - padding * 2.0;
    let text_box_x = 0.0; // Center of screen

    // Search query line
    let query_y = overlay_y + overlay_height / 2.0 - padding - line_height / 2.0;
    let query_text = format!("Search: {}_", state.query());
    draw.text(&query_text)
        .xy(pt2(text_box_x, query_y))
        .wh(pt2(text_box_width, line_height))
        .left_justify()
        .no_line_wrap()
        .color(rgb(1.0, 1.0, 1.0))
        .font_size(font_size);

    // Separator line
    let sep_y = query_y - line_height;
    draw.line()
        .start(pt2(bounds.left() + padding, sep_y))
        .end(pt2(bounds.right() - padding, sep_y))
        .color(rgba(1.0, 1.0, 1.0, 0.3))
        .weight(1.0);

    // List filtered results
    for (i, item) in state.filtered_items().iter().take(max_visible).enumerate() {
        let item_y = sep_y - line_height * (i as f32 + 1.0);
        let is_selected = i == state.selected_index();

        let prefix = if is_selected { "> " } else { "  " };
        let text = format!("{}{}", prefix, item.display());

        let color = if is_selected {
            rgb(0.3, 0.8, 1.0) // Highlight color
        } else {
            rgb(1.0, 1.0, 1.0) // White
        };

        draw.text(&text)
            .xy(pt2(text_box_x, item_y))
            .wh(pt2(text_box_width, line_height))
            .left_justify()
            .no_line_wrap()
            .color(color)
            .font_size(font_size);
    }

    // Show "..." if there are more results
    if state.filtered_items().len() > max_visible {
        let more_y = sep_y - line_height * (max_visible as f32 + 1.0);
        let more_text = format!(
            "  ... and {} more",
            state.filtered_items().len() - max_visible
        );
        draw.text(&more_text)
            .xy(pt2(text_box_x, more_y))
            .wh(pt2(text_box_width, line_height))
            .left_justify()
            .no_line_wrap()
            .color(rgba(1.0, 1.0, 1.0, 0.5))
            .font_size(font_size);
    }
}
