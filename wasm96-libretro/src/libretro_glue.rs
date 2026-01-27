//! wasm96-libretro: Libretro frontend for wasm96-engine.

use crate::libretro_callbacks::LibretroCallbacks;
use crate::libretro_env;
use crate::platform;
use std::ffi::{CString, c_char, c_uint, c_void};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use wasm96_engine::Engine;
use wasm96_libretro_sys::*;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

/// All libretro glue state.
struct LibretroGlueState {
    engine: Option<Engine>,
    callbacks: LibretroCallbacks,
    hw_render: HwRenderCallback,
    hw_render_initialized: bool,
    sram: Vec<u8>,
}

impl LibretroGlueState {
    fn new() -> Self {
        Self {
            engine: None,
            callbacks: LibretroCallbacks::new(),
            hw_render: HwRenderCallback {
                context_type: 3, // HwContextType::OpenGLCore
                context_reset: context_reset,
                get_current_framebuffer: dummy_get_current_framebuffer,
                get_proc_address: dummy_get_proc_address,
                depth: false,
                stencil: false,
                bottom_left_origin: false,
                version_major: 3,
                version_minor: 3,
                cache_context: true,
                context_destroy: context_destroy,
                debug_context: false,
            },
            hw_render_initialized: false,
            sram: vec![0u8; 4 * 1024 * 1024], // 4MB default
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
static GLUE: OnceLock<Mutex<LibretroGlueState>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn glue() -> &'static Mutex<LibretroGlueState> {
    GLUE.get_or_init(|| Mutex::new(LibretroGlueState::new()))
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static GLUE_STATE: RefCell<LibretroGlueState> = RefCell::new(LibretroGlueState::new());
}

#[inline]
fn with_glue_mut<R>(f: impl FnOnce(&mut LibretroGlueState) -> R) -> R {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut g = glue().lock().unwrap();
        f(&mut *g)
    }
    #[cfg(target_arch = "wasm32")]
    {
        GLUE_STATE.with(|cell| f(&mut *cell.borrow_mut()))
    }
}

unsafe extern "C" fn dummy_get_current_framebuffer() -> usize {
    0
}
unsafe extern "C" fn dummy_proc() {}
unsafe extern "C" fn dummy_get_proc_address(_: *const c_char) -> unsafe extern "C" fn() {
    dummy_proc
}

/// Helper to resolve GL symbols
fn get_proc_address_wrapper(symbol: &str) -> *const c_void {
    let c_str = CString::new(symbol).unwrap();
    let get_proc = with_glue_mut(|g| g.hw_render.get_proc_address);
    unsafe { get_proc(c_str.as_ptr()) as *const c_void }
}

unsafe extern "C" fn context_reset() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        gl::load_with(get_proc_address_wrapper);
        let use_gles = with_glue_mut(|g| {
            matches!(
                g.hw_render.context_type,
                x if x == HwContextType::OpenGLES2 as u32
                    || x == HwContextType::OpenGLES3 as u32
                    || x == HwContextType::OpenGLESVersion as u32
            )
        });
        if !crate::gl_renderer::init_gl_renderer(use_gles) {
            return;
        }
        wasm96_engine::av::graphics3d::init_gl_context(get_proc_address_wrapper);
    }
    with_glue_mut(|g| {
        if let Some(e) = g.engine.as_mut() {
            e.reset();
        }
        g.hw_render_initialized = true;
    });
}

