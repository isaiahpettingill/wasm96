//! Platform-dependent configuration helpers.
//!
//! This module centralizes small “policy” decisions that vary by target:
//! - Which libretro HW context to request (OpenGL core vs OpenGL ES).
//! - What audio output sample rate to report to libretro and use for mixing.
//! - Pixel format choice for software frames.
//! - Optional alignment rules for software framebuffer pitch (important on some ARM GPUs).
//!
//! The intent is to keep `libretro_glue.rs` and the rendering/audio code free of
//! scattered `#[cfg]` blocks and magic constants.

use libretro_sys::{HwContextType, PixelFormat};

/// libretro HW context selection (value + version pair).
#[derive(Debug, Clone, Copy)]
pub struct HwContextRequest {
    /// One of `RETRO_HW_CONTEXT_*` (e.g. `RETRO_HW_CONTEXT_OPENGL_CORE`).
    pub context_type: u32,
    pub version_major: u32,
    pub version_minor: u32,
}

/// Returns the preferred libretro HW context request for the current target.
///
/// Policy:
/// - Desktop (x86/x86_64): OpenGL core 3.3
/// - ARM64 (e.g. Raspberry Pi 3 aarch64): OpenGL ES 3.0
#[inline]
pub fn preferred_hw_context() -> HwContextRequest {
    // NOTE: We choose by CPU arch to match your stated deployment targets.
    // If you later need finer control (e.g. Linux + aarch64 but NOT Pi),
    // consider gating via Cargo features instead.
    if cfg!(target_arch = "aarch64") {
        HwContextRequest {
            context_type: HwContextType::OpenGLES3 as u32,
            version_major: 3,
            version_minor: 0,
        }
    } else {
        HwContextRequest {
            context_type: HwContextType::OpenGLCore as u32,
            version_major: 3,
            version_minor: 3,
        }
    }
}

/// Returns the output audio sample rate the core should report to libretro.
///
/// Policy:
/// - Desktop (x86/x86_64): 44100 Hz
/// - ARM64 (Pi 3): 48000 Hz
#[inline]
pub fn preferred_audio_sample_rate_hz() -> f64 {
    if cfg!(target_arch = "aarch64") {
        48_000.0
    } else {
        44_100.0
    }
}

/// Returns the pixel format the core should request for *software* frames.
///
/// We standardize on 32-bit XRGB8888. In `libretro-sys 0.1.1`, the equivalent
/// value is `PixelFormat::ARGB8888` (alpha is ignored by libretro for this format).
#[inline]
pub fn preferred_pixel_format() -> PixelFormat {
    PixelFormat::ARGB8888
}

/// Whether software frames should use an aligned pitch on the current target.
///
/// On Raspberry Pi (VideoCore), some paths are more reliable when rows are aligned.
/// Your plan specifies 8-byte alignment of row stride, implemented by padding the width
/// to a multiple of 8 pixels (since 4 bytes per pixel => 32-byte row alignment).
#[inline]
pub fn should_align_software_pitch() -> bool {
    cfg!(target_arch = "aarch64")
}

/// Computes the padded width (in pixels) for a software framebuffer row.
///
/// Rule:
/// - padded_width = (actual_width + 7) & !7
#[inline]
pub fn padded_width_pixels(actual_width: u32) -> u32 {
    (actual_width + 7) & !7
}

/// Returns a label-like string describing the selected platform policy.
///
/// This is intended for logging/debug prints.
#[inline]
pub fn platform_profile_name() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64-gles3-48khz"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64-glcore33-44khz"
    } else if cfg!(target_arch = "x86") {
        "x86-glcore33-44khz"
    } else {
        "generic-glcore33-44khz"
    }
}
