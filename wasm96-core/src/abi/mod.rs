//! wasm96-core ABI module
//!
//! This module defines the ABI contract between:
//! - **Host**: `wasm96-core` (libretro core)
//! - **Guest**: the loaded WASM module (“game/app”)
//!
//! ## High-level model (Immediate Mode)
//! The host owns the framebuffer and handles all rendering.
//! The guest issues drawing commands (draw rect, draw line, etc.) during its `draw` loop.
//!
//! ## Imports (guest -> host)
//! Imported from module `"env"`.
//!
//! ### Graphics
//! - `wasm96_graphics_set_size(width: u32, height: u32)`
//! - `wasm96_graphics_set_color(r: u32, g: u32, b: u32, a: u32)`
//! - `wasm96_graphics_background(r: u32, g: u32, b: u32)`
//! - `wasm96_graphics_point(x: i32, y: i32)`
//! - `wasm96_graphics_line(x1: i32, y1: i32, x2: i32, y2: i32)`
//! - `wasm96_graphics_rect(x: i32, y: i32, w: u32, h: u32)`
//! - `wasm96_graphics_rect_outline(x: i32, y: i32, w: u32, h: u32)`
//! - `wasm96_graphics_circle(x: i32, y: i32, r: u32)`
//! - `wasm96_graphics_circle_outline(x: i32, y: i32, r: u32)`
//!
//! Raw RGBA blit:
//! - `wasm96_graphics_image(x: i32, y: i32, w: u32, h: u32, ptr: u32, len: u32)`
//!
//! Keyed resources (no numeric ids required in the guest):
//! - `wasm96_graphics_svg_register(key_ptr: u32, key_len: u32, data_ptr: u32, data_len: u32) -> u32` (bool)
//! - `wasm96_graphics_svg_draw_key(key_ptr: u32, key_len: u32, x: i32, y: i32, w: u32, h: u32)`
//! - `wasm96_graphics_svg_unregister(key_ptr: u32, key_len: u32)`
//!
//! - `wasm96_graphics_gif_register(key_ptr: u32, key_len: u32, data_ptr: u32, data_len: u32) -> u32` (bool)
//! - `wasm96_graphics_gif_draw_key(key_ptr: u32, key_len: u32, x: i32, y: i32)`
//! - `wasm96_graphics_gif_draw_key_scaled(key_ptr: u32, key_len: u32, x: i32, y: i32, w: u32, h: u32)`
//! - `wasm96_graphics_gif_unregister(key_ptr: u32, key_len: u32)`
//!
//! - `wasm96_graphics_png_register(key_ptr: u32, key_len: u32, data_ptr: u32, data_len: u32) -> u32` (bool)
//! - `wasm96_graphics_png_draw_key(key_ptr: u32, key_len: u32, x: i32, y: i32)`
//! - `wasm96_graphics_png_draw_key_scaled(key_ptr: u32, key_len: u32, x: i32, y: i32, w: u32, h: u32)`
//! - `wasm96_graphics_png_unregister(key_ptr: u32, key_len: u32)`
//!
//! Fonts (keyed; special key `"spleen"` refers to the built-in Spleen font):
//! - `wasm96_graphics_font_register_ttf(key_ptr: u32, key_len: u32, data_ptr: u32, data_len: u32) -> u32` (bool)
//! - `wasm96_graphics_font_register_spleen(key_ptr: u32, key_len: u32, size: u32) -> u32` (bool)
//! - `wasm96_graphics_font_unregister(key_ptr: u32, key_len: u32)`
//! - `wasm96_graphics_text_key(x: i32, y: i32, font_key_ptr: u32, font_key_len: u32, text_ptr: u32, text_len: u32)`
//! - `wasm96_graphics_text_measure_key(font_key_ptr: u32, font_key_len: u32, text_ptr: u32, text_len: u32) -> u64`
//!
//! ### Input
//! - `wasm96_input_is_button_down(port: u32, btn: u32) -> u32` (bool)
//! - `wasm96_input_is_key_down(key: u32) -> u32` (bool)
//! - `wasm96_input_get_mouse_x() -> i32`
//! - `wasm96_input_get_mouse_y() -> i32`
//! - `wasm96_input_is_mouse_down(btn: u32) -> u32` (bool)
//!
//! ### Audio
//! - `wasm96_audio_init(sample_rate: u32) -> u32`
//! - `wasm96_audio_push_samples(ptr: u32, len: u32)`
//!
//! // Higher-level audio playback (host-mixed "channels/voices"):
//! - `wasm96_audio_play_wav(ptr: u32, len: u32)`
//! - `wasm96_audio_play_qoa(ptr: u32, len: u32)`
//! - `wasm96_audio_play_xm(ptr: u32, len: u32)`
//!
//! ### Storage
//! - `wasm96_storage_save(key_ptr: u32, key_len: u32, data_ptr: u32, data_len: u32)`
//! - `wasm96_storage_load(key_ptr: u32, key_len: u32) -> u64`
//!   - returns (ptr<<32)|len in guest memory; ptr=0,len=0 means “missing”
//! - `wasm96_storage_free(ptr: u32, len: u32)`
//!
//! ### System
//! - `wasm96_system_log(ptr: u32, len: u32)`
//! - `wasm96_system_millis() -> u64`
//!
//! ## Exports (host -> guest) required
//! The guest module **must** export:
//! - `setup()`
//! - `update()`
//! - `draw()`

