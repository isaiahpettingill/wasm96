//! Core-side shared state.
//!
//! This module owns the host-side state that bridges libretro callbacks and the
//! Wasmer host functions.
//!
//! ABI model (Immediate Mode):
//! - Host owns the framebuffer and handles all rendering commands.
//! - Guest issues commands (draw rect, line, etc.) which modify the host framebuffer.
//! - Host presents the framebuffer to libretro at the end of the frame.

use libretro_backend::RuntimeHandle;
use std::sync::{Mutex, OnceLock};
use wasmer::Memory;

/// Global core state accessed from:
/// - `Core::on_run` (to set the current `RuntimeHandle`)
/// - Wasmer host import functions
#[derive(Default)]
pub struct GlobalState {
    /// Current libretro runtime handle.
    pub handle: *mut RuntimeHandle,

    /// Guest linear memory export (`memory`).
    pub memory: *mut Memory,

    /// Host-owned video state (system memory).
    pub video: VideoState,

    /// Host-owned audio state (system memory).
    pub audio: AudioState,

    /// Cached input state.
    pub input: InputState,
}

// Raw pointers are used for `handle` and `memory`. We guard access with a mutex.
unsafe impl Send for GlobalState {}
unsafe impl Sync for GlobalState {}

static GLOBAL_STATE: OnceLock<Mutex<GlobalState>> = OnceLock::new();

pub fn global() -> &'static Mutex<GlobalState> {
    GLOBAL_STATE.get_or_init(|| Mutex::new(GlobalState::default()))
}

/// Host-owned framebuffer state for immediate mode drawing.
#[derive(Debug)]
pub struct VideoState {
    pub width: u32,
    pub height: u32,

    /// Framebuffer pixels (XRGB8888).
    /// Size is width * height.
    /// Stored as `u32` for easy pixel manipulation.
    /// Format: 0x00RRGGBB (little endian in memory: BB GG RR 00).
    pub framebuffer: Vec<u32>,

    /// Current drawing color (packed 0x00RRGGBB for XRGB8888).
    pub draw_color: u32,
}

impl Default for VideoState {
    fn default() -> Self {
        Self {
            width: 320, // Default size until set_size is called
            height: 240,
            framebuffer: vec![0; 320 * 240],
            draw_color: 0x00FFFFFF, // Default white
        }
    }
}

/// Host-owned audio buffer state.
#[derive(Debug)]
pub struct AudioState {
    pub sample_rate: u32,

    /// Host-owned audio staging buffer (interleaved i16).
    pub host_queue: Vec<i16>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            host_queue: Vec::new(),
        }
    }
}

/// Minimal cached input state.
#[derive(Default, Debug)]
pub struct InputState {
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_buttons: u32,
}

pub fn set_runtime_handle(handle: &mut RuntimeHandle) {
    let mut s = global().lock().unwrap();
    s.handle = handle as *mut _;
}

pub fn set_guest_memory(memory: &Memory) {
    let mut s = global().lock().unwrap();
    s.memory = memory as *const _ as *mut _;
}

pub fn clear_on_unload() {
    let mut s = global().lock().unwrap();
    s.handle = std::ptr::null_mut();
    s.memory = std::ptr::null_mut();

    s.video = VideoState::default();
    s.audio = AudioState::default();
    s.input = InputState::default();
}
