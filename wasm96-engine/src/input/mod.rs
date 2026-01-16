//! Input module for wasm96-engine.
//!
//! Responsibilities:
//! - Provide a stable ABI-facing set of input queries (joypad/keyboard/mouse).
//! - Implement those queries by calling into platform callbacks.
//! - Optionally cache/snapshot inputs per-frame for determinism.

use crate::abi::Button;
use crate::state;
use crate::PlatformCallbacks;

/// Convert ABI joypad button id into a generic button index.
fn map_joypad_button(button: u32) -> Option<u32> {
    match button {
        x if x == Button::B as u32 => Some(0),
        x if x == Button::Y as u32 => Some(1),
        x if x == Button::Select as u32 => Some(2),
        x if x == Button::Start as u32 => Some(3),
        x if x == Button::Up as u32 => Some(4),
        x if x == Button::Down as u32 => Some(5),
        x if x == Button::Left as u32 => Some(6),
        x if x == Button::Right as u32 => Some(7),
        x if x == Button::A as u32 => Some(8),
        x if x == Button::X as u32 => Some(9),
        x if x == Button::L1 as u32 => Some(10),
        x if x == Button::R1 as u32 => Some(11),
        x if x == Button::L2 as u32 => Some(12),
        x if x == Button::R2 as u32 => Some(13),
        x if x == Button::L3 as u32 => Some(14),
        x if x == Button::R3 as u32 => Some(15),
        _ => None,
    }
}

/// Query whether a given joypad button is pressed.
///
/// Returns 1 if pressed, else 0.
pub fn joypad_button_pressed(port: u32, button: u32) -> u32 {
    let Some(id) = map_joypad_button(button) else {
        return 0;
    };

    state::with_callbacks(|callbacks| {
        if callbacks.input_button_state(port, id) {
            1
        } else {
            0
        }
    })
    .unwrap_or(0)
}

/// Query whether a given key is pressed.
pub fn key_pressed(key: u32) -> u32 {
    state::with_callbacks(
        |callbacks| {
            if callbacks.input_key_state(key) {
                1
            } else {
                0
            }
        },
    )
    .unwrap_or(0)
}

/// Mouse X coordinate.
pub fn mouse_x() -> i32 {
    let s = state::global().lock().unwrap();
    s.input.mouse_x
}

/// Mouse Y coordinate.
pub fn mouse_y() -> i32 {
    let s = state::global().lock().unwrap();
    s.input.mouse_y
}

/// Mouse buttons bitmask.
pub fn mouse_buttons() -> u32 {
    let s = state::global().lock().unwrap();
    s.input.mouse_buttons
}

/// Snapshot inputs for the current frame into `state::InputState`.
///
/// Call this once per `on_run` before invoking guest `wasm96_frame`.
pub fn snapshot_per_frame(callbacks: &mut dyn PlatformCallbacks) {
    callbacks.input_poll();

    // Keep a single lock for updating `state::InputState`.
    let mut s = state::global().lock().unwrap();

    // Update mouse state
    s.input.mouse_x = callbacks.input_mouse_x();
    s.input.mouse_y = callbacks.input_mouse_y();

    // Build mouse buttons bitmask
    let mut buttons = 0u32;
    if callbacks.input_mouse_button(0) {
        buttons |= 1; // Left
    }
    if callbacks.input_mouse_button(1) {
        buttons |= 2; // Right
    }
    if callbacks.input_mouse_button(2) {
        buttons |= 4; // Middle
    }
    s.input.mouse_buttons = buttons;
}
