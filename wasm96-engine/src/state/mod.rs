//! Core-side shared state.
//!
//! This module owns the host-side state for the WASM runtime host functions.
//!
//! ABI model (Immediate Mode):
//! - Host owns the framebuffer and handles all rendering commands.
//! - Guest issues commands (draw rect, line, etc.) which modify the host framebuffer.
//! - Host presents the framebuffer to the platform frontend at the end of the frame.
//!
//! NOTE: Video pitch may be padded (row stride larger than visible width) on some
//! targets/drivers for compatibility/performance. Keep `width/height` as the visible
//! area and use `pitch_bytes` (or `stride_pixels`) for row stepping.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use wasmtime::Memory as WasmtimeMemory;

thread_local! {
    /// Thread-local storage for PlatformCallbacks during frame execution.
    ///
    /// This allows WASM imports to access platform callbacks without requiring
    /// them to be passed through Wasmtime's Caller<T> store data.
    static CALLBACKS: RefCell<Option<*mut dyn crate::PlatformCallbacks>> = RefCell::new(None);
}

/// A single host-side “audio channel” (a.k.a. a mixing voice).
///
/// This is used for higher-level playback APIs (e.g. `play_wav`, `play_ogg`, etc.)
/// where the host decodes and mixes audio. Guests get back an `id` that can be
/// adjusted (volume/pan/loop/stop) without pushing raw samples every frame.
///
/// NOTE: Actual decoding/mixing logic lives elsewhere (e.g. `av`); this is only state.
#[derive(Debug, Clone)]
pub struct AudioChannel {
    /// Whether this channel is currently active/playing.
    pub active: bool,

    /// Channel volume in Q8.8 fixed-point (256 = 1.0x).
    pub volume_q8_8: u32,

    /// Pan in i16 domain: -32768 = full left, 0 = center, 32767 = full right.
    pub pan_i16: i32,

    /// Whether playback should loop when reaching end.
    pub loop_enabled: bool,

    /// Interleaved stereo PCM samples (i16) for this channel.
    ///
    /// This is a simple representation that enables mixing without requiring the
    /// guest to continuously feed audio. Decoders can fill this buffer and reset
    /// `position_frames` as needed.
    pub pcm_stereo: Vec<i16>,

    /// Current playback position in *frames* (not i16 samples).
    /// One frame = 2 i16 samples (L, R).
    pub position_frames: usize,
}

impl Default for AudioChannel {
    fn default() -> Self {
        Self {
            active: false,
            volume_q8_8: 256,
            pan_i16: 0,
            loop_enabled: false,
            pcm_stereo: Vec::new(),
            position_frames: 0,
        }
    }
}

/// Global core state accessed from:
/// - `Engine::run_frame` (to set the current `RuntimeHandle`)
/// - host import functions
#[derive(Default)]
pub struct GlobalState {
    /// Guest linear memory export (`memory`) for the Wasmtime runtime.
    ///
    /// Stored as a raw pointer because the rest of the codebase accesses global state
    /// through a mutex-protected singleton and expects a stable address once set.
    pub memory_wasmtime: *const WasmtimeMemory,

    /// Owned copy of the memory handle to ensure the pointer above remains valid.
    pub memory_owned: Option<Box<WasmtimeMemory>>,

    /// Host-owned video state (system memory).
    pub video: VideoState,

    /// Host-owned audio state (system memory).
    pub audio: AudioState,

    /// Cached input state.
    pub input: InputState,

    /// Host-owned storage state (persistent-ish key/value store).
    pub storage: StorageState,
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
    /// Visible width (in pixels).
    pub width: u32,
    /// Visible height (in pixels).
    pub height: u32,

    /// Row stride in pixels (may be >= `width`).
    ///
    /// This allows us to align/pad rows for frontends/drivers (e.g. some ARM GPUs)
    /// while keeping the visible geometry unchanged.
    pub stride_pixels: u32,

    /// Cached pitch in bytes for XRGB8888 output.
    ///
    /// NOTE: XRGB8888 is 4 bytes/pixel, so `pitch_bytes = stride_pixels * 4`.
    pub pitch_bytes: usize,

    /// Framebuffer pixels (XRGB8888).
    ///
    /// Size is `stride_pixels * height` (NOT `width * height` when padded).
    /// Stored as `u32` for easy pixel manipulation.
    /// Format: 0x00RRGGBB (little endian in memory: BB GG RR 00).
    pub framebuffer: Vec<u32>,

