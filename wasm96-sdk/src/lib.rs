#![cfg_attr(not(feature = "std"), no_std)]

//! wasm96-sdk (handwritten)
//!
//! This crate is used by **guest** WASM apps that run inside the `wasm96` libretro core.
//!
//! ABI model (Immediate Mode):
//! - Host owns the framebuffer and handles rendering.
//! - Guest issues drawing commands.
//! - Guest exports `setup`, `update`, and `draw`.
//!
//! This file intentionally contains **no WIT** and **no codegen**.

#[cfg(not(feature = "std"))]
extern crate alloc;

use core::ffi::c_void;

/// Joypad button ids.
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Button {
    B = 0,
    Y = 1,
    Select = 2,
    Start = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
    A = 8,
    X = 9,
    L1 = 10,
    R1 = 11,
    L2 = 12,
    R2 = 13,
    L3 = 14,
    R3 = 15,
}

/// Low-level raw ABI imports.
#[allow(non_camel_case_types)]
pub mod sys {
    unsafe extern "C" {
        // Graphics
        #[link_name = "wasm96_graphics_set_size"]
        pub fn graphics_set_size(width: u32, height: u32);
        #[link_name = "wasm96_graphics_set_color"]
        pub fn graphics_set_color(r: u32, g: u32, b: u32, a: u32);
        #[link_name = "wasm96_graphics_background"]
        pub fn graphics_background(r: u32, g: u32, b: u32);
        #[link_name = "wasm96_graphics_point"]
        pub fn graphics_point(x: i32, y: i32);
        #[link_name = "wasm96_graphics_line"]
        pub fn graphics_line(x1: i32, y1: i32, x2: i32, y2: i32);
        #[link_name = "wasm96_graphics_rect"]
        pub fn graphics_rect(x: i32, y: i32, w: u32, h: u32);
        #[link_name = "wasm96_graphics_rect_outline"]
        pub fn graphics_rect_outline(x: i32, y: i32, w: u32, h: u32);
        #[link_name = "wasm96_graphics_circle"]
        pub fn graphics_circle(x: i32, y: i32, r: u32);
        #[link_name = "wasm96_graphics_circle_outline"]
        pub fn graphics_circle_outline(x: i32, y: i32, r: u32);
        #[link_name = "wasm96_graphics_image"]
        pub fn graphics_image(x: i32, y: i32, w: u32, h: u32, ptr: u32, len: u32);

        // Input
        #[link_name = "wasm96_input_is_button_down"]
        pub fn input_is_button_down(port: u32, btn: u32) -> u32;
        #[link_name = "wasm96_input_is_key_down"]
        pub fn input_is_key_down(key: u32) -> u32;
        #[link_name = "wasm96_input_get_mouse_x"]
        pub fn input_get_mouse_x() -> i32;
        #[link_name = "wasm96_input_get_mouse_y"]
        pub fn input_get_mouse_y() -> i32;
        #[link_name = "wasm96_input_is_mouse_down"]
        pub fn input_is_mouse_down(btn: u32) -> u32;

        // Audio
        #[link_name = "wasm96_audio_init"]
        pub fn audio_init(sample_rate: u32) -> u32;
        #[link_name = "wasm96_audio_push_samples"]
        pub fn audio_push_samples(ptr: u32, len: u32);

        // System
        #[link_name = "wasm96_system_log"]
        pub fn system_log(ptr: u32, len: u32);
        #[link_name = "wasm96_system_millis"]
        pub fn system_millis() -> u64;
    }
}

/// Graphics API.
pub mod graphics {
    use super::sys;

    /// Set the screen dimensions.
    pub fn set_size(width: u32, height: u32) {
        unsafe { sys::graphics_set_size(width, height) }
    }

    /// Set the current drawing color (RGBA).
    pub fn set_color(r: u8, g: u8, b: u8, a: u8) {
        unsafe { sys::graphics_set_color(r as u32, g as u32, b as u32, a as u32) }
    }

