//! Libretro callbacks adapter for wasm96-engine.
//!
//! This module implements the PlatformCallbacks trait required by wasm96-engine,
//! translating between the engine's platform-agnostic API and libretro's specific
//! callback functions.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm96_libretro_sys::{
    AudioSampleBatchFn, AudioSampleFn, DEVICE_ID_JOYPAD_A, DEVICE_ID_JOYPAD_B,
    DEVICE_ID_JOYPAD_DOWN, DEVICE_ID_JOYPAD_L, DEVICE_ID_JOYPAD_L2, DEVICE_ID_JOYPAD_L3,
    DEVICE_ID_JOYPAD_LEFT, DEVICE_ID_JOYPAD_R, DEVICE_ID_JOYPAD_R2, DEVICE_ID_JOYPAD_R3,
    DEVICE_ID_JOYPAD_RIGHT, DEVICE_ID_JOYPAD_SELECT, DEVICE_ID_JOYPAD_START, DEVICE_ID_JOYPAD_UP,
    DEVICE_ID_JOYPAD_X, DEVICE_ID_JOYPAD_Y, DEVICE_ID_MOUSE_LEFT, DEVICE_ID_MOUSE_MIDDLE,
    DEVICE_ID_MOUSE_RIGHT, DEVICE_ID_MOUSE_X, DEVICE_ID_MOUSE_Y, DEVICE_JOYPAD, DEVICE_KEYBOARD,
    DEVICE_MOUSE, ENVIRONMENT_SET_GEOMETRY, EnvironmentFn, HW_FRAME_BUFFER_VALID, InputPollFn,
    InputStateFn, VideoRefreshFn,
};

use wasm96_engine::{PlatformAudio, PlatformCallbacks, PlatformGraphics, PlatformInput};

#[cfg(not(target_arch = "wasm32"))]
use crate::gl_renderer;

static LOG_ONCE_SW: AtomicBool = AtomicBool::new(false);
static LOG_ONCE_HW: AtomicBool = AtomicBool::new(false);
static LOG_ONCE_PREP: AtomicBool = AtomicBool::new(false);

/// Libretro-specific implementation of PlatformCallbacks.
pub struct LibretroCallbacks {
    pub video_refresh: Option<VideoRefreshFn>,
    pub audio_sample: Option<AudioSampleFn>,
    pub audio_sample_batch: Option<AudioSampleBatchFn>,
    pub input_poll: Option<InputPollFn>,
    pub input_state: Option<InputStateFn>,
    pub env: Option<EnvironmentFn>,

    /// Current hardware rendering framebuffer (0 if not available).
    pub current_framebuffer: usize,
}

impl LibretroCallbacks {
    pub fn new() -> Self {
        Self {
            video_refresh: None,
            audio_sample: None,
            audio_sample_batch: None,
            input_poll: None,
            input_state: None,
            env: None,
            current_framebuffer: 0,
        }
    }

    /// Map generic button index back to libretro device ID.
    fn map_button_to_libretro(button: u32) -> Option<u32> {
        match button {
            0 => Some(DEVICE_ID_JOYPAD_B),
            1 => Some(DEVICE_ID_JOYPAD_Y),
            2 => Some(DEVICE_ID_JOYPAD_SELECT),
            3 => Some(DEVICE_ID_JOYPAD_START),
            4 => Some(DEVICE_ID_JOYPAD_UP),
            5 => Some(DEVICE_ID_JOYPAD_DOWN),
            6 => Some(DEVICE_ID_JOYPAD_LEFT),
            7 => Some(DEVICE_ID_JOYPAD_RIGHT),
            8 => Some(DEVICE_ID_JOYPAD_A),
            9 => Some(DEVICE_ID_JOYPAD_X),
            10 => Some(DEVICE_ID_JOYPAD_L),
            11 => Some(DEVICE_ID_JOYPAD_R),
            12 => Some(DEVICE_ID_JOYPAD_L2),
            13 => Some(DEVICE_ID_JOYPAD_R2),
            14 => Some(DEVICE_ID_JOYPAD_L3),
            15 => Some(DEVICE_ID_JOYPAD_R3),
            _ => None,
        }
    }

    /// Map generic mouse button index back to libretro device ID.
    fn map_mouse_button_to_libretro(button: u32) -> Option<u32> {
        match button {
            0 => Some(DEVICE_ID_MOUSE_LEFT),
            1 => Some(DEVICE_ID_MOUSE_RIGHT),
            2 => Some(DEVICE_ID_MOUSE_MIDDLE),
            _ => None,
        }
    }
}