unsafe extern "C" fn context_destroy() {
    with_glue_mut(|g| g.hw_render_initialized = false);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_api_version() -> c_uint {
    API_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_init() {
    with_glue_mut(|g| {
        g.engine = Some(Engine::new());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_deinit() {
    with_glue_mut(|g| {
        g.engine = None;
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_environment(cb: Option<EnvironmentFn>) {
    with_glue_mut(|g| {
        g.callbacks.env = cb;

        // Log the selected platform policy bundle early, before we negotiate anything.
        // This helps debug target-specific behavior (e.g. aarch64 choosing GLES3/48kHz).
        eprintln!(
            "(wasm96) platform profile: {}",
            platform::platform_profile_name()
        );

        let req = platform::preferred_hw_context();
        g.hw_render.context_type = req.context_type;
        g.hw_render.version_major = req.version_major;
        g.hw_render.version_minor = req.version_minor;
        g.hw_render.bottom_left_origin = true;

        if let Some(env) = g.callbacks.env {
            unsafe {
                env(
                    ENVIRONMENT_SET_HW_RENDER,
                    (&raw mut g.hw_render) as *mut _ as *mut c_void,
                );
            }
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_video_refresh(cb: Option<VideoRefreshFn>) {
    with_glue_mut(|g| g.callbacks.video_refresh = cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_audio_sample(cb: Option<AudioSampleFn>) {
    with_glue_mut(|g| g.callbacks.audio_sample = cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_audio_sample_batch(cb: Option<AudioSampleBatchFn>) {
    with_glue_mut(|g| g.callbacks.audio_sample_batch = cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_input_poll(cb: Option<InputPollFn>) {
    with_glue_mut(|g| g.callbacks.input_poll = cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_input_state(cb: Option<InputStateFn>) {
    with_glue_mut(|g| g.callbacks.input_state = cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_system_info(info: *mut SystemInfo) {
    if info.is_null() {
        return;
    }
    let info = unsafe { &mut *info };
    info.library_name = b"wasm96\0".as_ptr() as *const c_char;
    info.library_version = b"0.1.2\0".as_ptr() as *const c_char;
    info.valid_extensions = b"w96|wasm|wat\0".as_ptr() as *const c_char;
    info.need_fullpath = false;
    info.block_extract = false;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut SystemAvInfo) {
    if info.is_null() {
        return;
    }
    let info = unsafe { &mut *info };
    info.geometry.base_width = 320;
    info.geometry.base_height = 240;
    info.geometry.max_width = 1920;
    info.geometry.max_height = 1080;
    info.geometry.aspect_ratio = 4.0 / 3.0;
    info.timing.fps = 60.0;
    info.timing.sample_rate = platform::preferred_audio_sample_rate_hz();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_load_game(game: *const GameInfo) -> bool {
    eprintln!("(wasm96-libretro) retro_load_game called");
    let env_cb = with_glue_mut(|g| g.callbacks.env);
    libretro_env::negotiate_pixel_format(env_cb);

    if game.is_null() {
        eprintln!("(wasm96-libretro) retro_load_game: game is null");
        return false;
    }
    let game = unsafe { &*game };
    let data_slice = unsafe { std::slice::from_raw_parts(game.data as *const u8, game.size) };
    eprintln!(
        "(wasm96-libretro) retro_load_game: data size={}",
        data_slice.len()
    );

    with_glue_mut(|g| {
        // Initialize VFS from SRAM
        {
            eprintln!(
                "(wasm96-libretro) retro_load_game: initializing VFS from SRAM size={}",
                g.sram.len()
            );
            let mut gs = wasm96_engine::state::global().lock().unwrap();
            let disk = wasm96_engine::vfs::VirtualDisk::from_bytes(g.sram.clone());
            if g.sram.iter().all(|&b| b == 0) {
                eprintln!("(wasm96-libretro) retro_load_game: SRAM empty, formatting VFS");
                let _ = disk.format("WASM96");
            }
            gs.vfs.mount_slot(0, disk);
        }

        if let Some(engine) = g.engine.as_mut() {
            eprintln!("(wasm96-libretro) retro_load_game: calling engine.load_game_from_bytes");
            match engine.load_game_from_bytes(data_slice) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("(wasm96) Failed to load game: {e:?}");
                    false
                }
            }
        } else {
            eprintln!("(wasm96-libretro) retro_load_game: engine is None");
            false
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_run() {
    let env_cb = with_glue_mut(|g| g.callbacks.env);
    libretro_env::maybe_emit_set_geometry(env_cb);

    with_glue_mut(|g| {
        let fbo = unsafe { (g.hw_render.get_current_framebuffer)() };
        g.callbacks.current_framebuffer = fbo;
        if let Some(engine) = g.engine.as_mut() {
            engine.run_frame(&mut g.callbacks);
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_reset() {
    with_glue_mut(|g| {
        if let Some(engine) = g.engine.as_mut() {
            engine.reset();
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_unload_game() {
    with_glue_mut(|g| {
        if let Some(engine) = g.engine.as_mut() {
            engine.unload();
        }
        let gs = wasm96_engine::state::global().lock().unwrap();
        if let Some(disk) = gs.vfs.disk() {
            g.sram = disk.export();
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_region() -> c_uint {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_memory_data(id: c_uint) -> *mut c_void {
    if id == 0 {
        // RETRO_MEMORY_SAVE_RAM
        with_glue_mut(|g| g.sram.as_mut_ptr() as *mut c_void)
    } else {
        ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_get_memory_size(id: c_uint) -> usize {
    if id == 0 {
        // RETRO_MEMORY_SAVE_RAM
        with_glue_mut(|g| g.sram.len())
    } else {
        0
    }
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
    _type: c_uint,
    _info: *const GameInfo,
    _num: usize,
) -> bool {
    false
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {}