    /// Clear the screen with a specific color (RGB).
    pub fn background(r: u8, g: u8, b: u8) {
        unsafe { sys::graphics_background(r as u32, g as u32, b as u32) }
    }

    /// Draw a single pixel at (x, y).
    pub fn point(x: i32, y: i32) {
        unsafe { sys::graphics_point(x, y) }
    }

    /// Draw a line from (x1, y1) to (x2, y2).
    pub fn line(x1: i32, y1: i32, x2: i32, y2: i32) {
        unsafe { sys::graphics_line(x1, y1, x2, y2) }
    }

    /// Draw a filled rectangle.
    pub fn rect(x: i32, y: i32, w: u32, h: u32) {
        unsafe { sys::graphics_rect(x, y, w, h) }
    }

    /// Draw a rectangle outline.
    pub fn rect_outline(x: i32, y: i32, w: u32, h: u32) {
        unsafe { sys::graphics_rect_outline(x, y, w, h) }
    }

    /// Draw a filled circle.
    pub fn circle(x: i32, y: i32, r: u32) {
        unsafe { sys::graphics_circle(x, y, r) }
    }

    /// Draw a circle outline.
    pub fn circle_outline(x: i32, y: i32, r: u32) {
        unsafe { sys::graphics_circle_outline(x, y, r) }
    }

    /// Draw an image/sprite.
    /// `data` is a slice of RGBA bytes (4 bytes per pixel).
    pub fn image(x: i32, y: i32, w: u32, h: u32, data: &[u8]) {
        unsafe { sys::graphics_image(x, y, w, h, data.as_ptr() as u32, data.len() as u32) }
    }
}

/// Input API.
pub mod input {
    use super::{Button, sys};

    /// Returns true if the specified button is currently held down.
    pub fn is_button_down(port: u32, btn: Button) -> bool {
        unsafe { sys::input_is_button_down(port, btn as u32) != 0 }
    }

    /// Returns true if the specified key is currently held down.
    pub fn is_key_down(key: u32) -> bool {
        unsafe { sys::input_is_key_down(key) != 0 }
    }

    /// Get current mouse X position.
    pub fn get_mouse_x() -> i32 {
        unsafe { sys::input_get_mouse_x() }
    }

    /// Get current mouse Y position.
    pub fn get_mouse_y() -> i32 {
        unsafe { sys::input_get_mouse_y() }
    }

    /// Returns true if the specified mouse button is held down.
    /// 0 = Left, 1 = Right, 2 = Middle.
    pub fn is_mouse_down(btn: u32) -> bool {
        unsafe { sys::input_is_mouse_down(btn) != 0 }
    }
}

/// Audio API.
pub mod audio {
    use super::sys;

    /// Initialize audio system.
    pub fn init(sample_rate: u32) -> u32 {
        unsafe { sys::audio_init(sample_rate) }
    }

    /// Push a chunk of audio samples.
    /// Samples are interleaved stereo (L, R, L, R...) signed 16-bit integers.
    pub fn push_samples(samples: &[i16]) {
        unsafe { sys::audio_push_samples(samples.as_ptr() as u32, samples.len() as u32) }
    }
}

/// System API.
pub mod system {
    use super::sys;

    /// Log a message to the host console.
    pub fn log(message: &str) {
        unsafe { sys::system_log(message.as_ptr() as u32, message.len() as u32) }
    }

    /// Get the number of milliseconds since the app started.
    pub fn millis() -> u64 {
        unsafe { sys::system_millis() }
    }
}

/// Convenience prelude for guest apps.
pub mod prelude {
    pub use crate::Button;
    pub use crate::audio;
    pub use crate::graphics;
    pub use crate::input;
    pub use crate::system;
}

// Keep `c_void` referenced so it doesn't look unused in some configurations.
#[allow(dead_code)]
const _C_VOID: *const c_void = core::ptr::null();
