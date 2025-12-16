//! Input module for wasm96-core.
//!
//! Responsibilities:
//! - Provide a stable ABI-facing set of input queries (joypad/keyboard/mouse).
//! - Implement those queries by calling into `libretro_backend::RuntimeHandle`.
//! - Optionally cache/snapshot inputs per-frame for determinism.

use crate::abi::Button;
use crate::state;
use libretro_backend::{JoypadButton as LrJoypadButton, RuntimeHandle};

/// Convert ABI joypad button id into libretro-backend joypad button enum.
fn map_joypad_button(button: u32) -> Option<LrJoypadButton> {
    match button {
        x if x == Button::B as u32 => Some(LrJoypadButton::B),
        x if x == Button::Y as u32 => Some(LrJoypadButton::Y),
        x if x == Button::Select as u32 => Some(LrJoypadButton::Select),
        x if x == Button::Start as u32 => Some(LrJoypadButton::Start),
        x if x == Button::Up as u32 => Some(LrJoypadButton::Up),
        x if x == Button::Down as u32 => Some(LrJoypadButton::Down),
        x if x == Button::Left as u32 => Some(LrJoypadButton::Left),
        x if x == Button::Right as u32 => Some(LrJoypadButton::Right),
        x if x == Button::A as u32 => Some(LrJoypadButton::A),
        x if x == Button::X as u32 => Some(LrJoypadButton::X),
        x if x == Button::L1 as u32 => Some(LrJoypadButton::L1),
        x if x == Button::R1 as u32 => Some(LrJoypadButton::R1),
        x if x == Button::L2 as u32 => Some(LrJoypadButton::L2),
        x if x == Button::R2 as u32 => Some(LrJoypadButton::R2),
        x if x == Button::L3 as u32 => Some(LrJoypadButton::L3),
        x if x == Button::R3 as u32 => Some(LrJoypadButton::R3),
        _ => None,
    }
}

/// Return a mutable reference to the current RuntimeHandle if available.
fn with_handle<R>(f: impl FnOnce(&mut RuntimeHandle) -> R) -> Option<R> {
    let mut s = state::global().lock().unwrap();
    if s.handle.is_null() {
        return None;
    }
    // SAFETY: handle pointer is set at start of `on_run` and guarded by the mutex.
    let h = unsafe { &mut *s.handle };
    Some(f(h))
}

/// Query whether a given joypad button is pressed.
///
/// Returns 1 if pressed, else 0.
pub fn joypad_button_pressed(port: u32, button: u32) -> u32 {
    let Some(btn) = map_joypad_button(button) else {
        return 0;
    };

    with_handle(|h| {
        if h.is_joypad_button_pressed(port, btn) {
            1
        } else {
            0
        }
    })
    .unwrap_or(0)
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
