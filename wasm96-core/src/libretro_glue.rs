use std::ffi::{CString, c_void};
use std::os::raw::{c_char, c_uint};
use std::ptr;

use libretro_sys::*;

use crate::Wasm96Core;
use crate::av::graphics3d;
use crate::state;

static mut CORE: Option<Wasm96Core> = None;

// Callbacks
static mut VIDEO_CB: Option<VideoRefreshFn> = None;
static mut AUDIO_CB: Option<AudioSampleFn> = None;
static mut AUDIO_BATCH_CB: Option<AudioSampleBatchFn> = None;
static mut INPUT_POLL_CB: Option<InputPollFn> = None;
static mut INPUT_STATE_CB: Option<InputStateFn> = None;
static mut ENV_CB: Option<EnvironmentFn> = None;

// Dummies for HW_RENDER
unsafe extern "C" fn dummy_get_current_framebuffer() -> usize {
    0
}
unsafe extern "C" fn dummy_proc() {}
unsafe extern "C" fn dummy_get_proc_address(_: *const c_char) -> unsafe extern "C" fn() {
    dummy_proc
}

// HW Render
static mut HW_RENDER: HwRenderCallback = HwRenderCallback {
    context_type: 3, // RETRO_HW_CONTEXT_OPENGL_CORE
    context_reset: context_reset,
    get_current_framebuffer: dummy_get_current_framebuffer,
    get_proc_address: dummy_get_proc_address,
    depth: true,
    stencil: true,
    bottom_left_origin: true,
    version_major: 3,
    version_minor: 3,
    cache_context: true,
    context_destroy: context_destroy,
    debug_context: false,
};

unsafe extern "C" fn context_reset() {
    // Initialize GL context
    graphics3d::init_gl_context(get_proc_address_wrapper);

    unsafe {
        if let Some(c) = (&mut *(&raw mut CORE)).as_mut() {
            c.reset();
        }
    }
}

unsafe extern "C" fn context_destroy() {
    // graphics3d::deinit_gl_context();
}

fn get_proc_address_wrapper(symbol: &str) -> *const c_void {
    unsafe {
        let get_proc = HW_RENDER.get_proc_address;
        let c_str = CString::new(symbol).unwrap();
        get_proc(c_str.as_ptr()) as *const c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_api_version() -> c_uint {
    API_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_init() {
    unsafe {
        CORE = Some(Wasm96Core::default());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_deinit() {
    unsafe {
        CORE = None;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_environment(cb: Option<EnvironmentFn>) {
    unsafe {
        ENV_CB = cb;

        // Enable HW Render
        if let Some(env) = ENV_CB {
            let ret = env(
                ENVIRONMENT_SET_HW_RENDER,
                &raw mut HW_RENDER as *mut _ as *mut c_void,
            );
            if !ret {
                eprintln!("Failed to set HW render environment");
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_video_refresh(cb: Option<VideoRefreshFn>) {
    unsafe {
        VIDEO_CB = cb;
    }
    state::set_video_refresh_cb(cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_audio_sample(cb: Option<AudioSampleFn>) {
    unsafe {
        AUDIO_CB = cb;
    }
    state::set_audio_sample_cb(cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_audio_sample_batch(cb: Option<AudioSampleBatchFn>) {
    unsafe {
        AUDIO_BATCH_CB = cb;
    }
    state::set_audio_sample_batch_cb(cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_input_poll(cb: Option<InputPollFn>) {
    unsafe {
        INPUT_POLL_CB = cb;
    }
    state::set_input_poll_cb(cb);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_set_input_state(cb: Option<InputStateFn>) {
    unsafe {
        INPUT_STATE_CB = cb;
    }
    state::set_input_state_cb(cb);
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
    // Default values, will be updated after load
    info.geometry.base_width = 320;
    info.geometry.base_height = 240;
    info.geometry.max_width = 1920;
    info.geometry.max_height = 1080;
    info.geometry.aspect_ratio = 0.0;

    info.timing.fps = 60.0;
    info.timing.sample_rate = 44100.0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_load_game(game: *const GameInfo) -> bool {
    let core = unsafe {
        match (&mut *(&raw mut CORE)).as_mut() {
            Some(c) => c,
            None => return false,
        }
    };

    if game.is_null() {
        return false;
    }
    let game = unsafe { &*game };

    let data_slice = unsafe { std::slice::from_raw_parts(game.data as *const u8, game.size) };

    match core.load_game_from_bytes(data_slice) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_run() {
    let core = unsafe {
        match (&mut *(&raw mut CORE)).as_mut() {
            Some(c) => c,
            None => return,
        }
    };

    // Poll input
    unsafe {
        if let Some(poll) = INPUT_POLL_CB {
            poll();
        }
    }

    // Prepare 3D frame
    unsafe {
        let fbo = (HW_RENDER.get_current_framebuffer)();
        graphics3d::prepare_frame(fbo);
    }

    // Run core frame
    core.run_frame();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_reset() {
    unsafe {
        if let Some(c) = (&mut *(&raw mut CORE)).as_mut() {
            c.reset();
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn retro_unload_game() {
    unsafe {
        if let Some(c) = (&mut *(&raw mut CORE)).as_mut() {
            c.unload();
        }
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
