//! Keyboard bindings and input handling.
//!
//! Centralizes all keyboard shortcuts and key mapping logic.

use nannou::prelude::*;

/// Actions that can be triggered by key presses
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // App-level
    Quit,

    // Search mode navigation
    SearchCancel,
    SearchMoveUp,
    SearchMoveDown,
    SearchBackspace,
    SearchConfirm,
    SearchInput(char),

    // Normal mode
    StartSearch,
    ToggleDebugViz,
    CycleNext,
    SelectVisualization(usize),
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
pub fn parse_key(key: Key, shift: bool, search_active: bool) -> Option<Action> {
    // Global quit key
    if key == Key::Q {
        return Some(Action::Quit);
    }

    // Search mode bindings
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

    // Normal mode bindings
    match key {
        Key::Slash => Some(Action::StartSearch),
        Key::D => Some(Action::ToggleDebugViz),
        Key::Space => Some(Action::CycleNext),
        _ => parse_number_key(key, shift).map(Action::SelectVisualization),
    }
}

/// Parse number keys (0-9, Shift+0-9) into visualization indices
fn parse_number_key(key: Key, shift: bool) -> Option<usize> {
    let shift_offset = if shift { 10 } else { 0 };

    match key {
        Key::Key0 => Some(shift_offset),
        Key::Key1 => Some(1 + shift_offset),
        Key::Key2 => Some(2 + shift_offset),
        Key::Key3 => Some(3 + shift_offset),
        Key::Key4 => Some(4 + shift_offset),
        Key::Key5 => Some(5 + shift_offset),
        Key::Key6 => Some(6 + shift_offset),
        Key::Key7 => Some(7 + shift_offset),
        Key::Key8 => Some(8 + shift_offset),
        Key::Key9 => Some(9 + shift_offset),
        _ => None,
    }
}
