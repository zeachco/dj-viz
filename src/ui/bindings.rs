//! Keyboard bindings and input handling.
//!
//! Centralizes all keyboard shortcuts and key mapping logic.

use nannou::prelude::*;

/// Actions that can be triggered by key presses
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // App-level
    Quit,
    ShowHelp,

    // Search mode navigation (audio device search)
    SearchCancel,
    SearchMoveUp,
    SearchMoveDown,
    SearchBackspace,
    SearchConfirm,
    SearchInput(char),

    // Viz picker mode navigation
    VizPickerShow,
    VizPickerHide,
    VizPickerMoveUp,
    VizPickerMoveDown,
    VizPickerSelect,
    VizPickerToggle,

    // Normal mode
    StartSearch,
    ToggleDebugViz,
    ToggleLock,
    CycleNext,
    CycleScript,
}

/// Convert a Key to a character (alphanumeric only)
pub fn key_to_char(key: Key, shift: bool) -> Option<char> {
    let c = match key {
        Key::A => 'a',
        Key::B => 'b',
        Key::C => 'c',
        Key::D => 'd',
        Key::E => 'e',
        Key::F => 'f',
        Key::G => 'g',
        Key::H => 'h',
        Key::I => 'i',
        Key::J => 'j',
        Key::K => 'k',
        Key::L => 'l',
        Key::M => 'm',
        Key::N => 'n',
        Key::O => 'o',
        Key::P => 'p',
        Key::Q => 'q',
        Key::R => 'r',
        Key::S => 's',
        Key::T => 't',
        Key::U => 'u',
        Key::V => 'v',
        Key::W => 'w',
        Key::X => 'x',
        Key::Y => 'y',
        Key::Z => 'z',
        Key::Key0 => '0',
        Key::Key1 => '1',
        Key::Key2 => '2',
        Key::Key3 => '3',
        Key::Key4 => '4',
        Key::Key5 => '5',
        Key::Key6 => '6',
        Key::Key7 => '7',
        Key::Key8 => '8',
        Key::Key9 => '9',
        Key::Minus => '-',
        Key::Period => '.',
        Key::Underline => '_',
        _ => return None,
    };

    Some(if shift && c.is_alphabetic() {
        c.to_ascii_uppercase()
    } else {
        c
    })
}

/// Parse a key into an action based on current mode
pub fn parse_key(key: Key, shift: bool, search_active: bool, viz_picker_active: bool) -> Option<Action> {
    // Global quit key
    if key == Key::Q {
        return Some(Action::Quit);
    }

    // Help toggle (works in all modes except search)
    if !search_active && key == Key::H {
        return Some(Action::ShowHelp);
    }

    // Search mode bindings (audio device search)
    if search_active {
        return match key {
            Key::Escape => Some(Action::SearchCancel),
            Key::Up => Some(Action::SearchMoveUp),
            Key::Down => Some(Action::SearchMoveDown),
            Key::Back => Some(Action::SearchBackspace),
            Key::Return => Some(Action::SearchConfirm),
            _ => key_to_char(key, shift).map(Action::SearchInput),
        };
    }

    // Viz picker mode bindings
    if viz_picker_active {
        return match key {
            Key::Escape => Some(Action::VizPickerHide),
            Key::Up => Some(Action::VizPickerMoveUp),
            Key::Down => Some(Action::VizPickerMoveDown),
            Key::Return => Some(Action::VizPickerSelect),
            Key::T => Some(Action::VizPickerToggle),
            _ => None,
        };
    }

    // Normal mode bindings
    match key {
        Key::Slash => Some(Action::StartSearch),
        Key::D => Some(Action::ToggleDebugViz),
        Key::L => Some(Action::ToggleLock),
        Key::Space => Some(Action::CycleNext),
        Key::S => Some(Action::CycleScript),
        Key::Up | Key::Down => Some(Action::VizPickerShow),
        _ => None,
    }
}