    /// Current drawing color (packed 0x00RRGGBB for XRGB8888).
    pub draw_color: u32,

    /// Tracks whether geometry was last communicated to libretro for the current size.
    ///
    /// This is used so higher layers can request `RETRO_ENVIRONMENT_SET_GEOMETRY`
    /// only on changes.
    pub geometry_dirty: bool,
}

impl Default for VideoState {
    fn default() -> Self {
        let width = 320u32;
        let height = 240u32;
        let stride_pixels = width;
        let pitch_bytes = (stride_pixels as usize) * 4;

        Self {
            width, // Default size until set_size is called
            height,
            stride_pixels,
            pitch_bytes,
            framebuffer: vec![0; (stride_pixels * height) as usize],
            draw_color: 0x00FFFFFF, // Default white
            geometry_dirty: true,
        }
    }
}

/// Host-owned audio buffer state.
#[derive(Debug)]
pub struct AudioState {
    /// Output sample rate (what libretro expects).
    pub sample_rate: u32,

    /// Guest-provided staging buffer (interleaved i16 stereo).
    ///
    /// This is still supported for “raw push” style audio.
    pub host_queue: Vec<i16>,

    /// Host-mixed playback channels (decoded assets like WAV/QOA/M4A/OGG).
    ///
    /// Guests can trigger playback via higher-level audio APIs and the core will mix
    /// these channels into the output stream.
    pub channels: Vec<AudioChannel>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            host_queue: Vec::new(),

            channels: Vec::new(),
        }
    }
}

/// Host-owned storage state.
///
/// This is a simple in-memory key/value store used by the `storage` ABI.
/// Persistence (e.g. to disk via libretro save APIs) can be added later.
///
/// Keys and values are owned by the host.
#[derive(Debug, Default)]
pub struct StorageState {
    pub kv: HashMap<u64, Vec<u8>>,
}

/// Minimal cached input state.
#[derive(Default, Debug)]
pub struct InputState {
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_buttons: u32,
}

/// Set the guest memory for the Wasmtime runtime.
pub fn set_guest_memory_wasmtime(memory: &WasmtimeMemory) {
    let mut s = global().lock().unwrap();
    let boxed = Box::new(*memory);
    s.memory_wasmtime = &*boxed as *const _;
    s.memory_owned = Some(boxed);
}

pub fn clear_on_unload() {
    // If a previous panic occurred while holding the global lock, the mutex will be poisoned.
    // We still want to reset state in that case so other tests/frames can proceed.
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    s.memory_wasmtime = std::ptr::null();
    s.memory_owned = None;

    s.video = VideoState::default();
    s.audio = AudioState::default();
    s.input = InputState::default();
    s.storage = StorageState::default();
}

/// Snapshot of video state for querying without holding the lock.
#[derive(Debug, Clone, Copy)]
pub struct VideoStateSnapshot {
    pub width: u32,
    pub height: u32,
    pub stride_pixels: u32,
    pub geometry_dirty: bool,
}

/// Set the current platform callbacks for this thread.
///
/// SAFETY: The caller must ensure that the pointer remains valid for the duration
/// of the frame execution and that no other thread accesses these callbacks.
pub fn set_callbacks(callbacks: &mut dyn crate::PlatformCallbacks) {
    CALLBACKS.with(|c| {
        // SAFETY: The caller (Engine::run_frame) ensures that:
        // 1. The pointer remains valid for the duration of frame execution
        // 2. No other thread accesses these callbacks during this time
        // 3. clear_callbacks is called before the reference becomes invalid
        *c.borrow_mut() = Some(unsafe {
            std::mem::transmute::<
                &mut dyn crate::PlatformCallbacks,
                *mut dyn crate::PlatformCallbacks,
            >(callbacks)
        });
    });
}

/// Clear the current platform callbacks for this thread.
pub fn clear_callbacks() {
    CALLBACKS.with(|c| {
        *c.borrow_mut() = None;
    });
}

/// Execute a closure with access to the current platform callbacks.
///
/// Returns None if no callbacks are currently set.
pub fn with_callbacks<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut dyn crate::PlatformCallbacks) -> R,
{
    CALLBACKS.with(|c| {
        let callbacks_ptr = *c.borrow();
        callbacks_ptr.map(|ptr| {
            // SAFETY: We trust that the pointer is valid during frame execution.
            // The caller of set_callbacks must ensure this.
            let callbacks = unsafe { &mut *ptr };
            f(callbacks)
        })
    })
}