use wasmer::Function;

/// Wasmer import module name used by the guest.
pub const IMPORT_MODULE: &str = "env";

/// Guest export names (entrypoints).
pub mod guest_exports {
    /// Called once on startup.
    pub const SETUP: &str = "setup";
    /// Called once per frame to update logic.
    pub const UPDATE: &str = "update";
    /// Called once per frame to draw.
    pub const DRAW: &str = "draw";
}

/// Host import names provided to the guest.
pub mod host_imports {
    // Graphics
    pub const GRAPHICS_SET_SIZE: &str = "wasm96_graphics_set_size";
    pub const GRAPHICS_SET_COLOR: &str = "wasm96_graphics_set_color";
    pub const GRAPHICS_BACKGROUND: &str = "wasm96_graphics_background";
    pub const GRAPHICS_POINT: &str = "wasm96_graphics_point";
    pub const GRAPHICS_LINE: &str = "wasm96_graphics_line";
    pub const GRAPHICS_RECT: &str = "wasm96_graphics_rect";
    pub const GRAPHICS_RECT_OUTLINE: &str = "wasm96_graphics_rect_outline";
    pub const GRAPHICS_CIRCLE: &str = "wasm96_graphics_circle";
    pub const GRAPHICS_CIRCLE_OUTLINE: &str = "wasm96_graphics_circle_outline";

    // Raw RGBA blit
    pub const GRAPHICS_IMAGE: &str = "wasm96_graphics_image";

    // Keyed resources: SVG
    pub const GRAPHICS_SVG_REGISTER: &str = "wasm96_graphics_svg_register";
    pub const GRAPHICS_SVG_DRAW_KEY: &str = "wasm96_graphics_svg_draw_key";
    pub const GRAPHICS_SVG_UNREGISTER: &str = "wasm96_graphics_svg_unregister";

    // Keyed resources: GIF
    pub const GRAPHICS_GIF_REGISTER: &str = "wasm96_graphics_gif_register";
    pub const GRAPHICS_GIF_DRAW_KEY: &str = "wasm96_graphics_gif_draw_key";
    pub const GRAPHICS_GIF_DRAW_KEY_SCALED: &str = "wasm96_graphics_gif_draw_key_scaled";
    pub const GRAPHICS_GIF_UNREGISTER: &str = "wasm96_graphics_gif_unregister";

    // Keyed resources: PNG
    pub const GRAPHICS_PNG_REGISTER: &str = "wasm96_graphics_png_register";
    pub const GRAPHICS_PNG_DRAW_KEY: &str = "wasm96_graphics_png_draw_key";
    pub const GRAPHICS_PNG_DRAW_KEY_SCALED: &str = "wasm96_graphics_png_draw_key_scaled";
    pub const GRAPHICS_PNG_UNREGISTER: &str = "wasm96_graphics_png_unregister";

