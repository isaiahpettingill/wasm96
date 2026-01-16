//! Libretro callbacks adapter for wasm96-engine.
//!
//! This module implements the PlatformCallbacks trait required by wasm96-engine,
//! translating between the engine's platform-agnostic API and libretro's specific
//! callback functions.

use libretro_sys::{
    AudioSampleBatchFn, AudioSampleFn, DEVICE_ID_JOYPAD_A, DEVICE_ID_JOYPAD_B,
    DEVICE_ID_JOYPAD_DOWN, DEVICE_ID_JOYPAD_L, DEVICE_ID_JOYPAD_L2, DEVICE_ID_JOYPAD_L3,
    DEVICE_ID_JOYPAD_LEFT, DEVICE_ID_JOYPAD_R, DEVICE_ID_JOYPAD_R2, DEVICE_ID_JOYPAD_R3,
    DEVICE_ID_JOYPAD_RIGHT, DEVICE_ID_JOYPAD_SELECT, DEVICE_ID_JOYPAD_START, DEVICE_ID_JOYPAD_UP,
    DEVICE_ID_JOYPAD_X, DEVICE_ID_JOYPAD_Y, DEVICE_JOYPAD, ENVIRONMENT_SET_GEOMETRY, EnvironmentFn,
    HW_FRAME_BUFFER_VALID, InputPollFn, InputStateFn, VideoRefreshFn,
};
use std::ffi::c_void;

use wasm96_engine::PlatformCallbacks;

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
}

impl PlatformCallbacks for LibretroCallbacks {
    fn video_refresh(&mut self, framebuffer: &[u32], width: u32, height: u32, stride_pixels: u32) {
        if let Some(cb) = self.video_refresh {
            // If we have a valid HW framebuffer, present using HW rendering
            if self.current_framebuffer != 0 {
                // Tell libretro to present the HW framebuffer we've been rendering to
                unsafe {
                    cb(HW_FRAME_BUFFER_VALID as *const c_void, width, height, 0);
                }
            } else {
                // Software rendering path - pass the actual framebuffer data
                let data_ptr = framebuffer.as_ptr() as *const c_void;
                let pitch_bytes = (stride_pixels * 4) as usize;
                unsafe {
                    cb(data_ptr, width, height, pitch_bytes);
                }
            }
        }
    }

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

    fn input_key_state(&mut self, _key: u32) -> bool {
        // TODO: Wire to real keyboard input via libretro if/when exposed.
        false
    }

    fn input_mouse_x(&mut self) -> i32 {
        // TODO: Implement via libretro mouse device if needed.
        0
    }

    fn input_mouse_y(&mut self) -> i32 {
        // TODO: Implement via libretro mouse device if needed.
        0
    }

    fn input_mouse_button(&mut self, _button: u32) -> bool {
        // TODO: Implement via libretro mouse device if needed.
        false
    }

    fn get_current_framebuffer(&mut self) -> usize {
        self.current_framebuffer
    }

    fn notify_geometry_changed(&mut self, width: u32, height: u32) {
        // Notify libretro frontend of geometry change.
        if let Some(env) = self.env {
            use libretro_sys::GameGeometry;

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
