//! Libretro environment helpers.
//!
//! This module centralizes calls to the libretro `EnvironmentFn` callback for:
//! - pixel format negotiation
//! - geometry updates when the internal resolution changes
//!
//! Why this exists:
//! - The core’s visible resolution (`video.width`/`video.height`) can change at runtime.
//! - Some targets/drivers benefit from padded software pitch (row stride) while keeping
//!   visible geometry unchanged.
//! - Libretro frontends need `RETRO_ENVIRONMENT_SET_GEOMETRY` to be called when geometry
//!   changes, otherwise they may keep old assumptions about stride/aspect/viewport.
//!
//! Notes:
//! - This module assumes the core uses XRGB8888 for software frames.
//! - It does not own any global state; it reads the current video state snapshot.

use core::ffi::c_void;

use wasm96_libretro_sys::{
    ENVIRONMENT_SET_GEOMETRY, ENVIRONMENT_SET_PIXEL_FORMAT, EnvironmentFn, GameGeometry,
};

use crate::platform;

/// Request the preferred pixel format for software frames (XRGB8888).
///
/// This is mandatory if the core provides 32-bit pixels to `video_refresh_cb`,
/// otherwise many frontends will interpret the buffer as 15-bit RGB555, leading
/// to color and stride distortion.
pub fn negotiate_pixel_format(env: Option<EnvironmentFn>) -> bool {
    let Some(env) = env else { return false };

    // Libretro expects a pointer to the PixelFormat enum value.
    let mut fmt = platform::preferred_pixel_format();
    unsafe {
        env(
            ENVIRONMENT_SET_PIXEL_FORMAT,
            (&raw mut fmt) as *mut _ as *mut c_void,
        )
    }
}

/// Emit `RETRO_ENVIRONMENT_SET_GEOMETRY` if the core's geometry is marked dirty.
///
/// Intended call sites:
/// - `retro_run()` before presenting video
/// - or any time right after `graphics_set_size()` marks geometry dirty
///
/// What it does:
/// - Reads `video.width/height` (visible) and clears `geometry_dirty` if the call succeeds.
/// - Keeps `aspect_ratio = 0.0` so the frontend can derive aspect from width/height.
///
/// Returns:
/// - `true` if we attempted to set geometry (regardless of success)
/// - `false` if there was no env callback or geometry was not dirty
pub fn maybe_emit_set_geometry(env: Option<EnvironmentFn>) -> bool {
    let Some(env) = env else { return false };

    // Fast path: check dirty flag first.
    {
        let s = match wasm96_engine::state::global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if !s.video.geometry_dirty {
            return false;
        }
    }

    // Build geometry from current state.
    let (width, height) = {
        let s = match wasm96_engine::state::global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        (s.video.width, s.video.height)
    };

    // Libretro uses `GameGeometry`. We only set what we can guarantee.
    // `max_*` should be >= base_*; choose a conservative ceiling.
    let mut geom = GameGeometry {
        base_width: width,
        base_height: height,
        max_width: width.max(1920),
        max_height: height.max(1080),
        aspect_ratio: 0.0,
    };

    let ok = unsafe {
        env(
            ENVIRONMENT_SET_GEOMETRY,
            (&raw mut geom) as *mut _ as *mut c_void,
        )
    };

    // If the frontend accepted it, clear dirty flag.
    if ok {
        let mut s = match wasm96_engine::state::global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.video.geometry_dirty = false;
    }

    true
}