    // Shapes
    pub const GRAPHICS_TRIANGLE: &str = "wasm96_graphics_triangle";
    pub const GRAPHICS_TRIANGLE_OUTLINE: &str = "wasm96_graphics_triangle_outline";
    pub const GRAPHICS_BEZIER_QUADRATIC: &str = "wasm96_graphics_bezier_quadratic";
    pub const GRAPHICS_BEZIER_CUBIC: &str = "wasm96_graphics_bezier_cubic";
    pub const GRAPHICS_PILL: &str = "wasm96_graphics_pill";
    pub const GRAPHICS_PILL_OUTLINE: &str = "wasm96_graphics_pill_outline";

    // Fonts (keyed)
    pub const GRAPHICS_FONT_REGISTER_TTF: &str = "wasm96_graphics_font_register_ttf";
    pub const GRAPHICS_FONT_REGISTER_SPLEEN: &str = "wasm96_graphics_font_register_spleen";
    pub const GRAPHICS_FONT_UNREGISTER: &str = "wasm96_graphics_font_unregister";
    pub const GRAPHICS_TEXT_KEY: &str = "wasm96_graphics_text_key";
    pub const GRAPHICS_TEXT_MEASURE_KEY: &str = "wasm96_graphics_text_measure_key";

    // Input
    pub const INPUT_IS_BUTTON_DOWN: &str = "wasm96_input_is_button_down";
    pub const INPUT_IS_KEY_DOWN: &str = "wasm96_input_is_key_down";
    pub const INPUT_GET_MOUSE_X: &str = "wasm96_input_get_mouse_x";
    pub const INPUT_GET_MOUSE_Y: &str = "wasm96_input_get_mouse_y";
    pub const INPUT_IS_MOUSE_DOWN: &str = "wasm96_input_is_mouse_down";

    // Audio
    pub const AUDIO_INIT: &str = "wasm96_audio_init";
    pub const AUDIO_PUSH_SAMPLES: &str = "wasm96_audio_push_samples";

    // High-level audio playback (decoded + mixed on host)
    // Fire-and-forget (no ids/handles returned).
    pub const AUDIO_PLAY_WAV: &str = "wasm96_audio_play_wav";
    pub const AUDIO_PLAY_QOA: &str = "wasm96_audio_play_qoa";
    pub const AUDIO_PLAY_XM: &str = "wasm96_audio_play_xm";

    // Storage
    pub const STORAGE_SAVE: &str = "wasm96_storage_save";
    pub const STORAGE_LOAD: &str = "wasm96_storage_load";
    pub const STORAGE_FREE: &str = "wasm96_storage_free";

    // System
    pub const SYSTEM_LOG: &str = "wasm96_system_log";
    pub const SYSTEM_MILLIS: &str = "wasm96_system_millis";
}

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

/// Helpers for validating guest exports.
pub mod validate {
    use super::guest_exports;
    use wasmer::Instance;

    pub fn required_exports_present(instance: &Instance) -> Result<(), MissingExport> {
        if instance.exports.get_function(guest_exports::SETUP).is_err() {
            return Err(MissingExport::Setup);
        }
        if instance
            .exports
            .get_function(guest_exports::UPDATE)
            .is_err()
        {
            return Err(MissingExport::Update);
        }
        if instance.exports.get_function(guest_exports::DRAW).is_err() {
            return Err(MissingExport::Draw);
        }
        Ok(())
    }

    #[derive(Debug)]
    pub enum MissingExport {
        Setup,
        Update,
        Draw,
    }
}

/// A small view of a guest's entrypoints as `wasmer::Function`s.
#[derive(Clone)]
pub struct GuestEntrypoints {
    pub setup: Function,
    pub update: Function,
    pub draw: Function,
}

impl GuestEntrypoints {
    /// Resolve entrypoint exports from an instance.
    pub fn resolve(instance: &wasmer::Instance) -> Result<Self, wasmer::ExportError> {
        let setup = instance.exports.get_function(guest_exports::SETUP)?.clone();
        let update = instance
            .exports
            .get_function(guest_exports::UPDATE)?
            .clone();
        let draw = instance.exports.get_function(guest_exports::DRAW)?.clone();

        Ok(Self {
            setup,
            update,
            draw,
        })
    }
}
