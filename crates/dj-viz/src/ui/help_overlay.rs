//! Help overlay showing keyboard shortcuts.

use nannou::prelude::*;

/// Manages help overlay visibility
pub struct HelpOverlay {
    pub visible: bool,
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn draw(&self, draw: &Draw, _bounds: Rect, locked: bool) {
        if !self.visible {
            return;
        }

        let padding = 25.0;
        let line_height = 24.0;
        let font_size = 18;

        let shortcuts = [
            ("h", "Toggle this help"),
            ("q", "Quit"),
            ("", ""),
            ("--- Visualization ---", ""),
            ("l", if locked { "Unlock auto-cycling (currently LOCKED)" } else { "Lock auto-cycling (currently unlocked)" }),
            ("Space", "Cycle to random visualization"),
            ("Up/Down", "Open viz picker / navigate"),
            ("Scroll", "Open viz picker / navigate"),
            ("Enter/Click", "Select visualization"),
            ("t/Right-click", "Toggle viz as overlay"),
            ("Esc", "Close viz picker"),
            ("", ""),
            ("--- Other ---", ""),
            ("d", "Toggle debug overlay"),
            ("s", "Cycle Rhai scripts"),
            ("/", "Search audio devices"),
        ];

        let visible_lines = shortcuts.len();
        let overlay_height = line_height * (visible_lines as f32) + padding * 2.0;
        let overlay_width = 580.0;
        let key_col_width = 120.0;

        // Center on screen
        let overlay_x = 0.0;
        let overlay_y = 0.0;

        // Semi-transparent background
        draw.rect()
            .x_y(overlay_x, overlay_y)
            .w_h(overlay_width, overlay_height)
            .color(rgba(0.0, 0.0, 0.0, 0.9));

        // Border
        draw.rect()
            .x_y(overlay_x, overlay_y)
            .w_h(overlay_width, overlay_height)
            .stroke(rgba(1.0, 1.0, 1.0, 0.3))
            .stroke_weight(1.0)
            .no_fill();

        let start_y = overlay_y + overlay_height / 2.0 - padding - line_height / 2.0;

        for (i, (key, desc)) in shortcuts.iter().enumerate() {
            let y = start_y - (i as f32) * line_height;

            if key.is_empty() && desc.is_empty() {
                continue; // Blank line
            }

            if key.starts_with("---") {
                // Section header
                draw.text(key)
                    .xy(pt2(overlay_x, y))
                    .wh(pt2(overlay_width - padding * 2.0, line_height))
                    .center_justify()
                    .color(rgba(0.5, 0.8, 1.0, 0.8))
                    .font_size(font_size);
            } else {
                // Key on left (right-aligned), description on right (left-aligned)
                let left_edge = overlay_x - overlay_width / 2.0 + padding;
                let key_x = left_edge + key_col_width / 2.0;
                let desc_x = left_edge + key_col_width + 15.0 + (overlay_width - key_col_width - padding * 2.0 - 15.0) / 2.0;

                draw.text(key)
                    .xy(pt2(key_x, y))
                    .wh(pt2(key_col_width, line_height))
                    .right_justify()
                    .color(rgb(0.3, 0.8, 1.0))
                    .font_size(font_size);

                draw.text(desc)
                    .xy(pt2(desc_x, y))
                    .wh(pt2(overlay_width - key_col_width - padding * 2.0 - 15.0, line_height))
                    .left_justify()
                    .color(rgb(1.0, 1.0, 1.0))
                    .font_size(font_size);
            }
        }
    }
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}