impl PlatformGraphics for LibretroCallbacks {
    fn prepare_frame(&mut self, width: u32, height: u32) {
        // Native: prepare GL rendering if hardware framebuffer is available.
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.current_framebuffer != 0 {
                if !LOG_ONCE_PREP.swap(true, Ordering::Relaxed) {
                    eprintln!(
                        "(wasm96-libretro) prepare_frame: using HW FBO={} size={}x{}",
                        self.current_framebuffer, width, height
                    );
                }
                gl_renderer::prepare_frame(self.current_framebuffer as u32, width, height);
            } else if !LOG_ONCE_PREP.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "(wasm96-libretro) prepare_frame: no HW FBO (software path) size={}x{}",
                    width, height
                );
            }
        }

        // wasm32 (RetroArch Web): wgpu handles per-frame setup internally.
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (width, height);
        }
    }

    fn present_software_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        stride_pixels: u32,
    ) {
        if !LOG_ONCE_SW.swap(true, Ordering::Relaxed) {
            eprintln!(
                "(wasm96-libretro) present_software_frame: fb_len={} size={}x{} stride_pixels={} pitch_bytes={} hw_fbo={}",
                framebuffer.len(),
                width,
                height,
                stride_pixels,
                (stride_pixels * 4) as usize,
                self.current_framebuffer
            );
        }

        // Native-only: keep GL clear behavior consistent when a HW FBO exists.
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.current_framebuffer != 0 {
                let ok = gl_renderer::clear_framebuffer(0.0, 0.0, 0.0, 1.0);
                if !ok && LOG_ONCE_SW.load(Ordering::Relaxed) {
                    eprintln!(
                        "(wasm96-libretro) present_software_frame: gl clear failed (renderer not ready / invalid FBO)"
                    );
                }
            }
        }

        if let Some(cb) = self.video_refresh {
            let data_ptr = framebuffer.as_ptr() as *const c_void;
            let pitch_bytes = (stride_pixels * 4) as usize;
            unsafe {
                cb(data_ptr, width, height, pitch_bytes);
            }
        } else if LOG_ONCE_SW.load(Ordering::Relaxed) {
            eprintln!("(wasm96-libretro) present_software_frame: video_refresh callback is None");
        }
    }

    fn present_hardware_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        stride_pixels: u32,
    ) {
        if !LOG_ONCE_HW.swap(true, Ordering::Relaxed) {
            eprintln!(
                "(wasm96-libretro) present_hardware_frame: fb_len={} size={}x{} stride_pixels={} hw_fbo={}",
                framebuffer.len(),
                width,
                height,
                stride_pixels,
                self.current_framebuffer
            );
        }

        // Native: upload framebuffer to GL texture and composite to the libretro-provided FBO.
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.current_framebuffer != 0 {
                gl_renderer::prepare_frame(self.current_framebuffer as u32, width, height);
            }

            let ok = gl_renderer::composite_frame(framebuffer, width, height, stride_pixels);
            if !ok && LOG_ONCE_HW.load(Ordering::Relaxed) {
                eprintln!(
                    "(wasm96-libretro) present_hardware_frame: composite_frame returned false (renderer not ready / invalid FBO)"
                );
            }
        }

        // wasm32 (RetroArch Web): wgpu path (stub for now).
        // This file is updated to route away from the native GL compositor on wasm32; the actual
        // wgpu renderer will be introduced behind a crate module and invoked here.
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (framebuffer, width, height, stride_pixels);
        }

        if let Some(cb) = self.video_refresh {
            unsafe {
                cb(HW_FRAME_BUFFER_VALID as *const c_void, width, height, 0);
            }
        } else if LOG_ONCE_HW.load(Ordering::Relaxed) {
            eprintln!("(wasm96-libretro) present_hardware_frame: video_refresh callback is None");
        }
    }

    fn get_hardware_framebuffer(&mut self) -> usize {
        self.current_framebuffer
    }

    fn notify_geometry_changed(&mut self, width: u32, height: u32) {
        // Notify libretro frontend of geometry change.
        if let Some(env) = self.env {
            use wasm96_libretro_sys::GameGeometry;

            let mut geom = GameGeometry {
                base_width: width,
                base_height: height,
                max_width: width.max(1920),
                max_height: height.max(1080),
                aspect_ratio: 0.0,
            };

            unsafe {
                env(
                    ENVIRONMENT_SET_GEOMETRY,
                    (&raw mut geom) as *mut _ as *mut c_void,
                );
            }
        }
    }
}

impl PlatformAudio for LibretroCallbacks {
    fn audio_batch(&mut self, samples: &[i16]) {
        let frames = samples.len() / 2;

        if let Some(batch_cb) = self.audio_sample_batch {
            unsafe {
                batch_cb(samples.as_ptr(), frames);
            }
        } else if let Some(sample_cb) = self.audio_sample {
            for chunk in samples.chunks(2) {
                unsafe {
                    sample_cb(chunk[0], chunk[1]);
                }
            }
        }
    }
}

impl PlatformInput for LibretroCallbacks {
    fn input_poll(&mut self) {
        if let Some(poll) = self.input_poll {
            unsafe {
                poll();
            }
        }
    }

    fn input_button_state(&mut self, port: u32, button: u32) -> bool {
        let Some(id) = Self::map_button_to_libretro(button) else {
            return false;
        };

        if let Some(input_state) = self.input_state {
            unsafe {
                let val = input_state(port, DEVICE_JOYPAD, 0, id);
                val != 0
            }
        } else {
            false
        }
    }

    fn input_key_state(&mut self, key: u32) -> bool {
        if let Some(input_state) = self.input_state {
            unsafe { input_state(0, DEVICE_KEYBOARD, 0, key) != 0 }
        } else {
            false
        }
    }

    fn input_mouse_x(&mut self) -> i32 {
        if let Some(input_state) = self.input_state {
            unsafe { input_state(0, DEVICE_MOUSE, 0, DEVICE_ID_MOUSE_X) as i32 }
        } else {
            0
        }
    }

    fn input_mouse_y(&mut self) -> i32 {
        if let Some(input_state) = self.input_state {
            unsafe { input_state(0, DEVICE_MOUSE, 0, DEVICE_ID_MOUSE_Y) as i32 }
        } else {
            0
        }
    }

    fn input_mouse_button(&mut self, button: u32) -> bool {
        let Some(id) = Self::map_mouse_button_to_libretro(button) else {
            return false;
        };

        if let Some(input_state) = self.input_state {
            unsafe { input_state(0, DEVICE_MOUSE, 0, id) != 0 }
        } else {
            false
        }
    }
}

impl PlatformCallbacks for LibretroCallbacks {}
