//! Minimal libretro C ABI bindings for wasm96.
//!
//! Goals:
//! - Provide the libretro types/constants needed by `wasm96-libretro`.
//! - Avoid depending on `libc` (which can be problematic on some wasm32 toolchains).
//! - Stay small: only include what the project currently uses.
//!
//! Notes:
//! - This is not a complete libretro API surface.
//! - The numeric values match the libretro C headers (`libretro.h`) where applicable.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use core::ffi::{c_char, c_void};

pub type c_uint = u32;

// ---------------------------------------------
// Core constants
// ---------------------------------------------

pub const API_VERSION: c_uint = 1;

// ---------------------------------------------
// Device constants (libretro input)
// ---------------------------------------------

pub const DEVICE_TYPE_SHIFT: c_uint = 8;
pub const DEVICE_MASK: c_uint = (1 << DEVICE_TYPE_SHIFT) - 1;

pub const DEVICE_NONE: c_uint = 0;
pub const DEVICE_JOYPAD: c_uint = 1;
pub const DEVICE_MOUSE: c_uint = 2;
pub const DEVICE_KEYBOARD: c_uint = 3;
pub const DEVICE_LIGHTGUN: c_uint = 4;
pub const DEVICE_ANALOG: c_uint = 5;
pub const DEVICE_POINTER: c_uint = 6;

// Joypad IDs (RETRO_DEVICE_ID_JOYPAD_*)
pub const DEVICE_ID_JOYPAD_B: c_uint = 0;
pub const DEVICE_ID_JOYPAD_Y: c_uint = 1;
pub const DEVICE_ID_JOYPAD_SELECT: c_uint = 2;
pub const DEVICE_ID_JOYPAD_START: c_uint = 3;
pub const DEVICE_ID_JOYPAD_UP: c_uint = 4;
pub const DEVICE_ID_JOYPAD_DOWN: c_uint = 5;
pub const DEVICE_ID_JOYPAD_LEFT: c_uint = 6;
pub const DEVICE_ID_JOYPAD_RIGHT: c_uint = 7;
pub const DEVICE_ID_JOYPAD_A: c_uint = 8;
pub const DEVICE_ID_JOYPAD_X: c_uint = 9;
pub const DEVICE_ID_JOYPAD_L: c_uint = 10;
pub const DEVICE_ID_JOYPAD_R: c_uint = 11;
pub const DEVICE_ID_JOYPAD_L2: c_uint = 12;
pub const DEVICE_ID_JOYPAD_R2: c_uint = 13;
pub const DEVICE_ID_JOYPAD_L3: c_uint = 14;
pub const DEVICE_ID_JOYPAD_R3: c_uint = 15;

// Mouse IDs (RETRO_DEVICE_ID_MOUSE_*)
pub const DEVICE_ID_MOUSE_X: c_uint = 0;
pub const DEVICE_ID_MOUSE_Y: c_uint = 1;
pub const DEVICE_ID_MOUSE_LEFT: c_uint = 2;
pub const DEVICE_ID_MOUSE_RIGHT: c_uint = 3;
pub const DEVICE_ID_MOUSE_MIDDLE: c_uint = 4;

// ---------------------------------------------
// Pixel formats / video constants
// ---------------------------------------------

/// RETRO_PIXEL_FORMAT_*
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    _0RGB1555 = 0,
    XRGB8888 = 1,
    RGB565 = 2,
}

pub const HW_FRAME_BUFFER_VALID: usize = (-1isize) as usize;

// ---------------------------------------------
// Environment commands (retro_environment)
// ---------------------------------------------

pub const ENVIRONMENT_SET_PIXEL_FORMAT: c_uint = 10;
pub const ENVIRONMENT_SET_HW_RENDER: c_uint = 14;
pub const ENVIRONMENT_SET_GEOMETRY: c_uint = 37;

// ---------------------------------------------
// HW context type (retro_hw_context_type)
// ---------------------------------------------

/// RETRO_HW_CONTEXT_*
///
/// This is a subset: include only what wasm96 uses.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HwContextType {
    OpenGL = 1,
    OpenGLES2 = 2,
    OpenGLCore = 3,
    OpenGLES3 = 4,

    // Present in libretro for "request a specific GLES version" via major/minor fields.
    // Kept because wasm96 checks for it.
    OpenGLESVersion = 5,
}

// ---------------------------------------------
// Function pointer types
// ---------------------------------------------

pub type EnvironmentFn = unsafe extern "C" fn(cmd: c_uint, data: *mut c_void) -> bool;

pub type VideoRefreshFn =
    unsafe extern "C" fn(data: *const c_void, width: c_uint, height: c_uint, pitch: usize);

pub type AudioSampleFn = unsafe extern "C" fn(left: i16, right: i16);

pub type AudioSampleBatchFn = unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;

pub type InputPollFn = unsafe extern "C" fn();

pub type InputStateFn =
    unsafe extern "C" fn(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16;

// ---------------------------------------------
// Structs used by exported entry points
// ---------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GameGeometry {
    pub base_width: c_uint,
    pub base_height: c_uint,
    pub max_width: c_uint,
    pub max_height: c_uint,
    pub aspect_ratio: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SystemTiming {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SystemAvInfo {
    pub geometry: GameGeometry,
    pub timing: SystemTiming,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

// ---------------------------------------------
// HW render callback (retro_hw_render_callback)
// ---------------------------------------------

pub type HwGetCurrentFramebufferFn = unsafe extern "C" fn() -> usize;
pub type HwGetProcAddressFn = unsafe extern "C" fn(sym: *const c_char) -> unsafe extern "C" fn();

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HwRenderCallback {
    pub context_type: c_uint,
    pub context_reset: unsafe extern "C" fn(),
    pub get_current_framebuffer: HwGetCurrentFramebufferFn,
    pub get_proc_address: HwGetProcAddressFn,

    pub depth: bool,
    pub stencil: bool,
    pub bottom_left_origin: bool,

    pub version_major: c_uint,
    pub version_minor: c_uint,

    pub cache_context: bool,
    pub context_destroy: unsafe extern "C" fn(),
    pub debug_context: bool,
}
