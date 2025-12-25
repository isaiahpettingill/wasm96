//! Input module for wasm96-core.
//!
//! Responsibilities:
//! - Provide a stable ABI-facing set of input queries (joypad/keyboard/mouse).
//! - Implement those queries by calling into libretro callbacks.
//! - Optionally cache/snapshot inputs per-frame for determinism.

use crate::abi::Button;
use crate::state;
use libretro_sys::*;

/// Convert ABI joypad button id into libretro device ID.
fn map_joypad_button(button: u32) -> Option<u32> {
    match button {
        x if x == Button::B as u32 => Some(DEVICE_ID_JOYPAD_B),
        x if x == Button::Y as u32 => Some(DEVICE_ID_JOYPAD_Y),
        x if x == Button::Select as u32 => Some(DEVICE_ID_JOYPAD_SELECT),
        x if x == Button::Start as u32 => Some(DEVICE_ID_JOYPAD_START),
        x if x == Button::Up as u32 => Some(DEVICE_ID_JOYPAD_UP),
        x if x == Button::Down as u32 => Some(DEVICE_ID_JOYPAD_DOWN),
        x if x == Button::Left as u32 => Some(DEVICE_ID_JOYPAD_LEFT),
        x if x == Button::Right as u32 => Some(DEVICE_ID_JOYPAD_RIGHT),
        x if x == Button::A as u32 => Some(DEVICE_ID_JOYPAD_A),
        x if x == Button::X as u32 => Some(DEVICE_ID_JOYPAD_X),
        x if x == Button::L1 as u32 => Some(DEVICE_ID_JOYPAD_L),
        x if x == Button::R1 as u32 => Some(DEVICE_ID_JOYPAD_R),
        x if x == Button::L2 as u32 => Some(DEVICE_ID_JOYPAD_L2),
        x if x == Button::R2 as u32 => Some(DEVICE_ID_JOYPAD_R2),
        x if x == Button::L3 as u32 => Some(DEVICE_ID_JOYPAD_L3),
        x if x == Button::R3 as u32 => Some(DEVICE_ID_JOYPAD_R3),
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

    let cb = {
        let s = state::global().lock().unwrap();
        s.input_state_cb
    };

    if let Some(input_state) = cb {
        unsafe {
            let val = input_state(port, DEVICE_JOYPAD, 0, id);
            if val != 0 { 1 } else { 0 }
        }
    } else {
        0
    }
}

/// Query whether a given key is pressed.
pub fn key_pressed(_key: u32) -> u32 {
    // TODO(libretro): wire to real keyboard input via libretro if/when exposed.
    0
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
pub fn snapshot_per_frame() {
    // Keep a single lock for updating `state::InputState`.
    let mut s = state::global().lock().unwrap();

    // If you later add real device querying, do it here. For now, keep defaults.
    // s.input.mouse_x = ...
    // s.input.mouse_y = ...
    // s.input.mouse_buttons = ...

    let _ = &mut *s;
}
