//! Libretro glue layer for wasm96-engine.
//!
//! This module provides the libretro C API entry points and manages the bridge
//! between libretro callbacks and the platform-agnostic wasm96-engine.

use std::ffi::{CString, c_char, c_void};
use std::os::raw::c_uint;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use libretro_sys::*;

use wasm96_engine::Engine;

use crate::libretro_callbacks::LibretroCallbacks;
use crate::libretro_env;
use crate::platform;

/// All libretro glue state.
///
/// This is guarded by a `Mutex` to avoid UB/data races. Libretro itself is typically
/// single-threaded, but frontends *can* call into the core from different threads
/// depending on the driver; this keeps us on the safe side.
struct LibretroGlueState {
    engine: Option<Engine>,
    callbacks: LibretroCallbacks,

    // HW render callback struct (passed to frontend).
    hw_render: HwRenderCallback,

    // Prevent spamming warnings if HW render is rejected.
    printed_hw_render_warn: bool,

    // Track whether HW render has been initialized (to avoid duplicate setup).
    hw_render_initialized: bool,
}

impl LibretroGlueState {
    fn new() -> Self {
        Self {
            engine: None,
            callbacks: LibretroCallbacks::new(),
            hw_render: HwRenderCallback {
                // Will be overridden in `retro_set_environment` based on platform policy.
                context_type: 3, // HwContextType::OpenGLCore
                context_reset: context_reset,
                get_current_framebuffer: dummy_get_current_framebuffer,
                get_proc_address: dummy_get_proc_address,
                depth: true,
                stencil: true,
                bottom_left_origin: false,
                version_major: 3,
                version_minor: 3,
                cache_context: true,
                context_destroy: context_destroy,
                debug_context: false,
            },
            printed_hw_render_warn: false,
            hw_render_initialized: false,
        }
    }
}

static GLUE: OnceLock<Mutex<LibretroGlueState>> = OnceLock::new();

fn glue() -> &'static Mutex<LibretroGlueState> {
    GLUE.get_or_init(|| Mutex::new(LibretroGlueState::new()))
}

// Dummies for HW_RENDER bootstrap. Frontend overwrites these when HW render is accepted.
unsafe extern "C" fn dummy_get_current_framebuffer() -> usize {
    0
}
unsafe extern "C" fn dummy_proc() {}
unsafe extern "C" fn dummy_get_proc_address(_: *const c_char) -> unsafe extern "C" fn() {
    dummy_proc
}

/// Small helper used by GL init code to resolve GL symbols through the libretro HW callback.
fn get_proc_address_wrapper(symbol: &str) -> *const c_void {
    let c_str = CString::new(symbol).unwrap();
    let get_proc = {
        let g = glue().lock().unwrap();
        g.hw_render.get_proc_address
    };
    unsafe { get_proc(c_str.as_ptr()) as *const c_void }
}

/// Called by frontend when the HW context is created/reset.
unsafe extern "C" fn context_reset() {
    eprintln!("(wasm96) context_reset called - initializing GL context");
    // Initialize GL context
    wasm96_engine::av::graphics3d::init_gl_context(get_proc_address_wrapper);
    eprintln!("(wasm96) GL context initialized");

    // Reset engine state (guest may depend on GL resources).
    let mut g = glue().lock().unwrap();
    if let Some(e) = g.engine.as_mut() {
        e.reset();
    }
    eprintln!("(wasm96) context_reset complete");
}

/// Called by frontend when the HW context is destroyed.
/// We currently keep this as a no-op; add resource teardown if needed.
unsafe extern "C" fn context_destroy() {
    // wasm96_engine::av::graphics3d::deinit_gl_context();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_api_version() -> c_uint {
    API_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_init() {
    eprintln!("(wasm96) retro_init called");
    let mut g = glue().lock().unwrap();
    g.engine = Some(Engine::new());
    eprintln!("(wasm96) Engine initialized");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_deinit() {
    let mut g = glue().lock().unwrap();
    g.engine = None;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_environment(cb: Option<EnvironmentFn>) {
    eprintln!("(wasm96) retro_set_environment called");
    let mut g = glue().lock().unwrap();
    g.callbacks.env = cb;

    eprintln!(
        "(wasm96) platform profile: {}",
        platform::platform_profile_name()
    );

    // Apply platform-specific HW context request (OpenGL core vs GLES3).
    let req = platform::preferred_hw_context();
    g.hw_render.context_type = req.context_type;
    g.hw_render.version_major = req.version_major;
    g.hw_render.version_minor = req.version_minor;

    // Align libretro HW framebuffer origin with our top-left software buffer convention.
    g.hw_render.bottom_left_origin = false;

    // Enable HW Render (only on first call)
    if g.hw_render_initialized {
        eprintln!("(wasm96) HW render already initialized, skipping");
        return;
    }

    if let Some(env) = g.callbacks.env {
        let preferred_context_type = g.hw_render.context_type;
        let preferred_major = g.hw_render.version_major;
        let preferred_minor = g.hw_render.version_minor;

        let alt = if preferred_context_type == 2 {
            // Preferred: OpenGLES3 -> Alternate: OpenGLCore 3.3
            (3, 3, 3)
        } else {
            // Preferred: OpenGLCore -> Alternate: OpenGLES3 3.0
            (2, 3, 0)
        };

        let ret = unsafe {
            env(
                ENVIRONMENT_SET_HW_RENDER,
                (&raw mut g.hw_render) as *mut _ as *mut c_void,
            )
        };

        let ret = if ret {
            true
        } else {
            // Retry HW context negotiation once with the alternate context type.
            g.hw_render.context_type = alt.0;
            g.hw_render.version_major = alt.1;
            g.hw_render.version_minor = alt.2;

            eprintln!(
                "(wasm96) HW render request rejected; retrying with alternate context_type={} version={}.{}",
                g.hw_render.context_type, g.hw_render.version_major, g.hw_render.version_minor
            );

            unsafe {
                env(
                    ENVIRONMENT_SET_HW_RENDER,
                    (&raw mut g.hw_render) as *mut _ as *mut c_void,
                )
            }
        };

        if !ret {
            // Restore preferred request parameters (best-effort) so later logging/state reflects policy.
            g.hw_render.context_type = preferred_context_type;
            g.hw_render.version_major = preferred_major;
            g.hw_render.version_minor = preferred_minor;

            // Some frontends/drivers may reject HW render env setup during init.
            // This is not necessarily fatal (software mode can still run),
            // so avoid spamming or treating it as a hard error.
            if !g.printed_hw_render_warn {
                eprintln!("(wasm96) HW render environment not available; continuing without it");
                g.printed_hw_render_warn = true;
            }
        } else {
            eprintln!("(wasm96) HW render enabled successfully");
            g.hw_render_initialized = true;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_video_refresh(cb: Option<VideoRefreshFn>) {
    let mut g = glue().lock().unwrap();
    g.callbacks.video_refresh = cb;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_audio_sample(cb: Option<AudioSampleFn>) {
    let mut g = glue().lock().unwrap();
    g.callbacks.audio_sample = cb;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_audio_sample_batch(cb: Option<AudioSampleBatchFn>) {
    let mut g = glue().lock().unwrap();
    g.callbacks.audio_sample_batch = cb;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_input_poll(cb: Option<InputPollFn>) {
    let mut g = glue().lock().unwrap();
    g.callbacks.input_poll = cb;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_input_state(cb: Option<InputStateFn>) {
    let mut g = glue().lock().unwrap();
    g.callbacks.input_state = cb;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_system_info(info: *mut SystemInfo) {
    let info = unsafe { &mut *info };
    info.library_name = b"Wasm96\0".as_ptr() as *const c_char;
    info.library_version = b"1.0.0\0".as_ptr() as *const c_char;
    info.valid_extensions = b"wasm|wat|w96\0".as_ptr() as *const c_char;
    info.need_fullpath = false;
    info.block_extract = false;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut SystemAvInfo) {
    let info = unsafe { &mut *info };
    // Default values; frontend may query before load.
    info.geometry.base_width = 320;
    info.geometry.base_height = 240;
    info.geometry.max_width = 1920;
    info.geometry.max_height = 1080;
    info.geometry.aspect_ratio = 0.0;

    info.timing.fps = 60.0;
    info.timing.sample_rate = platform::preferred_audio_sample_rate_hz();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_load_game(game: *const GameInfo) -> bool {
    eprintln!("(wasm96) retro_load_game called");
    // Pixel format negotiation (mandatory for correct 32-bit color + stride handling).
    // Copy env cb out under lock, then call helper without holding mutex.
    let env_cb = { glue().lock().unwrap().callbacks.env };
    libretro_env::negotiate_pixel_format(env_cb);

    if game.is_null() {
        return false;
    }
    let game = unsafe { &*game };
    let data_slice = unsafe { std::slice::from_raw_parts(game.data as *const u8, game.size) };

    let mut g = glue().lock().unwrap();
    let Some(engine) = g.engine.as_mut() else {
        return false;
    };

    match engine.load_game_from_bytes(data_slice) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("(wasm96) Failed to load game content: {e:?}");
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_run() {
    // If internal geometry changed, inform frontend before presenting frames.
    let env_cb = { glue().lock().unwrap().callbacks.env };
    libretro_env::maybe_emit_set_geometry(env_cb);

    // Get current HW framebuffer (only if we have a valid HW framebuffer).
    let (fbo, do_prepare) = {
        let g = glue().lock().unwrap();
        let fbo = unsafe { (g.hw_render.get_current_framebuffer)() };
        (fbo, fbo != 0)
    };

    if do_prepare {
        wasm96_engine::av::graphics3d::prepare_frame(fbo);
    }

    // Run engine frame
    let mut g = glue().lock().unwrap();

    // Update framebuffer in callbacks
    g.callbacks.current_framebuffer = fbo;

    // We need to split the borrow - take callbacks out temporarily
    let callbacks_ptr = &mut g.callbacks as *mut LibretroCallbacks;

    if let Some(engine) = g.engine.as_mut() {
        // SAFETY: We hold the mutex lock for the entire duration, ensuring exclusive access
        unsafe {
            engine.run_frame(&mut *callbacks_ptr);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_reset() {
    let mut g = glue().lock().unwrap();
    if let Some(e) = g.engine.as_mut() {
        e.reset();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_unload_game() {
    let mut g = glue().lock().unwrap();
    if let Some(e) = g.engine.as_mut() {
        e.unload();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_region() -> c_uint {
    0 // RETRO_REGION_NTSC
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_memory_size(_id: c_uint) -> usize {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_serialize_size() -> usize {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_serialize(_data: *mut c_void, _size: usize) -> bool {
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_unserialize(_data: *const c_void, _size: usize) -> bool {
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_cheat_reset() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_cheat_set(_index: c_uint, _enabled: bool, _code: *const c_char) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_load_game_special(
    _id: c_uint,
    _info: *const GameInfo,
    _num_info: usize,
) -> bool {
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {}
