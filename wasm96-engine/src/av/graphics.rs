// Needed for `alloc::` in this crate.
extern crate alloc;

// -------------------------------------------------------------------------------------------------
// Fonts & text (guest-facing behavior + ABI contract)
//
// This module implements the host-side of wasm96's immediate-mode graphics API. Guests call wasm
// imports such as `wasm96_graphics_text_key` and `wasm96_graphics_text_measure_key` to draw or measure
// text.
//
// Key design: **fonts are keyed by u64**
// - The guest never receives a numeric "font id" handle.
// - Instead, fonts are registered and referenced by an arbitrary `u64` key supplied by the guest.
// - In practice, SDKs hash string keys (e.g. "ui", "title", "debug") to `u64` on the guest side.
//
// Supported font sources:
// - TTF/OTF fonts registered via `wasm96_graphics_font_register_ttf`
// - BDF fonts registered via `wasm96_graphics_font_register_bdf`
// - Built-in Spleen bitmap fonts selected via `wasm96_graphics_font_register_spleen`
//
// IMPORTANT: fallback behavior
// - `wasm96_graphics_text_key` will fall back to built-in Spleen at size 16 if the provided `font_key`
//   has not been registered.
// - `wasm96_graphics_text_measure_key` uses the exact same fallback to keep layout stable between
//   measurement and rendering.
//
// This means text can "just work" without explicit registration, but stable UI metrics should register
// fonts in `setup()`.
//
// Memory model / safety notes for guest pointers:
// - `*_register_*` functions read the font bytes from guest memory immediately and clone/parse them on
//   the host. The guest-provided buffer only needs to remain valid for the duration of the call.
// - `*_text_*` functions read the UTF-8 text bytes from guest memory immediately during the call.
//
// UTF-8 expectations:
// - Text pointers provided by the guest are expected to be valid UTF-8. Invalid data will result in a
//   no-op / zero-size behavior depending on the code path (see call sites).
//
// Performance notes:
// - Register fonts once (typically in `setup()`), not per-frame.
// - Drawing many small text calls is slower than drawing fewer larger strings.
//
// -------------------------------------------------------------------------------------------------

use crate::state::global;
use glam::{Mat4, Vec3};

#[cfg(not(target_arch = "wasm32"))]
use wasmtime::Caller;
#[cfg(target_arch = "wasm32")]
#[allow(unused)]
type Caller<'a, T> = core::marker::PhantomData<(&'a (), T)>;

// External crates for rendering
use fontdue::{Font, FontSettings};

// External crates for asset decoding
use asefile::AsepriteFile;
use resvg::usvg::Tree;
use std::collections::HashMap;
use std::io::Cursor;

// Storage ABI helpers
use alloc::vec::Vec;

use super::resources::{
    AsepriteResource, AvError, FontResource, GifResource, ImageResource, RESOURCES,
};
use super::utils::{graphics_image_from_host, system_millis, tri_edge};

// Material parsing (MTL)
//
// We intentionally avoid external MTL crates here because several are either
// incomplete or require nightly features. For wasm96's current needs, a small
// `map_Kd` extractor is sufficient.

/// Parse a `.mtl` file and return a list of diffuse texture filenames referenced by `map_Kd`.
///
/// Notes:
/// - This is intentionally conservative: it only extracts `map_Kd` (diffuse/albedo).
/// - Returns the value as written (often a relative filename). If options are present
///   (e.g. `-s`, `-o`, `-mm`), they are currently ignored and we try to take the last
///   non-option token as the filename (best-effort).
fn mtl_diffuse_map_filenames(mtl_bytes: &[u8]) -> Vec<String> {
    let Ok(mtl_str) = core::str::from_utf8(mtl_bytes) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    for raw_line in mtl_str.lines() {
        let line = raw_line.trim();

        // Skip blank lines and whole-line comments.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip trailing comments.
        let line = match line.split_once('#') {
            Some((before, _comment)) => before.trim(),
            None => line,
        };
        if line.is_empty() {
            continue;
        }

        // Tokenize by ASCII whitespace.
        let mut parts = line.split_whitespace();

        let Some(keyword) = parts.next() else {
            continue;
        };
        if keyword != "map_Kd" {
            continue;
        }

        let tokens: Vec<&str> = parts.collect();
        if tokens.is_empty() {
            continue;
        }

        // Best-effort: MTL allows options before the filename; we ignore options and
        // take the last token (also handles simplest form: `map_Kd file.png`).
        //
        // This won't perfectly handle filenames that include spaces (rare in MTL).
        if let Some(filename) = tokens.last().copied() {
            out.push(filename.to_string());
        }
    }

    out
}

/// Helper: register an encoded image (PNG/JPEG) under a key, based on filename extension.
///
/// Returns 1 on success, 0 on failure/unknown extension.
///
/// This is a host-side helper intended to support MTL workflows.
fn register_encoded_texture_by_extension(key: u64, filename: &str, encoded: &[u8]) -> u32 {
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".png") {
        // We already have a decoder + keyed resource path for PNG.
        if let Some(decoded) = decode_png_to_rgba(encoded) {
            let mut res = RESOURCES.lock().unwrap();
            res.keyed_images.insert(key, decoded);
            return 1;
        }
        return 0;
    }

    // Support both .jpg and .jpeg.
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        if let Some(decoded) = decode_jpeg_to_rgba(encoded) {
            let mut res = RESOURCES.lock().unwrap();
            res.keyed_images.insert(key, decoded);
            return 1;
        }
        return 0;
    }

    0
}

/// Set the screen dimensions. Resizes the host framebuffer.
pub fn graphics_set_size(width: u32, height: u32) {
    if width == 0 || height == 0 {
        return;
    }
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    s.video.width = width;
    s.video.height = height;

    // Allocate software framebuffer (no padding - platform-agnostic).
    let stride_pixels = width;
    s.video.stride_pixels = stride_pixels;
    s.video.pitch_bytes = (stride_pixels as usize) * 4;

    s.video
        .framebuffer
        .resize((stride_pixels * height) as usize, 0);

    // Mark geometry dirty so libretro side can emit SET_GEOMETRY on next opportunity.
    s.video.geometry_dirty = true;

    // Clear to black on resize
    s.video.framebuffer.fill(0);
}

/// Set the current drawing color.
pub fn graphics_set_color(r: u32, g: u32, b: u32, a: u32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    // Pack as 0xAARRGGBB (ARGB8888).
    // We use the alpha channel for the overlay shader (0 = transparent).
    let color = ((a & 0xFF) << 24) | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    s.video.draw_color = color;
}

/// Get red component of current color (0-255).
pub fn graphics_red() -> u32 {
    let s = global().lock().unwrap();
    (s.video.draw_color >> 16) & 0xFF
}

/// Get green component of current color (0-255).
pub fn graphics_green() -> u32 {
    let s = global().lock().unwrap();
    (s.video.draw_color >> 8) & 0xFF
}

/// Get blue component of current color (0-255).
pub fn graphics_blue() -> u32 {
    let s = global().lock().unwrap();
    s.video.draw_color & 0xFF
}

/// Get alpha component of current color (0-255).
pub fn graphics_alpha() -> u32 {
    let s = global().lock().unwrap();
    (s.video.draw_color >> 24) & 0xFF
}

/// Apply a transformation matrix.
pub fn graphics_apply_matrix(
    m00: f32,
    m01: f32,
    m02: f32,
    m03: f32,
    m10: f32,
    m11: f32,
    m12: f32,
    m13: f32,
    m20: f32,
    m21: f32,
    m22: f32,
    m23: f32,
    m30: f32,
    m31: f32,
    m32: f32,
    m33: f32,
) {
    let mut s = global().lock().unwrap();
    let other = Mat4::from_cols_array(&[
        m00, m10, m20, m30, m01, m11, m21, m31, m02, m12, m22, m32, m03, m13, m23, m33,
    ]);
    s.video.transform = s.video.transform * other;
}

/// Reset the transformation matrix to identity.
pub fn graphics_reset_matrix() {
    let mut s = global().lock().unwrap();
    s.video.transform = Mat4::IDENTITY;
}

/// Rotate the coordinate system around the Z axis (2D rotation).
pub fn graphics_rotate(angle: f32) {
    let mut s = global().lock().unwrap();
    s.video.transform = s.video.transform * Mat4::from_rotation_z(angle);
}

/// Rotate the coordinate system around the X axis.
pub fn graphics_rotate_x(angle: f32) {
    let mut s = global().lock().unwrap();
    s.video.transform = s.video.transform * Mat4::from_rotation_x(angle);
}

/// Rotate the coordinate system around the Y axis.
pub fn graphics_rotate_y(angle: f32) {
    let mut s = global().lock().unwrap();
    s.video.transform = s.video.transform * Mat4::from_rotation_y(angle);
}

/// Rotate the coordinate system around the Z axis.
pub fn graphics_rotate_z(angle: f32) {
    let mut s = global().lock().unwrap();
    s.video.transform = s.video.transform * Mat4::from_rotation_z(angle);
}

/// Scale the coordinate system.
pub fn graphics_scale(sx: f32, sy: f32, sz: f32) {
    let mut s = global().lock().unwrap();
    s.video.transform = s.video.transform * Mat4::from_scale(Vec3::new(sx, sy, sz));
}

/// Shear the coordinate system along the X axis.
pub fn graphics_shear_x(angle: f32) {
    let mut s = global().lock().unwrap();
    let shear = Mat4::from_cols_array(&[
        1.0,
        0.0,
        0.0,
        0.0,
        angle.tan(),
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);
    s.video.transform = s.video.transform * shear;
}

/// Shear the coordinate system along the Y axis.
pub fn graphics_shear_y(angle: f32) {
    let mut s = global().lock().unwrap();
    let shear = Mat4::from_cols_array(&[
        1.0,
        angle.tan(),
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);
    s.video.transform = s.video.transform * shear;
}

/// Translate the coordinate system.
pub fn graphics_translate(x: f32, y: f32, z: f32) {
    let mut s = global().lock().unwrap();
    s.video.transform = s.video.transform * Mat4::from_translation(Vec3::new(x, y, z));
}

/// Push the current transformation matrix onto the stack.
pub fn graphics_push_matrix() {
    let mut s = global().lock().unwrap();
    let current = s.video.transform;
    s.video.transform_stack.push(current);
}

/// Pop the last transformation matrix from the stack.
pub fn graphics_pop_matrix() {
    let mut s = global().lock().unwrap();
    if let Some(prev) = s.video.transform_stack.pop() {
        s.video.transform = prev;
    }
}

/// Get brightness (perceived) of current color (0-255).
/// Uses ITU-R BT.709 luma: 0.2126*R + 0.7152*G + 0.0722*B
pub fn graphics_brightness() -> u32 {
    let s = global().lock().unwrap();
    let r = ((s.video.draw_color >> 16) & 0xFF) as f32;
    let g = ((s.video.draw_color >> 8) & 0xFF) as f32;
    let b = (s.video.draw_color & 0xFF) as f32;
    (0.2126 * r + 0.7152 * g + 0.0722 * b) as u32
}

/// Convert RGB to HSL.
fn rgb_to_hsl(r: u32, g: u32, b: u32) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    let l = (max + min) / 2.0;

    let s = if delta == 0.0 {
        0.0
    } else if l <= 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    (h, s, l)
}

/// Convert HSL to RGB.
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u32, u32, u32) {
    let c = if l <= 0.5 {
        2.0 * l * s
    } else {
        (2.0 - 2.0 * l) * s
    };
    let m = l - c / 2.0;

    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let hh = (h / 60.0).rem_euclid(6.0);
        let x = c * (1.0 - (hh.rem_euclid(2.0) - 1.0).abs());

        let (r1, g1, b1) = match hh as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        (r1 + m, g1 + m, b1 + m)
    };

    (
        (r.min(1.0).max(0.0) * 255.0) as u32,
        (g.min(1.0).max(0.0) * 255.0) as u32,
        (b.min(1.0).max(0.0) * 255.0) as u32,
    )
}

/// Get hue of current color (0-360 degrees).
pub fn graphics_hue() -> f32 {
    let s = global().lock().unwrap();
    let r = (s.video.draw_color >> 16) & 0xFF;
    let g = (s.video.draw_color >> 8) & 0xFF;
    let b = s.video.draw_color & 0xFF;
    rgb_to_hsl(r, g, b).0
}

/// Get saturation of current color (0-100%).
pub fn graphics_saturation() -> f32 {
    let s = global().lock().unwrap();
    let r = (s.video.draw_color >> 16) & 0xFF;
    let g = (s.video.draw_color >> 8) & 0xFF;
    let b = s.video.draw_color & 0xFF;
    rgb_to_hsl(r, g, b).1 * 100.0
}

/// Get lightness of current color (0-100%).
pub fn graphics_lightness() -> f32 {
    let s = global().lock().unwrap();
    let r = (s.video.draw_color >> 16) & 0xFF;
    let g = (s.video.draw_color >> 8) & 0xFF;
    let b = s.video.draw_color & 0xFF;
    rgb_to_hsl(r, g, b).2 * 100.0
}

/// Create and set a color from RGB values.
pub fn graphics_color_rgb(r: u32, g: u32, b: u32, a: u32) {
    graphics_set_color(r, g, b, a);
}

/// Create and set a color from HSL values.
pub fn graphics_color_hsl(h: f32, s: f32, l: f32, a: u32) {
    let (r, g, b) = hsl_to_rgb(h / 360.0, s / 100.0, l / 100.0);
    graphics_set_color(r, g, b, a);
}

/// Linear interpolation between two colors.
/// t is in range 0.0-1.0
pub fn graphics_lerp_color(
    r1: u32,
    g1: u32,
    b1: u32,
    a1: u32,
    r2: u32,
    g2: u32,
    b2: u32,
    a2: u32,
    t: f32,
) -> u32 {
    let t = t.min(1.0).max(0.0);
    let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u32;
    let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u32;
    let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u32;
    let a = (a1 as f32 + (a2 as f32 - a1 as f32) * t) as u32;
    ((a & 0xFF) << 24) | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF)
}

/// Linear interpolation between two palette colors (indices 0-255).
pub fn graphics_palette_lerp(c1: u32, c2: u32, t: f32) -> u32 {
    let t = t.min(1.0).max(0.0);
    let r1 = (c1 >> 16) & 0xFF;
    let g1 = (c1 >> 8) & 0xFF;
    let b1 = c1 & 0xFF;
    let a1 = (c1 >> 24) & 0xFF;

    let r2 = (c2 >> 16) & 0xFF;
    let g2 = (c2 >> 8) & 0xFF;
    let b2 = c2 & 0xFF;
    let a2 = (c2 >> 24) & 0xFF;

    let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u32;
    let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u32;
    let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u32;
    let a = (a1 as f32 + (a2 as f32 - a1 as f32) * t) as u32;
    ((a & 0xFF) << 24) | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF)
}

/// Set fill color.
pub fn graphics_fill(r: u32, g: u32, b: u32, a: u32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let color = ((a & 0xFF) << 24) | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    s.video.fill_color = color;
    s.video.fill_enabled = true;
}

/// Disable fill color.
pub fn graphics_no_fill() {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.fill_enabled = false;
}

/// Set stroke color.
pub fn graphics_stroke(r: u32, g: u32, b: u32, a: u32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let color = ((a & 0xFF) << 24) | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    s.video.stroke_color = color;
    s.video.stroke_enabled = true;
}

/// Disable stroke color.
pub fn graphics_no_stroke() {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.stroke_enabled = false;
}

/// Enable erase mode (draw with destination alpha blending).
pub fn graphics_erase() {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.erase_mode_enabled = true;
}

/// Disable erase mode.
pub fn graphics_no_erase() {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.erase_mode_enabled = false;
}

/// Set color mode (0 = RGB, 1 = HSL).
pub fn graphics_color_mode(mode: u32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.color_mode = match mode {
        0 => crate::state::ColorMode::RGB,
        1 => crate::state::ColorMode::HSL,
        _ => crate::state::ColorMode::RGB,
    };
}

/// Set clipping region.
pub fn graphics_clip(x: i32, y: i32, w: u32, h: u32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.clip_rect = Some((x, y, w, h));
}

/// Begin clipping region (alias for clip).
pub fn graphics_begin_clip(x: i32, y: i32, w: u32, h: u32) {
    graphics_clip(x, y, w, h);
}

/// End clipping region (clear clip).
pub fn graphics_end_clip() {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.clip_rect = None;
}

/// Clear the screen to a specific color.
pub fn graphics_background(r: u32, g: u32, b: u32) {
    // On non-wasm32 targets we may have a 3D backend that renders into a GL or WGPU framebuffer.
    // On wasm32 the 3D backend may be disabled/implemented differently, so avoid referencing it.
    #[cfg(not(target_arch = "wasm32"))]
    let hw_cleared = {
        let is_3d = super::graphics3d::STATE_3D.lock().unwrap().enabled;
        if is_3d {
            let r_f = r as f32 / 255.0;
            let g_f = g as f32 / 255.0;
            let b_f = b as f32 / 255.0;

            let gl_cleared = super::graphics3d::clear_framebuffer(r_f, g_f, b_f, 1.0);
            let wgpu_cleared = super::wgpu_backend::wgpu_clear_framebuffer(r_f, g_f, b_f, 1.0);
            gl_cleared || wgpu_cleared
        } else {
            false
        }
    };
    #[cfg(target_arch = "wasm32")]
    let hw_cleared = false;

    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if hw_cleared {
        // Clear software framebuffer to transparent so it doesn't occlude the 3D scene
        s.video.framebuffer.fill(0x00000000);
        return;
    } else {
        // Fallback: software-only rendering - clear to requested color
        let color = (0xFF << 24) | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
        s.video.framebuffer.fill(color);
    }
}

/// Clear the framebuffer to transparent.
pub fn graphics_clear() {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.video.framebuffer.fill(0x00000000);
}

/// Draw a single pixel.
pub fn graphics_point(x: i32, y: i32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let p = s
        .video
        .transform
        .transform_point3(Vec3::new(x as f32, y as f32, 0.0));
    let x = p.x as i32;
    let y = p.y as i32;

    let w = s.video.width as i32;
    let h = s.video.height as i32;

    if x >= 0 && x < w && y >= 0 && y < h {
        let stride = s.video.stride_pixels as i32;
        let idx = (y * stride + x) as usize;
        s.video.framebuffer[idx] = s.video.draw_color;
    }
}

/// Draw a line using Bresenham's algorithm.
pub fn graphics_line(mut x0: i32, mut y0: i32, mut x1: i32, mut y1: i32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let p0 = s
        .video
        .transform
        .transform_point3(Vec3::new(x0 as f32, y0 as f32, 0.0));
    let p1 = s
        .video
        .transform
        .transform_point3(Vec3::new(x1 as f32, y1 as f32, 0.0));
    x0 = p0.x as i32;
    y0 = p0.y as i32;
    x1 = p1.x as i32;
    y1 = p1.y as i32;

    let w = s.video.width as i32;
    let h = s.video.height as i32;
    let stride = s.video.stride_pixels as i32;

    let color = if s.video.stroke_enabled {
        s.video.stroke_color
    } else {
        s.video.draw_color
    };

    let fb = &mut s.video.framebuffer;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
            fb[(y0 * stride + x0) as usize] = color;
        }

        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Draw a filled rectangle.
pub fn graphics_rect(x: i32, y: i32, w: u32, h: u32) {
    {
        let s = global().lock().unwrap();
        if s.video.transform != Mat4::IDENTITY {
            drop(s);
            let x1 = x;
            let y1 = y;
            let x2 = x + w as i32;
            let y2 = y;
            let x3 = x + w as i32;
            let y3 = y + h as i32;
            let x4 = x;
            let y4 = y + h as i32;
            graphics_quad(x1, y1, x2, y2, x3, y3, x4, y4);
            return;
        }
    }

    let mut s = global().lock().unwrap();
    let screen_w = s.video.width as i32;
    let screen_h = s.video.height as i32;
    let color = if s.video.fill_enabled {
        s.video.fill_color
    } else {
        s.video.draw_color
    };

    let x_start = x.max(0);
    let y_start = y.max(0);
    let x_end = (x + w as i32).min(screen_w);
    let y_end = (y + h as i32).min(screen_h);

    if x_start >= x_end || y_start >= y_end {
        return;
    }

    let fb_w = s.video.stride_pixels as usize;
    let fb = &mut s.video.framebuffer;

    for curr_y in y_start..y_end {
        let start_idx = (curr_y as usize) * fb_w + (x_start as usize);
        let end_idx = (curr_y as usize) * fb_w + (x_end as usize);
        fb[start_idx..end_idx].fill(color);
    }
}

/// Draw a rectangle outline.
pub fn graphics_rect_outline(x: i32, y: i32, w: u32, h: u32) {
    let x1 = x;
    let y1 = y;
    let x2 = x + w as i32;
    let y2 = y;
    let x3 = x + w as i32;
    let y3 = y + h as i32;
    let x4 = x;
    let y4 = y + h as i32;

    graphics_line(x1, y1, x2, y2);
    graphics_line(x2, y2, x3, y3);
    graphics_line(x3, y3, x4, y4);
    graphics_line(x4, y4, x1, y1);
}

/// Draw a filled circle.
pub fn graphics_circle(cx: i32, cy: i32, r: u32) {
    {
        let s = global().lock().unwrap();
        if s.video.transform != Mat4::IDENTITY {
            drop(s);
            // Fallback to ellipse for transformed circle
            graphics_ellipse(cx, cy, r * 2, r * 2);
            return;
        }
    }
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let screen_w = s.video.width as i32;
    let screen_h = s.video.height as i32;
    let stride = s.video.stride_pixels as i32;
    let color = if s.video.fill_enabled {
        s.video.fill_color
    } else {
        s.video.draw_color
    };
    let fb = &mut s.video.framebuffer;

    let r_sq = (r * r) as i32;
    let r_i32 = r as i32;

    let x_min = (cx - r_i32).max(0);
    let x_max = (cx + r_i32).min(screen_w);
    let y_min = (cy - r_i32).max(0);
    let y_max = (cy + r_i32).min(screen_h);

    for curr_y in y_min..y_max {
        for x in x_min..x_max {
            let dx = x - cx;
            let dy = curr_y - cy;
            if dx * dx + dy * dy <= r_sq {
                fb[(curr_y * stride + x) as usize] = color;
            }
        }
    }
}

/// Draw a circle outline (Bresenham's circle algorithm).
pub fn graphics_circle_outline(cx: i32, cy: i32, r: u32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let w = s.video.width as i32;
    let h = s.video.height as i32;
    let stride = s.video.stride_pixels as i32;
    let old_color = s.video.draw_color;

    if s.video.stroke_enabled {
        s.video.draw_color = s.video.stroke_color;
    }

    let color = s.video.draw_color;
    let fb = &mut s.video.framebuffer;

    let mut x = 0;
    let mut y = r as i32;
    let mut d = 3 - 2 * r as i32;

    let mut plot = |x: i32, y: i32| {
        if x >= 0 && x < w && y >= 0 && y < h {
            fb[(y * stride + x) as usize] = color;
        }
    };

    while y >= x {
        plot(cx + x, cy + y);
        plot(cx - x, cy + y);
        plot(cx + x, cy - y);
        plot(cx - x, cy - y);
        plot(cx + y, cy + x);
        plot(cx - y, cy + x);
        plot(cx + y, cy - x);
        plot(cx - y, cy - x);

        x += 1;
        if d > 0 {
            y -= 1;
            d = d + 4 * (x - y) + 10;
        } else {
            d = d + 4 * x + 6;
        }
    }

    s.video.draw_color = old_color;
}

/// Draw a filled ellipse centered at (cx, cy) with width w and height h.
pub fn graphics_ellipse(cx: i32, cy: i32, w: u32, h: u32) {
    if w == 0 || h == 0 {
        return;
    }
    {
        let s = global().lock().unwrap();
        if s.video.transform != Mat4::IDENTITY {
            drop(s);
            // Tessellate ellipse into triangles
            let segments = 32;
            let rx = w as f32 / 2.0;
            let ry = h as f32 / 2.0;
            for i in 0..segments {
                let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
                let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
                let x1 = cx as f32 + a1.cos() * rx;
                let y1 = cy as f32 + a1.sin() * ry;
                let x2 = cx as f32 + a2.cos() * rx;
                let y2 = cy as f32 + a2.sin() * ry;
                graphics_triangle(cx, cy, x1 as i32, y1 as i32, x2 as i32, y2 as i32);
            }
            return;
        }
    }
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let screen_w = s.video.width as i32;
    let screen_h = s.video.height as i32;
    let stride = s.video.stride_pixels as i32;

    let color = if s.video.fill_enabled {
        s.video.fill_color
    } else {
        s.video.draw_color
    };

    let fb = &mut s.video.framebuffer;

    let rx = (w as i32) / 2;
    let ry = (h as i32) / 2;
    if rx == 0 || ry == 0 {
        return;
    }

    let x_min = (cx - rx).max(0);
    let x_max = (cx + rx).min(screen_w);
    let y_min = (cy - ry).max(0);
    let y_max = (cy + ry).min(screen_h);

    let rx2 = (rx * rx) as i64;
    let ry2 = (ry * ry) as i64;

    for y in y_min..y_max {
        let dy = (y - cy) as i64;
        let dy2 = dy * dy;
        for x in x_min..x_max {
            let dx = (x - cx) as i64;
            let dx2 = dx * dx;
            if dx2 * ry2 + dy2 * rx2 <= rx2 * ry2 {
                fb[(y * stride + x) as usize] = color;
            }
        }
    }
}

/// Draw an arc centered at (cx, cy) with width w and height h, from start to end (radians).
pub fn graphics_arc(cx: i32, cy: i32, w: u32, h: u32, start: f32, end: f32) {
    if w == 0 || h == 0 {
        return;
    }
    let rx = (w as f32) / 2.0;
    let ry = (h as f32) / 2.0;
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }

    let mut a0 = start;
    let mut a1 = end;
    if a1 < a0 {
        core::mem::swap(&mut a0, &mut a1);
    }

    let segments = 64u32;
    let step = (a1 - a0) / segments as f32;
    let mut prev_x = cx as f32 + rx * a0.cos();
    let mut prev_y = cy as f32 + ry * a0.sin();

    for i in 1..=segments {
        let t = a0 + step * i as f32;
        let x = cx as f32 + rx * t.cos();
        let y = cy as f32 + ry * t.sin();
        graphics_line(
            prev_x.round() as i32,
            prev_y.round() as i32,
            x.round() as i32,
            y.round() as i32,
        );
        prev_x = x;
        prev_y = y;
    }
}

/// Draw a filled quad using two triangles.
pub fn graphics_quad(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32, x4: i32, y4: i32) {
    graphics_triangle(x1, y1, x2, y2, x3, y3);
    graphics_triangle(x1, y1, x3, y3, x4, y4);
}

/// Draw an image from guest memory.
/// `ptr` points to RGBA bytes (4 bytes per pixel).
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_image(
    caller: &mut Caller<'_, crate::state::Wasm96Ctx>,
    x: i32,
    y: i32,
    img_w: u32,
    img_h: u32,
    ptr: u32,
    len: u32,
) -> Result<(), AvError> {
    // Basic validation
    let expected_len = img_w.checked_mul(img_h).and_then(|s| s.checked_mul(4));
    if let Some(req) = expected_len {
        if len < req {
            // Not enough data provided
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Read guest memory
    let memory = caller
        .get_export("memory")
        .and_then(wasmtime::Extern::into_memory)
        .ok_or(AvError::MissingMemory)?;

    // We read the whole image into a temp buffer.
    // Optimization: could read row-by-row to avoid large allocation,
    // but for retro resolutions this is fine.
    let mut img_data = vec![0u8; len as usize];
    memory
        .read(&*caller, ptr as usize, &mut img_data)
        .map_err(|_| AvError::MemoryReadFailed)?;

    // Lock and draw (using helper to handle transformations)
    graphics_image_from_host(x, y, img_w, img_h, &img_data);

    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_image(
    _env: &mut (),
    x: i32,
    y: i32,
    img_w: u32,
    img_h: u32,
    ptr: u32,
    len: u32,
) -> Result<(), AvError> {
    // Basic validation
    let expected_len = img_w.checked_mul(img_h).and_then(|s| s.checked_mul(4));
    if let Some(req) = expected_len {
        if len < req {
            return Ok(());
        }
    } else {
        return Ok(());
    }

    let img_data = super::utils::read_guest_bytes(ptr, len)?;
    graphics_image_from_host(x, y, img_w, img_h, &img_data);
    Ok(())
}

/// Decode PNG bytes from guest memory and draw at (x, y) at the image's natural size.
///
/// If decoding fails, this is a no-op.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_image_png(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    x: i32,
    y: i32,
    ptr: u32,
    len: u32,
) -> Result<(), AvError> {
    let png_bytes = super::utils::read_guest_bytes(env, ptr, len)?;

    let decoded = match decode_png_to_rgba(&png_bytes) {
        Some(d) => d,
        None => return Ok(()),
    };

    graphics_image_from_host(x, y, decoded.width, decoded.height, &decoded.rgba);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_image_png(
    _env: &mut (),
    x: i32,
    y: i32,
    ptr: u32,
    len: u32,
) -> Result<(), AvError> {
    let png_bytes = super::utils::read_guest_bytes(ptr, len)?;

    let decoded = match decode_png_to_rgba(&png_bytes) {
        Some(d) => d,
        None => return Ok(()),
    };

    graphics_image_from_host(x, y, decoded.width, decoded.height, &decoded.rgba);
    Ok(())
}

/// Decode JPEG bytes from guest memory and draw at (x, y) at the image's natural size.
///
/// If decoding fails, this is a no-op.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_image_jpeg(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    x: i32,
    y: i32,
    ptr: u32,
    len: u32,
) -> Result<(), AvError> {
    let jpeg_bytes = super::utils::read_guest_bytes(env, ptr, len)?;

    let decoded = match decode_jpeg_to_rgba(&jpeg_bytes) {
        Some(d) => d,
        None => return Ok(()),
    };

    graphics_image_from_host(x, y, decoded.width, decoded.height, &decoded.rgba);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_image_jpeg(
    _env: &mut (),
    x: i32,
    y: i32,
    ptr: u32,
    len: u32,
) -> Result<(), AvError> {
    let jpeg_bytes = super::utils::read_guest_bytes(ptr, len)?;

    let decoded = match decode_jpeg_to_rgba(&jpeg_bytes) {
        Some(d) => d,
        None => return Ok(()),
    };

    graphics_image_from_host(x, y, decoded.width, decoded.height, &decoded.rgba);
    Ok(())
}

/// Load a `.mtl` file from guest memory, parse it, and register any referenced diffuse textures.
///
/// ABI:
/// - `mtl_ptr/mtl_len` points to the `.mtl` file bytes.
/// - `tex_ptr/tex_len` points to one encoded texture blob (PNG/JPEG).
/// - `tex_filename_ptr/tex_filename_len` is the texture filename (used for extension detection
///   and matching against `map_Kd` entries).
/// - `texture_key` is the keyed image id under which the decoded texture will be registered.
///
/// Returns 1 on success, 0 on failure.
///
/// Notes:
/// - Current implementation expects the guest to call this once per texture blob, passing the
///   indicated `tex_filename` from the MTL. If the filename does not appear in the MTL's `map_Kd`
///   list, this is a no-op and returns 0.
/// - This keeps the host stateless regarding filesystem paths while still enabling OBJ+MTL style
///   materials in a "ROM-bytes only" environment.
/// Register raw MTL bytes under a string key.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_mtl_register(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let mtl_bytes = match super::utils::read_guest_bytes(env, data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_mtls.insert(key, mtl_bytes);
    1
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_mtl_register(key: u64, data_ptr: u32, data_len: u32) -> u32 {
    let mtl_bytes = match super::utils::read_guest_bytes(data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_mtls.insert(key, mtl_bytes);
    1
}

#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_mtl_register_texture(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    texture_key: u64,
    mtl_ptr: u32,
    mtl_len: u32,
    tex_filename_ptr: u32,
    tex_filename_len: u32,
    tex_ptr: u32,
    tex_len: u32,
) -> u32 {
    let mtl_bytes = match super::utils::read_guest_bytes(env, mtl_ptr, mtl_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let tex_filename_bytes =
        match super::utils::read_guest_bytes(env, tex_filename_ptr, tex_filename_len) {
            Ok(b) => b,
            Err(_) => return 0,
        };

    let tex_filename = match core::str::from_utf8(&tex_filename_bytes) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let tex_bytes = match super::utils::read_guest_bytes(env, tex_ptr, tex_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    // Only register if the MTL actually references this filename as a diffuse map.
    let diffuse_files = mtl_diffuse_map_filenames(&mtl_bytes);
    if !diffuse_files.iter().any(|f| f == tex_filename) {
        return 0;
    }

    register_encoded_texture_by_extension(texture_key, tex_filename, &tex_bytes)
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_mtl_register_texture(
    texture_key: u64,
    mtl_ptr: u32,
    mtl_len: u32,
    tex_filename_ptr: u32,
    tex_filename_len: u32,
    tex_ptr: u32,
    tex_len: u32,
) -> u32 {
    let mtl_bytes = match super::utils::read_guest_bytes(mtl_ptr, mtl_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let tex_filename_bytes =
        match super::utils::read_guest_bytes(tex_filename_ptr, tex_filename_len) {
            Ok(b) => b,
            Err(_) => return 0,
        };

    let tex_filename = match core::str::from_utf8(&tex_filename_bytes) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let tex_bytes = match super::utils::read_guest_bytes(tex_ptr, tex_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let diffuse_files = mtl_diffuse_map_filenames(&mtl_bytes);
    if !diffuse_files.iter().any(|f| f == tex_filename) {
        return 0;
    }

    register_encoded_texture_by_extension(texture_key, tex_filename, &tex_bytes)
}

fn decode_png_to_rgba(png_bytes: &[u8]) -> Option<ImageResource> {
    let cursor = std::io::Cursor::new(png_bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().ok()?;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;

    let w = info.width;
    let h = info.height;
    if w == 0 || h == 0 {
        return None;
    }

    let bytes = &buf[..info.buffer_size()];

    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => bytes
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => bytes.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => bytes
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => {
            // If the decoder didn't expand indexed color, we don't support it here.
            return None;
        }
    };

    Some(ImageResource {
        rgba,
        width: w,
        height: h,
    })
}

fn decode_jpeg_to_rgba(jpeg_bytes: &[u8]) -> Option<ImageResource> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(jpeg_bytes));
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;

    let w = info.width as u32;
    let h = info.height as u32;
    if w == 0 || h == 0 {
        return None;
    }

    let rgba: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        jpeg_decoder::PixelFormat::L16 => {
            // 16-bit grayscale: take the high byte as an 8-bit intensity.
            pixels
                .chunks_exact(2)
                .flat_map(|p| {
                    let g = p[0]; // high byte
                    [g, g, g, 255]
                })
                .collect()
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            // Minimal support: convert CMYK -> RGB using a simple approximation.
            // RGB = 255 - min(255, C + K) (same for M/Y)
            pixels
                .chunks_exact(4)
                .flat_map(|p| {
                    let c = p[0] as u16;
                    let m = p[1] as u16;
                    let y = p[2] as u16;
                    let k = p[3] as u16;

                    let r = 255u8.saturating_sub((c + k).min(255) as u8);
                    let g = 255u8.saturating_sub((m + k).min(255) as u8);
                    let b = 255u8.saturating_sub((y + k).min(255) as u8);

                    [r, g, b, 255]
                })
                .collect()
        }
    };

    Some(ImageResource {
        rgba,
        width: w,
        height: h,
    })
}

/// Register a PNG under a string key (bytes are encoded PNG).
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_png_register(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let png_bytes = match super::utils::read_guest_bytes(env, data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let decoded = match decode_png_to_rgba(&png_bytes) {
        Some(d) => d,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_images.insert(key, decoded);
    1
}

/// Register a PNG under a string key (bytes are encoded PNG) (wasm32/web).
#[cfg(target_arch = "wasm32")]
pub fn graphics_png_register(key: u64, data_ptr: u32, data_len: u32) -> u32 {
    let png_bytes = match super::utils::read_guest_bytes(data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let decoded = match decode_png_to_rgba(&png_bytes) {
        Some(d) => d,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_images.insert(key, decoded);
    1
}

#[cfg(test)]
mod mtl_tests {
    use super::*;

    #[test]
    fn mtl_parses_map_kd_filename() {
        // Keep fixture inline to avoid depending on filesystem access in tests.
        let mtl = br#"
newmtl TestMaterial
Kd 1.0 1.0 1.0
map_Kd test_texture.png
"#;

        let files = mtl_diffuse_map_filenames(mtl);
        assert_eq!(files, vec!["test_texture.png".to_string()]);
    }

    // NOTE:
    // We intentionally do not unit-test `register_encoded_texture_by_extension(...)` directly here
    // because it takes a `wasmtime::Caller`, which is not constructible in a plain unit test
    // without embedding a full Wasmtime runtime + instance.
    //
    // The extension filtering behavior is still covered indirectly by higher-level integration
    // of the graphics/image registration paths during normal runtime execution.
}

/// Draw a keyed PNG at natural size.
pub fn graphics_png_draw_key(key: u64, x: i32, y: i32) {
    graphics_image_draw_key(key, x, y);
}

/// Draw a keyed PNG scaled (nearest-neighbor).
pub fn graphics_png_draw_key_scaled(key: u64, x: i32, y: i32, w: u32, h: u32) {
    graphics_image_draw_key_scaled(key, x, y, w, h);
}

/// Unregister a keyed PNG.
pub fn graphics_png_unregister(key: u64) {
    let mut res = RESOURCES.lock().unwrap();
    res.keyed_images.remove(&key);
}

/// Register a JPEG under a string key (bytes are encoded JPEG).
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_jpeg_register(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let jpeg_bytes = match super::utils::read_guest_bytes(env, data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let decoded = match decode_jpeg_to_rgba(&jpeg_bytes) {
        Some(d) => d,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_images.insert(key, decoded);
    1
}

/// Register a JPEG under a string key (bytes are encoded JPEG) (wasm32/web).
#[cfg(target_arch = "wasm32")]
pub fn graphics_jpeg_register(key: u64, data_ptr: u32, data_len: u32) -> u32 {
    let jpeg_bytes = match super::utils::read_guest_bytes(data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    let decoded = match decode_jpeg_to_rgba(&jpeg_bytes) {
        Some(d) => d,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_images.insert(key, decoded);
    1
}

/// Draw a keyed JPEG at natural size.
pub fn graphics_jpeg_draw_key(key: u64, x: i32, y: i32) {
    graphics_image_draw_key(key, x, y);
}

/// Draw a keyed JPEG scaled (nearest-neighbor).
pub fn graphics_jpeg_draw_key_scaled(key: u64, x: i32, y: i32, w: u32, h: u32) {
    graphics_image_draw_key_scaled(key, x, y, w, h);
}

/// Unregister a keyed JPEG.
pub fn graphics_jpeg_unregister(key: u64) {
    let mut res = RESOURCES.lock().unwrap();
    res.keyed_images.remove(&key);
}

/// Draw any keyed decoded image at natural size.
fn graphics_image_draw_key(key: u64, x: i32, y: i32) {
    let img = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_images.get(&key).cloned()
    };

    if let Some(img) = img {
        graphics_image_from_host(x, y, img.width, img.height, &img.rgba);
    }
}

/// Draw any keyed decoded image scaled (nearest-neighbor).
fn graphics_image_draw_key_scaled(key: u64, x: i32, y: i32, w: u32, h: u32) {
    let img = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_images.get(&key).cloned()
    };

    let Some(img) = img else {
        return;
    };

    // Natural size if either dimension is 0.
    if w == 0 || h == 0 {
        graphics_image_from_host(x, y, img.width, img.height, &img.rgba);
        return;
    }

    let src_w = img.width;
    let src_h = img.height;
    if src_w == 0 || src_h == 0 {
        return;
    }

    let mut dst = vec![0u8; (w as usize).saturating_mul(h as usize).saturating_mul(4)];
    for dy in 0..h {
        let sy = (dy as u64 * src_h as u64 / h as u64) as u32;
        let sy = sy.min(src_h.saturating_sub(1));
        for dx in 0..w {
            let sx = (dx as u64 * src_w as u64 / w as u64) as u32;
            let sx = sx.min(src_w.saturating_sub(1));

            let sidx = ((sy as usize) * (src_w as usize) + (sx as usize)) * 4;
            let didx = ((dy as usize) * (w as usize) + (dx as usize)) * 4;

            if sidx + 3 < img.rgba.len() && didx + 3 < dst.len() {
                dst[didx] = img.rgba[sidx];
                dst[didx + 1] = img.rgba[sidx + 1];
                dst[didx + 2] = img.rgba[sidx + 2];
                dst[didx + 3] = img.rgba[sidx + 3];
            }
        }
    }

    graphics_image_from_host(x, y, w, h, &dst);
}

/// Draw a filled triangle using a barycentric (edge-function) rasterizer.
///
/// Properties:
/// - Works for any vertex order (winding), filled area is consistent.
/// - Clips to framebuffer bounds.
/// - Uses integer edge functions for stability/determinism.
///
/// Rasterization rule:
/// - We treat pixels as **samples at pixel centers**: (x + 0.5, y + 0.5).
///   This avoids cases where a triangle covers no integer lattice points and would
///   otherwise render as empty for small/skinny triangles.
pub fn graphics_triangle(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) {
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let p1 = s
        .video
        .transform
        .transform_point3(Vec3::new(x1 as f32, y1 as f32, 0.0));
    let p2 = s
        .video
        .transform
        .transform_point3(Vec3::new(x2 as f32, y2 as f32, 0.0));
    let p3 = s
        .video
        .transform
        .transform_point3(Vec3::new(x3 as f32, y3 as f32, 0.0));
    let x1 = p1.x as i32;
    let y1 = p1.y as i32;
    let x2 = p2.x as i32;
    let y2 = p2.y as i32;
    let x3 = p3.x as i32;
    let y3 = p3.y as i32;
    let w = s.video.width as i32;
    let h = s.video.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }

    let color = s.video.draw_color;
    let fb = &mut s.video.framebuffer;

    // Use 2x fixed-point coordinates so we can represent pixel centers as integers.
    // A pixel center at (x + 0.5, y + 0.5) becomes P2 = (2x + 1, 2y + 1).
    let v0 = (x1 * 2, y1 * 2);
    let v1 = (x2 * 2, y2 * 2);
    let v2 = (x3 * 2, y3 * 2);

    // Degenerate (area==0): nothing to fill.
    let area = tri_edge(v0, v1, v2);
    if area == 0 {
        return;
    }

    // Bounding box in pixel coordinates (inclusive), computed from the triangle vertices.
    // We convert from 2x space back into pixel indices.
    let min_x = ((v0.0.min(v1.0).min(v2.0)) >> 1).max(0);
    let max_x = ((v0.0.max(v1.0).max(v2.0)) >> 1).min(w - 1);
    let min_y = ((v0.1.min(v1.1).min(v2.1)) >> 1).max(0);
    let max_y = ((v0.1.max(v1.1).max(v2.1)) >> 1).min(h - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    // Make the edge tests winding-invariant by normalizing the edge function
    // values to the same sign (i.e. as if the triangle had positive area).
    //
    // IMPORTANT: The sign normalization must match the sign of the triangle's own
    // area under the *same* (a,b,c) ordering used by `tri_edge(a,b,c)`.
    let sign = if area > 0 { 1 } else { -1 };

    for y in min_y..=max_y {
        let row = (y as usize) * (w as usize);
        let p_y = y * 2 + 1;
        for x in min_x..=max_x {
            // Sample at pixel center in 2x space.
            let p = (x * 2 + 1, p_y);

            // Edge functions for triangle v0,v1,v2.
            // Multiply by `sign` so "inside" corresponds to >= 0 regardless of winding.
            //
            // NOTE:
            // `tri_edge(a, b, c)` computes a left-of test for the directed edge a->b at point c.
            // For point-in-triangle, the consistent set is:
            //   w0 = edge(v0->v1, p)
            //   w1 = edge(v1->v2, p)
            //   w2 = edge(v2->v0, p)
            let w0 = tri_edge(v0, v1, p) * sign;
            let w1 = tri_edge(v1, v2, p) * sign;
            let w2 = tri_edge(v2, v0, p) * sign;

            if w0 >= 0 && w1 >= 0 && w2 >= 0 {
                fb[row + x as usize] = color;
            }
        }
    }
}

/// Draw a triangle outline.
pub fn graphics_triangle_outline(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) {
    graphics_line(x1, y1, x2, y2);
    graphics_line(x2, y2, x3, y3);
    graphics_line(x3, y3, x1, y1);
}

/// Draw a quadratic Bezier curve.
pub fn graphics_bezier_quadratic(
    x1: i32,
    y1: i32,
    cx: i32,
    cy: i32,
    x2: i32,
    y2: i32,
    segments: u32,
) {
    if segments == 0 {
        return;
    }
    let mut prev_x = x1 as f32;
    let mut prev_y = y1 as f32;
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let x =
            (1.0 - t).powi(2) * x1 as f32 + 2.0 * (1.0 - t) * t * cx as f32 + t.powi(2) * x2 as f32;
        let y =
            (1.0 - t).powi(2) * y1 as f32 + 2.0 * (1.0 - t) * t * cy as f32 + t.powi(2) * y2 as f32;
        graphics_line(prev_x as i32, prev_y as i32, x as i32, y as i32);
        prev_x = x;
        prev_y = y;
    }
}

/// Draw a cubic Bezier curve.
pub fn graphics_bezier_cubic(
    x1: i32,
    y1: i32,
    cx1: i32,
    cy1: i32,
    cx2: i32,
    cy2: i32,
    x2: i32,
    y2: i32,
    segments: u32,
) {
    if segments == 0 {
        return;
    }
    let mut prev_x = x1 as f32;
    let mut prev_y = y1 as f32;
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let x = (1.0 - t).powi(3) * x1 as f32
            + 3.0 * (1.0 - t).powi(2) * t * cx1 as f32
            + 3.0 * (1.0 - t) * t.powi(2) * cx2 as f32
            + t.powi(3) * x2 as f32;
        let y = (1.0 - t).powi(3) * y1 as f32
            + 3.0 * (1.0 - t).powi(2) * t * cy1 as f32
            + 3.0 * (1.0 - t) * t.powi(2) * cy2 as f32
            + t.powi(3) * y2 as f32;
        graphics_line(prev_x as i32, prev_y as i32, x as i32, y as i32);
        prev_x = x;
        prev_y = y;
    }
}

/// Draw a filled pill.
pub fn graphics_pill(x: i32, y: i32, w: u32, h: u32) {
    if w == 0 || h == 0 {
        return;
    }
    let r = (w.min(h) / 2) as i32;
    // Draw center rect
    graphics_rect(x + r, y, w - 2 * r as u32, h);
    // Draw left cap
    graphics_circle(x + r, y + r, r as u32);
    // Draw right cap
    graphics_circle(x + w as i32 - r, y + r, r as u32);
}

/// Draw a pill outline.
pub fn graphics_pill_outline(x: i32, y: i32, w: u32, h: u32) {
    if w == 0 || h == 0 {
        return;
    }
    let r = (w.min(h) / 2) as i32;
    // Outline center rect
    graphics_rect_outline(x + r, y, w - 2 * r as u32, h);
    // Outline left cap
    graphics_circle_outline(x + r, y + r, r as u32);
    // Outline right cap
    graphics_circle_outline(x + w as i32 - r, y + r, r as u32);
}

/// Create SVG resource.
/// Register SVG resource under a string key.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_svg_register(
    caller: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let data = match super::utils::read_guest_bytes(caller, data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    // Reuse the existing SVG parser logic by feeding bytes directly.
    let svg_str = match std::str::from_utf8(&data) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let tree = match Tree::from_str(svg_str, &resvg::usvg::Options::default()) {
        Ok(t) => t,
        Err(_) => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.svgs.insert(id, tree);
    res.keyed_svgs.insert(key, id);
    1
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_svg_register(key: u64, data_ptr: u32, data_len: u32) -> u32 {
    let data = match super::utils::read_guest_bytes(data_ptr, data_len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    // Reuse the existing SVG parser logic by feeding bytes directly.
    let svg_str = match std::str::from_utf8(&data) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let tree = match Tree::from_str(svg_str, &resvg::usvg::Options::default()) {
        Ok(t) => t,
        Err(_) => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.svgs.insert(id, tree);
    res.keyed_svgs.insert(key, id);
    1
}

/// Draw keyed SVG.
pub fn graphics_svg_draw_key(key: u64, x: i32, y: i32, w: u32, h: u32) {
    let id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_svgs.get(&key).copied()
    };

    if let Some(id) = id {
        graphics_svg_draw(id, x, y, w, h);
    }
}

/// Unregister keyed SVG and free the underlying resource.
pub fn graphics_svg_unregister(key: u64) {
    let id = {
        let mut res = RESOURCES.lock().unwrap();
        res.keyed_svgs.remove(&key)
    };

    if let Some(id) = id {
        graphics_svg_destroy(id);
    }
}

/// Draw SVG.
pub fn graphics_svg_draw(id: u32, x: i32, y: i32, w: u32, h: u32) {
    let res = RESOURCES.lock().unwrap();
    if let Some(tree) = res.svgs.get(&id) {
        let pixmap_size = tiny_skia::IntSize::from_wh(w, h).unwrap();
        let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

        let sx = w as f32 / tree.size().width();
        let sy = h as f32 / tree.size().height();
        let transform = tiny_skia::Transform::from_scale(sx, sy);

        resvg::render(tree, transform, &mut pixmap.as_mut());
        // Now draw pixmap as image
        let rgba_data: Vec<u8> = pixmap
            .data()
            .chunks_exact(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2], rgba[3]])
            .collect();
        graphics_image_from_host(x, y, w, h, &rgba_data);
    }
}

/// Destroy SVG.
pub fn graphics_svg_destroy(id: u32) {
    let mut res = RESOURCES.lock().unwrap();
    res.svgs.remove(&id);
}

/// Create GIF resource.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_gif_create(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) -> u32 {
    let data = match super::utils::read_guest_bytes(env, ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let cursor = std::io::Cursor::new(&data);
    let mut decoder = match gif::DecodeOptions::new().read_info(cursor) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    // Decode all frames up-front and store in `GifResource` (frames = RGBA bytes per frame).
    // NOTE: This is a minimal decoder path that ignores disposal/blending; it matches
    // the simple resource model used elsewhere in this crate.
    let width = decoder.width();
    let height = decoder.height();

    // Cache the global palette up-front so we never need to borrow `decoder` while a `frame`
    // (which borrows from `decoder`) is alive.
    let global_palette_bytes: Option<Vec<u8>> = decoder.global_palette().map(|p| p.to_vec());

    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut delays: Vec<u16> = Vec::new();

    loop {
        match decoder.read_next_frame() {
            Ok(Some(frame)) => {
                // GIF frame buffer is indexed color; map through the active palette.
                // `decoder.read_next_frame()` returns a frame that borrows from `decoder`,
                // so we must not call back into `decoder` (even immutably) while `frame`
                // is alive. Copy the palette bytes we need first.
                let palette_bytes: Vec<u8> = if let Some(p) = frame.palette.as_deref() {
                    p.to_vec()
                } else if let Some(p) = global_palette_bytes.as_deref() {
                    p.to_vec()
                } else {
                    return 0;
                };

                let w = width as usize;
                let h = height as usize;

                // Expand indices -> RGBA8888.
                let mut rgba = vec![0u8; w * h * 4];
                for (i, &idx) in frame.buffer.iter().enumerate() {
                    let pi = (idx as usize) * 3;
                    if pi + 2 >= palette_bytes.len() {
                        return 0;
                    }
                    let r = palette_bytes[pi];
                    let g = palette_bytes[pi + 1];
                    let b = palette_bytes[pi + 2];

                    let out = i * 4;
                    rgba[out] = r;
                    rgba[out + 1] = g;
                    rgba[out + 2] = b;
                    rgba[out + 3] = 255;
                }

                frames.push(rgba);
                delays.push(frame.delay);
            }
            Ok(None) => break,
            Err(_) => return 0,
        }
    }

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.gifs.insert(
        id,
        GifResource {
            frames,
            delays,
            width,
            height,
        },
    );
    id
}

/// Create GIF resource (wasm32/web).
#[cfg(target_arch = "wasm32")]
pub fn graphics_gif_create(ptr: u32, len: u32) -> u32 {
    let data = match super::utils::read_guest_bytes(ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let cursor = std::io::Cursor::new(&data);
    let mut decoder = match gif::DecodeOptions::new().read_info(cursor) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let width = decoder.width();
    let height = decoder.height();
    let global_palette: Option<Vec<u8>> = decoder.global_palette().map(|p| p.to_vec());

    let mut frames = Vec::new();
    let mut delays = Vec::new();

    // Canvas for composition (RGBA)
    let mut canvas = vec![0u8; width as usize * height as usize * 4];
    // Backup for "Restore to Previous" disposal
    let mut previous_canvas = canvas.clone();

    let mut last_disposal = gif::DisposalMethod::Any;
    let mut last_rect = (0u16, 0u16, 0u16, 0u16); // left, top, width, height

    while let Some(frame) = match decoder.read_next_frame() {
        Ok(f) => f,
        Err(_) => return 0,
    } {
        // 1. Handle disposal of the *previous* frame
        match last_disposal {
            gif::DisposalMethod::Any | gif::DisposalMethod::Keep => {
                // Do nothing, draw on top
            }
            gif::DisposalMethod::Background => {
                // Restore background (transparent) for the area of the previous frame
                let (lx, ly, lw, lh) = last_rect;
                for y in ly..(ly + lh) {
                    if y >= height {
                        break;
                    }
                    for x in lx..(lx + lw) {
                        if x >= width {
                            break;
                        }
                        let idx = ((y as usize) * (width as usize) + (x as usize)) * 4;
                        if idx + 3 < canvas.len() {
                            canvas[idx] = 0;
                            canvas[idx + 1] = 0;
                            canvas[idx + 2] = 0;
                            canvas[idx + 3] = 0;
                        }
                    }
                }
            }
            gif::DisposalMethod::Previous => {
                // Restore to state before previous frame
                canvas = previous_canvas.clone();
            }
        }

        // Save state if *current* frame says "Restore to Previous" (for the next iteration)
        if frame.dispose == gif::DisposalMethod::Previous {
            previous_canvas = canvas.clone();
        }

        last_disposal = frame.dispose;
        last_rect = (frame.left, frame.top, frame.width, frame.height);

        // 2. Draw current frame onto canvas
        let palette: Option<&[u8]> = frame.palette.as_deref().or(global_palette.as_deref());
        if let Some(palette) = palette {
            let transparent_idx = frame.transparent;
            let fw = frame.width as usize;
            let fh = frame.height as usize;
            let fl = frame.left as usize;
            let ft = frame.top as usize;

            // Helper to write a pixel
            let mut put_pixel = |x: usize, y: usize, color_idx: u8| {
                if Some(color_idx) == transparent_idx {
                    return;
                }
                let base = (color_idx as usize) * 3;
                if base + 2 >= palette.len() {
                    return;
                }
                let r = palette[base];
                let g = palette[base + 1];
                let b = palette[base + 2];

                let cx = fl + x;
                let cy = ft + y;
                if cx < width as usize && cy < height as usize {
                    let idx = (cy * (width as usize) + cx) * 4;
                    canvas[idx] = r;
                    canvas[idx + 1] = g;
                    canvas[idx + 2] = b;
                    canvas[idx + 3] = 255;
                }
            };

            if frame.interlaced {
                let mut offset = 0;
                // Pass 1: Every 8th row, starting at 0
                for y in (0..fh).step_by(8) {
                    for x in 0..fw {
                        if offset < frame.buffer.len() {
                            put_pixel(x, y, frame.buffer[offset]);
                            offset += 1;
                        }
                    }
                }
                // Pass 2: Every 8th row, starting at 4
                for y in (4..fh).step_by(8) {
                    for x in 0..fw {
                        if offset < frame.buffer.len() {
                            put_pixel(x, y, frame.buffer[offset]);
                            offset += 1;
                        }
                    }
                }
                // Pass 3: Every 4th row, starting at 2
                for y in (2..fh).step_by(4) {
                    for x in 0..fw {
                        if offset < frame.buffer.len() {
                            put_pixel(x, y, frame.buffer[offset]);
                            offset += 1;
                        }
                    }
                }
                // Pass 4: Every 2nd row, starting at 1
                for y in (1..fh).step_by(2) {
                    for x in 0..fw {
                        if offset < frame.buffer.len() {
                            put_pixel(x, y, frame.buffer[offset]);
                            offset += 1;
                        }
                    }
                }
            } else {
                // Normal
                for (i, &idx) in frame.buffer.iter().enumerate() {
                    let x = i % fw;
                    let y = i / fw;
                    put_pixel(x, y, idx);
                }
            }
        }

        frames.push(canvas.clone());
        delays.push(frame.delay);
    }

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.gifs.insert(
        id,
        GifResource {
            frames,
            delays,
            width,
            height,
        },
    );
    id
}

/// Draw GIF at natural size.
pub fn graphics_gif_draw(id: u32, x: i32, y: i32) {
    graphics_gif_draw_scaled(id, x, y, 0, 0); // 0 means natural
}

/// Draw GIF scaled.
pub fn graphics_gif_draw_scaled(id: u32, x: i32, y: i32, w: u32, h: u32) {
    let res = RESOURCES.lock().unwrap();
    if let Some(gif) = res.gifs.get(&id) {
        let millis = system_millis();
        let total_delay_ms: u64 = gif.delays.iter().map(|&d| d as u64 * 10).sum();

        let mut frame_idx = 0;
        if total_delay_ms > 0 {
            let mut rem = millis % total_delay_ms;
            for (i, &d) in gif.delays.iter().enumerate() {
                let d_ms = d as u64 * 10;
                // Treat 0 delay as 100ms (common GIF viewer behavior)
                let effective_delay = if d_ms == 0 { 100 } else { d_ms };
                if rem < effective_delay {
                    frame_idx = i;
                    break;
                }
                rem = rem.saturating_sub(effective_delay);
            }
        }

        let src_rgba = &gif.frames[frame_idx];
        let src_w = gif.width as u32;
        let src_h = gif.height as u32;

        // Natural size if either dimension is 0.
        if w == 0 || h == 0 {
            graphics_image_from_host(x, y, src_w, src_h, src_rgba);
            return;
        }

        // Nearest-neighbor resample into a temporary RGBA buffer, then blit.
        // This keeps the public API unchanged (host-side draw from RGBA).
        if src_w == 0 || src_h == 0 {
            return;
        }

        let mut dst = vec![0u8; (w as usize).saturating_mul(h as usize).saturating_mul(4)];
        for dy in 0..h {
            let sy = (dy as u64 * src_h as u64 / h as u64) as u32;
            let sy = sy.min(src_h.saturating_sub(1));
            for dx in 0..w {
                let sx = (dx as u64 * src_w as u64 / w as u64) as u32;
                let sx = sx.min(src_w.saturating_sub(1));

                let sidx = ((sy as usize) * (src_w as usize) + (sx as usize)) * 4;
                let didx = ((dy as usize) * (w as usize) + (dx as usize)) * 4;

                if sidx + 3 < src_rgba.len() && didx + 3 < dst.len() {
                    dst[didx] = src_rgba[sidx];
                    dst[didx + 1] = src_rgba[sidx + 1];
                    dst[didx + 2] = src_rgba[sidx + 2];
                    dst[didx + 3] = src_rgba[sidx + 3];
                }
            }
        }

        graphics_image_from_host(x, y, w, h, &dst);
    }
}

/// Destroy GIF.
pub fn graphics_gif_destroy(id: u32) {
    let mut res = RESOURCES.lock().unwrap();
    res.gifs.remove(&id);
}

/// Register GIF resource under a string key.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_gif_register(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let id = graphics_gif_create(env, data_ptr, data_len);
    if id == 0 {
        return 0;
    }

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_gifs.insert(key, id);
    1
}

/// Register GIF resource under a string key (wasm32/web).
#[cfg(target_arch = "wasm32")]
pub fn graphics_gif_register(key: u64, data_ptr: u32, data_len: u32) -> u32 {
    let id = graphics_gif_create(data_ptr, data_len);
    if id == 0 {
        return 0;
    }

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_gifs.insert(key, id);
    1
}

/// Draw keyed GIF at natural size.
pub fn graphics_gif_draw_key(key: u64, x: i32, y: i32) {
    let id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_gifs.get(&key).copied()
    };

    if let Some(id) = id {
        graphics_gif_draw(id, x, y);
    }
}

/// Draw keyed GIF scaled.
pub fn graphics_gif_draw_key_scaled(key: u64, x: i32, y: i32, w: u32, h: u32) {
    let id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_gifs.get(&key).copied()
    };

    if let Some(id) = id {
        graphics_gif_draw_scaled(id, x, y, w, h);
    }
}

/// Unregister keyed GIF and destroy its underlying resource.
pub fn graphics_gif_unregister(key: u64) {
    let id = {
        let mut res = RESOURCES.lock().unwrap();
        res.keyed_gifs.remove(&key)
    };

    if let Some(id) = id {
        graphics_gif_destroy(id);
    }
}

pub(crate) fn aseprite_build_resource_safe(data: &[u8]) -> Option<AsepriteResource> {
    if data.is_empty() {
        return None;
    }
    let result = std::panic::catch_unwind(|| {
        let aseprite = AsepriteFile::read(Cursor::new(data)).ok()?;

        let width = aseprite.width() as u16;
        let height = aseprite.height() as u16;

        let mut frames = Vec::new();
        let mut delays = Vec::new();
        let mut tags = Vec::new();

        let frame_count = aseprite.num_frames();
        for frame_idx in 0..frame_count {
            let frame = aseprite.frame(frame_idx);
            delays.push(frame.duration() as u16);
            let image = frame.image();
            frames.push(image.into_raw());
        }

        let tag_count = aseprite.num_tags();
        for tag_id in 0..tag_count {
            if let Some(tag) = aseprite.get_tag(tag_id) {
                tags.push((
                    tag.name().to_string(),
                    tag.from_frame() as usize,
                    tag.to_frame() as usize + 1,
                ));
            }
        }

        Some(AsepriteResource {
            frames,
            delays,
            width,
            height,
            tags,
        })
    });

    match result {
        Ok(res) => res,
        Err(_) => None,
    }
}

/// Create an Aseprite resource from guest memory.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_aseprite_create(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) -> u32 {
    let data = match super::utils::read_guest_bytes(env, ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    if data.is_empty() {
        return 0;
    }

    let aseprite_resource = match aseprite_build_resource_safe(&data) {
        Some(r) => r,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.aseprites.insert(id, aseprite_resource);
    id
}

/// Create an Aseprite resource from guest memory (wasm32/web).
#[cfg(target_arch = "wasm32")]
pub fn graphics_aseprite_create(ptr: u32, len: u32) -> u32 {
    let data = match super::utils::read_guest_bytes(ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    if data.is_empty() {
        return 0;
    }

    let aseprite_resource = match aseprite_build_resource_safe(&data) {
        Some(r) => r,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.aseprites.insert(id, aseprite_resource);
    id
}

/// Draw Aseprite frame at natural size.
pub fn graphics_aseprite_draw(id: u32, x: i32, y: i32, frame: u32) {
    graphics_aseprite_draw_scaled(id, x, y, frame, 0, 0);
}

/// Draw Aseprite frame scaled.
pub fn graphics_aseprite_draw_scaled(id: u32, x: i32, y: i32, frame: u32, w: u32, h: u32) {
    let res = RESOURCES.lock().unwrap();
    if let Some(ase) = res.aseprites.get(&id) {
        let frame_idx = frame as usize;
        if frame_idx >= ase.frames.len() {
            return;
        }

        let src_rgba = &ase.frames[frame_idx];
        let src_w = ase.width as u32;
        let src_h = ase.height as u32;

        if w == 0 || h == 0 {
            graphics_image_from_host(x, y, src_w, src_h, src_rgba);
            return;
        }

        if src_w == 0 || src_h == 0 {
            return;
        }

        let mut dst = vec![0u8; (w as usize).saturating_mul(h as usize).saturating_mul(4)];
        for dy in 0..h {
            let sy = (dy as u64 * src_h as u64 / h as u64) as u32;
            let sy = sy.min(src_h.saturating_sub(1));
            for dx in 0..w {
                let sx = (dx as u64 * src_w as u64 / w as u64) as u32;
                let sx = sx.min(src_w.saturating_sub(1));

                let sidx = ((sy as usize) * (src_w as usize) + (sx as usize)) * 4;
                let didx = ((dy as usize) * (w as usize) + (dx as usize)) * 4;

                if sidx + 3 < src_rgba.len() && didx + 3 < dst.len() {
                    dst[didx] = src_rgba[sidx];
                    dst[didx + 1] = src_rgba[sidx + 1];
                    dst[didx + 2] = src_rgba[sidx + 2];
                    dst[didx + 3] = src_rgba[sidx + 3];
                }
            }
        }

        graphics_image_from_host(x, y, w, h, &dst);
    }
}

/// Play Aseprite animation by tag name scaled.
pub fn graphics_aseprite_play(id: u32, x: i32, y: i32, tag_name: &str) {
    graphics_aseprite_play_scaled(id, x, y, tag_name, 0, 0);
}

pub(crate) fn aseprite_select_frame(
    ase: &AsepriteResource,
    tag_name: &str,
    now_ms: u64,
) -> Option<usize> {
    if ase.frames.is_empty() {
        return None;
    }

    let (mut start, mut end) =
        if let Some((_, start, end)) = ase.tags.iter().find(|(name, _, _)| name == tag_name) {
            (*start, *end)
        } else {
            (0, ase.frames.len())
        };

    end = end.min(ase.frames.len());
    start = start.min(end);
    if start >= end {
        return None;
    }

    let mut total_delay_ms: u64 = 0;
    for i in start..end {
        if i < ase.delays.len() {
            total_delay_ms += ase.delays[i] as u64;
        }
    }

    let mut frame_idx = start;
    if total_delay_ms > 0 {
        let mut rem = now_ms % total_delay_ms;
        for i in start..end {
            if i >= ase.delays.len() {
                break;
            }
            let d_ms = ase.delays[i] as u64;
            let effective_delay = if d_ms == 0 { 100 } else { d_ms };
            if rem < effective_delay {
                frame_idx = i;
                break;
            }
            rem = rem.saturating_sub(effective_delay);
        }
    }

    Some(frame_idx)
}

/// Play Aseprite animation by tag name scaled.
pub fn graphics_aseprite_play_scaled(id: u32, x: i32, y: i32, tag_name: &str, w: u32, h: u32) {
    let frame_idx = {
        let res = RESOURCES.lock().unwrap();
        if let Some(ase) = res.aseprites.get(&id) {
            let millis = system_millis();
            aseprite_select_frame(ase, tag_name, millis)
        } else {
            None
        }
    };

    if let Some(frame_idx) = frame_idx {
        graphics_aseprite_draw_scaled(id, x, y, frame_idx as u32, w, h);
    }
}

/// Play Aseprite animation by tag name.
pub fn graphics_aseprite_play_key(key: u64, x: i32, y: i32, tag_name: &str) {
    graphics_aseprite_play_key_scaled(key, x, y, tag_name, 0, 0);
}

/// Play Aseprite animation by tag name scaled.
pub fn graphics_aseprite_play_key_scaled(key: u64, x: i32, y: i32, tag_name: &str, w: u32, h: u32) {
    let id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_aseprites.get(&key).copied()
    };

    if let Some(id) = id {
        graphics_aseprite_play_scaled(id, x, y, tag_name, w, h);
    }
}

/// Destroy Aseprite by id.
pub fn graphics_aseprite_destroy(id: u32) {
    let mut res = RESOURCES.lock().unwrap();
    res.aseprites.remove(&id);
}

/// Register Aseprite resource under a string key.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_aseprite_register(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let id = graphics_aseprite_create(env, data_ptr, data_len);
    if id == 0 {
        return 0;
    }

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_aseprites.insert(key, id);
    1
}

/// Register Aseprite resource under a string key (wasm32/web).
#[cfg(target_arch = "wasm32")]
pub fn graphics_aseprite_register(key: u64, data_ptr: u32, data_len: u32) -> u32 {
    let id = graphics_aseprite_create(data_ptr, data_len);
    if id == 0 {
        return 0;
    }

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_aseprites.insert(key, id);
    1
}

/// Draw Aseprite by key at natural size with specific frame.
pub fn graphics_aseprite_draw_key(key: u64, x: i32, y: i32, frame: u32) {
    graphics_aseprite_draw_key_scaled(key, x, y, frame, 0, 0);
}

/// Draw Aseprite by key scaled with specific frame.
pub fn graphics_aseprite_draw_key_scaled(key: u64, x: i32, y: i32, frame: u32, w: u32, h: u32) {
    let id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_aseprites.get(&key).copied()
    };

    if let Some(id) = id {
        graphics_aseprite_draw_scaled(id, x, y, frame, w, h);
    }
}

/// Unregister Aseprite by key and destroy its underlying resource.
pub fn graphics_aseprite_unregister(key: u64) {
    let id = {
        let mut res = RESOURCES.lock().unwrap();
        res.keyed_aseprites.remove(&key)
    };

    if let Some(id) = id {
        graphics_aseprite_destroy(id);
    }
}

/// Upload a TTF/OTF font from guest memory and return a host-side font id.
///
/// This is a **host-internal** helper used by `graphics_font_register_ttf`.
///
/// Guest ABI (callers):
/// - Guests call `wasm96_graphics_font_register_ttf(key, data_ptr, data_len)`.
/// - The host reads `data_ptr..data_ptr+data_len` from the guest's linear memory and attempts to
///   parse the font.
///
/// Returns:
/// - `0` on failure (invalid pointer/length, invalid font bytes, parse failure)
/// - a non-zero font id on success (stored in the host resource table)
///
/// Notes:
/// - The host stores the parsed `Font` in `RESOURCES.fonts` under the returned id.
/// - The guest never sees this id; guests use the original `key` (u64) when drawing/measuring text.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_font_upload_ttf(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) -> u32 {
    let data = match super::utils::read_guest_bytes(env, ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let font = match Font::from_bytes(data, FontSettings::default()) {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.fonts.insert(id, FontResource::Ttf(font));
    id
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_font_upload_ttf(
    _env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) -> u32 {
    let data = match super::utils::read_guest_bytes(ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let font = match Font::from_bytes(data, FontSettings::default()) {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.fonts.insert(id, FontResource::Ttf(font));
    id
}

#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_font_upload_bdf(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) -> u32 {
    let data = match super::utils::read_guest_bytes(env, ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let (glyphs, width, height) = match parse_bdf(&data) {
        Some(res) => res,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.fonts.insert(
        id,
        FontResource::Bdf {
            width,
            height,
            glyphs,
        },
    );
    id
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_font_upload_bdf(
    _env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) -> u32 {
    let data = match super::utils::read_guest_bytes(ptr, len) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let (glyphs, width, height) = match parse_bdf(&data) {
        Some(res) => res,
        None => return 0,
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.fonts.insert(
        id,
        FontResource::Bdf {
            width,
            height,
            glyphs,
        },
    );
    id
}

/// Register a TTF/OTF font under a key.
///
/// Guest ABI:
/// - `key`: arbitrary u64 selected by the guest (often a hashed string).
/// - `data_ptr/data_len`: encoded TTF/OTF bytes in guest memory.
///
/// Returns:
/// - `1` on success
/// - `0` on failure
///
/// What "success" means:
/// - The host successfully parsed the font bytes and stored a `FontResource::Ttf` in the host
///   resource table.
/// - The `key` now maps to that font resource until `graphics_font_unregister(key)` is called.
///
/// Recommended usage:
/// - Register fonts once during guest `setup()`.
/// - Reuse the same key each frame when rendering or measuring.
pub fn graphics_font_register_ttf(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let id = graphics_font_upload_ttf(env, data_ptr, data_len);
    if id == 0 {
        return 0;
    }

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_fonts.insert(key, id);
    1
}

/// Register a BDF bitmap font under a key.
///
/// Guest ABI:
/// - `key`: arbitrary u64 selected by the guest (often a hashed string).
/// - `data_ptr/data_len`: BDF file bytes in guest memory (text-based format).
///
/// Returns:
/// - `1` on success
/// - `0` on failure
///
/// What "success" means:
/// - The host successfully parsed the BDF, extracted a glyph bitmap map, and stored it in the host
///   resource table.
/// - The key now maps to that host font resource until `graphics_font_unregister(key)` is called.
///
/// When to use BDF:
/// - Pixel-art UIs, debug overlays, and deterministic bitmap metrics.
///
/// Caveats:
/// - Current parser is intentionally minimal and expects a relatively well-formed BDF.
/// - Missing glyphs will simply not render (per glyph lookup).
pub fn graphics_font_register_bdf(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    key: u64,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let id = graphics_font_upload_bdf(env, data_ptr, data_len);
    if id == 0 {
        return 0;
    }

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_fonts.insert(key, id);
    1
}

/// Register a built-in Spleen bitmap font under a key.
///
/// The Spleen family is bundled with the host as BDF assets. This function selects one of the
/// supported Spleen sizes and associates it with `key`.
///
/// Guest ABI:
/// - `key`: arbitrary u64 selected by the guest (often a hashed string).
/// - `size`: requested pixel size. Supported sizes are currently:
///   - 8, 16, 24, 32, 64
///
/// Returns:
/// - `1` on success
/// - `0` if `size` is unsupported or the built-in BDF could not be parsed.
///
/// Why register Spleen if there is a fallback?
/// - The host fallback (when `font_key` is unknown) uses Spleen size 16 only.
/// - Registering Spleen under your own key lets you choose sizes and ensures stable layout.
///
/// See also:
/// - `graphics_text_key` and `graphics_text_measure_key` fallback to Spleen size 16 when missing.
pub fn graphics_font_register_spleen(key: u64, size: u32) -> u32 {
    let id = graphics_font_use_spleen(size);
    if id == 0 {
        return 0;
    }

    let mut res = RESOURCES.lock().unwrap();
    res.keyed_fonts.insert(key, id);
    1
}

/// Unregister a keyed font.
///
/// Guest ABI:
/// - `key`: the u64 key previously used to register a font.
///
/// Behavior:
/// - Removes the key -> font-id mapping from `RESOURCES.keyed_fonts`.
/// - Drops the underlying font resource from `RESOURCES.fonts` (if present).
///
/// After unregistering:
/// - Calls to `graphics_text_key` / `graphics_text_measure_key` using this key will again behave as
///   "missing font key" and therefore use the fallback Spleen size 16.
pub fn graphics_font_unregister(key: u64) {
    let id = {
        let mut res = RESOURCES.lock().unwrap();
        res.keyed_fonts.remove(&key)
    };

    if let Some(id) = id {
        let mut res = RESOURCES.lock().unwrap();
        res.fonts.remove(&id);
    }
}

/// Draw UTF-8 text using a keyed font at the given screen position.
///
/// Guest ABI:
/// - `x`, `y`: top-left origin in screen coordinates.
/// - `font_key`: u64 key of a registered font.
/// - `text_ptr/text_len`: UTF-8 bytes in guest memory.
///
/// Fallback behavior:
/// - If `font_key` is not registered, the host falls back to built-in Spleen size 16.
///   This makes text work out-of-the-box even if the guest never registered a font.
///
/// Notes:
/// - The host reads the string bytes from guest memory immediately during this call.
/// - If the host cannot access guest memory or cannot resolve a font id (including fallback),
///   this function becomes a no-op.
/// - Rendering is alpha-blended in the host (TTF/OTF smoothing + proper blending are handled by host).
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_text_key(
    x: i32,
    y: i32,
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    font_key: u64,
    text_ptr: u32,
    text_len: u32,
) {
    let font_id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_fonts.get(&font_key).copied()
    };

    // If no keyed font is registered, fall back to built-in Spleen at size 16.
    // This makes text rendering work out-of-the-box even if the guest never
    // called `wasm96_graphics_font_register_*`.
    let font_id = match font_id {
        Some(id) => id,
        None => graphics_font_use_spleen(16),
    };

    if font_id == 0 {
        return;
    }

    graphics_text(x, y, font_id, env, text_ptr, text_len);
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_text_key(x: i32, y: i32, font_key: u64, text_ptr: u32, text_len: u32) {
    let font_id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_fonts.get(&font_key).copied()
    };

    let font_id = match font_id {
        Some(id) => id,
        None => graphics_font_use_spleen(16),
    };

    if font_id == 0 {
        return;
    }

    let text = match super::utils::read_guest_string(text_ptr, text_len) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Reuse the existing font rendering paths by looking up the font resource.
    let res = RESOURCES.lock().unwrap();
    if let Some(font) = res.fonts.get(&font_id) {
        match font {
            FontResource::Ttf(f) => {
                // Mirror the simple TTF text rendering logic used by the native path,
                // but source the string from web memory.
                let mut s = match global().lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let width = s.video.width as i32;
                let height = s.video.height as i32;
                let draw_color = s.video.draw_color;
                let r_fg = ((draw_color >> 16) & 0xFF) as f32;
                let g_fg = ((draw_color >> 8) & 0xFF) as f32;
                let b_fg = (draw_color & 0xFF) as f32;
                let r_fg_sq = r_fg * r_fg;
                let g_fg_sq = g_fg * g_fg;
                let b_fg_sq = b_fg * b_fg;

                let mut px = x as f32;
                for ch in text.chars() {
                    let (metrics, bitmap) = f.rasterize(ch, 16.0); // fixed size
                    let start_x = px.round() as i32;
                    for (i, &alpha) in bitmap.iter().enumerate() {
                        if alpha > 0 {
                            let gx = start_x + (i % metrics.width) as i32;
                            let gy = y + (i / metrics.width) as i32;

                            if gx >= 0 && gx < width && gy >= 0 && gy < height {
                                let stride = s.video.stride_pixels as i32;
                                let idx = (gy * stride + gx) as usize;
                                let bg = s.video.framebuffer[idx];

                                let a = alpha as f32 / 255.0;
                                let inv_a = 1.0 - a;

                                let r_bg = ((bg >> 16) & 0xFF) as f32;
                                let g_bg = ((bg >> 8) & 0xFF) as f32;
                                let b_bg = (bg & 0xFF) as f32;

                                let r = (r_fg_sq * a + r_bg * r_bg * inv_a).sqrt() as u32;
                                let g = (g_fg_sq * a + g_bg * g_bg * inv_a).sqrt() as u32;
                                let b = (b_fg_sq * a + b_bg * b_bg * inv_a).sqrt() as u32;

                                s.video.framebuffer[idx] = (r << 16) | (g << 8) | b;
                            }
                        }
                    }
                    px += metrics.advance_width;
                }
            }
            FontResource::Bdf {
                width,
                height,
                glyphs,
            } => {
                let stride = (*width + 7) / 8;
                let mut px = x;
                for ch in text.chars() {
                    if let Some(bitmap) = glyphs.get(&ch) {
                        for row in 0..*height as usize {
                            for byte_idx in 0..stride as usize {
                                let idx = row * stride as usize + byte_idx;
                                if idx < bitmap.len() {
                                    let byte = bitmap[idx];
                                    for bit in 0..8 {
                                        let col = byte_idx * 8 + bit;
                                        if col < *width as usize {
                                            if (byte & (1 << (7 - bit))) != 0 {
                                                graphics_point(px + col as i32, y + row as i32);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    px += *width as i32;
                }
            }
        }
    }
}

/// Measure UTF-8 text using a keyed font.
///
/// This is intended for layout (centering, right-align, UI sizing).
///
/// Guest ABI:
/// - `font_key`: u64 key of a registered font.
/// - `text_ptr/text_len`: UTF-8 bytes in guest memory.
///
/// Return value:
/// - Packed `u64`: `(width << 32) | height`, where `width` and `height` are pixel dimensions.
/// - Returns `0` if measurement cannot be performed (e.g. memory access failure or no font even
///   after fallback).
///
/// Fallback behavior:
/// - If `font_key` is not registered, the host measures using the same fallback as drawing:
///   built-in Spleen size 16. This keeps measured size consistent with `graphics_text_key`.
///
/// Notes:
/// - Like draw, measurement reads the string bytes immediately from guest memory.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_text_measure_key(
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    font_key: u64,
    text_ptr: u32,
    text_len: u32,
) -> u64 {
    let font_id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_fonts.get(&font_key).copied()
    };

    // If no keyed font is registered, fall back to built-in Spleen at size 16.
    // This keeps measure behavior consistent with `graphics_text_key`.
    let font_id = match font_id {
        Some(id) => id,
        None => graphics_font_use_spleen(16),
    };

    if font_id == 0 {
        return 0;
    }

    graphics_text_measure(font_id, env, text_ptr, text_len)
}

#[cfg(target_arch = "wasm32")]
pub fn graphics_text_measure_key(font_key: u64, text_ptr: u32, text_len: u32) -> u64 {
    let font_id = {
        let res = RESOURCES.lock().unwrap();
        res.keyed_fonts.get(&font_key).copied()
    };

    let font_id = match font_id {
        Some(id) => id,
        None => graphics_font_use_spleen(16),
    };

    if font_id == 0 {
        return 0;
    }

    let text = match super::utils::read_guest_string(text_ptr, text_len) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let res = RESOURCES.lock().unwrap();
    let Some(font) = res.fonts.get(&font_id) else {
        return 0;
    };

    match font {
        FontResource::Ttf(f) => {
            // Very close to the native measurement: sum advance widths; height fixed.
            let mut w = 0.0f32;
            for ch in text.chars() {
                let (metrics, _bitmap) = f.rasterize(ch, 16.0);
                w += metrics.advance_width;
            }
            let width = w.ceil().max(0.0) as u32;
            let height = 16u32;
            ((width as u64) << 32) | (height as u64)
        }
        FontResource::Bdf { width, height, .. } => {
            let width_px = (*width as u64) * (text.chars().count() as u64);
            ((width_px as u64) << 32) | (*height as u64)
        }
    }
}

/// Parse BDF font data into glyph map.
fn parse_bdf(bdf_data: &[u8]) -> Option<(HashMap<char, Vec<u8>>, u32, u32)> {
    let text = core::str::from_utf8(bdf_data).ok()?;
    let mut glyphs = HashMap::new();
    let mut lines = text.lines();
    let mut width = 0;
    let mut height = 0;

    while let Some(line) = lines.next() {
        if line.starts_with("FONTBOUNDINGBOX") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                width = parts[1].parse().unwrap_or(0);
                height = parts[2].parse().unwrap_or(0);
            }
        } else if line.starts_with("STARTCHAR") {
            let mut encoding = None;
            let mut bitmap = Vec::new();
            let mut in_bitmap = false;
            for inner_line in lines.by_ref() {
                if inner_line.starts_with("ENCODING") {
                    if let Some(enc_str) = inner_line.split_whitespace().nth(1) {
                        encoding = enc_str.parse::<u32>().ok().and_then(char::from_u32);
                    }
                } else if inner_line == "BITMAP" {
                    in_bitmap = true;
                } else if inner_line == "ENDCHAR" {
                    break;
                } else if in_bitmap {
                    let hex = inner_line.trim();
                    // Parse hex string into bytes. Each 2 hex chars is a byte.
                    for i in (0..hex.len()).step_by(2) {
                        if i + 2 <= hex.len() {
                            if let Ok(byte) = u8::from_str_radix(&hex[i..i + 2], 16) {
                                bitmap.push(byte);
                            }
                        }
                    }
                }
            }
            if let Some(ch) = encoding {
                glyphs.insert(ch, bitmap);
            }
        }
    }

    if width > 0 && height > 0 {
        Some((glyphs, width, height))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_video(width: u32, height: u32) {
        let mut s = match global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.video.width = width;
        s.video.height = height;
        s.video.stride_pixels = width;
        s.video.pitch_bytes = (width as usize) * 4;
        s.video.framebuffer = vec![0; (width * height) as usize];
        s.video.draw_color = 0xFFFFFFFF;
        s.video.fill_color = 0xFFFFFFFF;
        s.video.stroke_color = 0xFFFFFFFF;
        s.video.fill_enabled = false;
        s.video.stroke_enabled = false;
        s.video.erase_mode_enabled = false;
        s.video.color_mode = crate::state::ColorMode::RGB;
        s.video.clip_rect = None;
        s.video.transform = Mat4::IDENTITY;
        s.video.transform_stack.clear();
        s.video.geometry_dirty = true;
    }

    #[test]
    fn test_parse_bdf_spleen_32x64() {
        let bdf_data = include_bytes!("../assets/spleen-32x64.bdf");
        let (glyphs, width, height) = parse_bdf(bdf_data).expect("Failed to parse BDF");
        assert_eq!(width, 32);
        assert_eq!(height, 64);
        assert!(!glyphs.is_empty());
        assert!(glyphs.contains_key(&'A'));
    }

    #[test]
    fn test_graphics_ellipse_rasterization() {
        reset_video(32, 24);
        graphics_set_color(10, 20, 30, 255);
        graphics_ellipse(16, 12, 10, 6);

        let s = match global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let expected = (255u32 << 24) | (10u32 << 16) | (20u32 << 8) | 30u32;
        let center_idx = (12 * 32 + 16) as usize;
        assert_eq!(s.video.framebuffer[center_idx], expected);
        assert_eq!(s.video.framebuffer[0], 0);
    }

    #[test]
    fn test_graphics_arc_rasterization() {
        reset_video(40, 40);
        graphics_set_color(1, 2, 3, 255);
        graphics_arc(20, 20, 10, 10, 0.0, std::f32::consts::FRAC_PI_2);

        let s = match global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let expected = (255u32 << 24) | (1u32 << 16) | (2u32 << 8) | 3u32;
        let start_idx = (20 * 40 + 25) as usize;
        let candidate_idx = (20 * 40 + 24) as usize;
        let mut matched = false;
        if start_idx < s.video.framebuffer.len() {
            matched |= s.video.framebuffer[start_idx] == expected;
        }
        if candidate_idx < s.video.framebuffer.len() {
            matched |= s.video.framebuffer[candidate_idx] == expected;
        }
        assert!(matched, "arc should draw a pixel at the start angle");
    }

    #[test]
    fn test_graphics_quad_rasterization() {
        reset_video(20, 20);
        graphics_set_color(200, 100, 50, 255);
        graphics_quad(2, 2, 10, 2, 10, 8, 2, 8);

        let s = match global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let expected = (255u32 << 24) | (200u32 << 16) | (100u32 << 8) | 50u32;
        let inside_idx = (5 * 20 + 5) as usize;
        assert_eq!(s.video.framebuffer[inside_idx], expected);
    }
}

/// Use (load) a built-in Spleen font at the given size and return a host-side font id.
///
/// This is a **host-internal** helper used for:
/// - explicit registration: `graphics_font_register_spleen(key, size)`
/// - fallback behavior: `graphics_text_key` and `graphics_text_measure_key` (size 16)
///
/// Supported sizes:
/// - 8, 16, 24, 32, 64
///
/// Returns:
/// - `0` if the size is unsupported or parsing fails
/// - otherwise, a host font id inserted into `RESOURCES.fonts` as a `FontResource::Bdf`
///
/// Note:
/// - The returned id is not stable across runs and is not exposed to guests. Guests should only
///   rely on their chosen `font_key` values.
pub fn graphics_font_use_spleen(size: u32) -> u32 {
    let data = match size {
        8 => super::resources::SPLEEN_5X8,
        16 => super::resources::SPLEEN_8X16,
        24 => super::resources::SPLEEN_12X24,
        32 => super::resources::SPLEEN_16X32,
        64 => super::resources::SPLEEN_32X64,
        _ => return 0,
    };
    let Some((glyphs, width, height)) = parse_bdf(data) else {
        return 0;
    };

    let mut res = RESOURCES.lock().unwrap();
    let id = res.next_id;
    res.next_id += 1;
    res.fonts.insert(
        id,
        FontResource::Bdf {
            width,
            height,
            glyphs,
        },
    );
    id
}

/// Draw text.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_text(
    x: i32,
    y: i32,
    font_id: u32,
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) {
    let memory_ptr = {
        let s = global().lock().unwrap();
        s.memory_wasmtime
    };
    if memory_ptr.is_null() {
        return;
    }

    let mem = unsafe { &*memory_ptr };

    let mut text_bytes = vec![0u8; len as usize];
    if mem.read(env, ptr as usize, &mut text_bytes).is_err() {
        return;
    }

    let text = match std::str::from_utf8(&text_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    let res = RESOURCES.lock().unwrap();
    if let Some(font) = res.fonts.get(&font_id) {
        match font {
            FontResource::Ttf(f) => {
                // Lock global state once for the whole string to enable blending
                let mut s = match global().lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let width = s.video.width as i32;
                let height = s.video.height as i32;
                let draw_color = s.video.draw_color;
                let r_fg = ((draw_color >> 16) & 0xFF) as f32;
                let g_fg = ((draw_color >> 8) & 0xFF) as f32;
                let b_fg = (draw_color & 0xFF) as f32;
                let r_fg_sq = r_fg * r_fg;
                let g_fg_sq = g_fg * g_fg;
                let b_fg_sq = b_fg * b_fg;

                let mut px = x as f32;
                for ch in text.chars() {
                    let (metrics, bitmap) = f.rasterize(ch, 16.0); // fixed size
                    let start_x = px.round() as i32;
                    for (i, &alpha) in bitmap.iter().enumerate() {
                        if alpha > 0 {
                            let gx = start_x + (i % metrics.width) as i32;
                            let gy = y + (i / metrics.width) as i32;

                            if gx >= 0 && gx < width && gy >= 0 && gy < height {
                                let stride = s.video.stride_pixels as i32;
                                let idx = (gy * stride + gx) as usize;
                                let bg = s.video.framebuffer[idx];

                                // Alpha blend (gamma-correct approximation)
                                let a = alpha as f32 / 255.0;
                                let inv_a = 1.0 - a;

                                let r_bg = ((bg >> 16) & 0xFF) as f32;
                                let g_bg = ((bg >> 8) & 0xFF) as f32;
                                let b_bg = (bg & 0xFF) as f32;

                                let r = (r_fg_sq * a + r_bg * r_bg * inv_a).sqrt() as u32;
                                let g = (g_fg_sq * a + g_bg * g_bg * inv_a).sqrt() as u32;
                                let b = (b_fg_sq * a + b_bg * b_bg * inv_a).sqrt() as u32;

                                s.video.framebuffer[idx] = (r << 16) | (g << 8) | b;
                            }
                        }
                    }
                    px += metrics.advance_width;
                }
            }
            FontResource::Bdf {
                width,
                height,
                glyphs,
            } => {
                let stride = (width + 7) / 8;
                let mut px = x;
                for ch in text.chars() {
                    if let Some(bitmap) = glyphs.get(&ch) {
                        for row in 0..*height as usize {
                            for byte_idx in 0..stride as usize {
                                let idx = row * stride as usize + byte_idx;
                                if idx < bitmap.len() {
                                    let byte = bitmap[idx];
                                    for bit in 0..8 {
                                        let col = byte_idx * 8 + bit;
                                        if col < *width as usize {
                                            if (byte & (1 << (7 - bit))) != 0 {
                                                graphics_point(px + col as i32, y + row as i32);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    px += *width as i32;
                }
            }
        }
    }
}

/// Measure text.
#[cfg(not(target_arch = "wasm32"))]
pub fn graphics_text_measure(
    font_id: u32,
    env: &mut Caller<'_, crate::state::Wasm96Ctx>,
    ptr: u32,
    len: u32,
) -> u64 {
    let memory_ptr = {
        let s = global().lock().unwrap();
        s.memory_wasmtime
    };
    if memory_ptr.is_null() {
        return 0;
    }

    let mem = unsafe { &*memory_ptr };

    let mut text_bytes = vec![0u8; len as usize];
    if mem.read(env, ptr as usize, &mut text_bytes).is_err() {
        return 0;
    }

    let text = match std::str::from_utf8(&text_bytes) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let res = RESOURCES.lock().unwrap();
    let (width, height) = if let Some(font) = res.fonts.get(&font_id) {
        match font {
            FontResource::Ttf(f) => {
                let mut width = 0.0;
                let mut height: f32 = 0.0;
                for ch in text.chars() {
                    let (metrics, _) = f.rasterize(ch, 16.0);
                    width += metrics.advance_width;
                    height = height.max(metrics.height as f32);
                }
                (width.round() as u32, height as u32)
            }
            FontResource::Bdf {
                width,
                height,
                glyphs: _,
            } => (text.chars().count() as u32 * *width, *height),
        }
    } else {
        (0, 0)
    };

    ((width as u64) << 32) | (height as u64)
}

/// Present the framebuffer to the platform frontend.
pub fn video_present_host(callbacks: &mut dyn crate::PlatformCallbacks) {
    // Flush any 3D content before presenting (native-only).
    //
    // IMPORTANT: When running under libretro, the frontend (`wasm96-libretro`) is responsible
    // for compositing the 2D software framebuffer into the HW FBO. The engine-side compositor
    // MUST be skipped in that case to avoid a double overlay pass.
    #[cfg(not(target_arch = "wasm32"))]
    {
        // If the frontend is providing a HW framebuffer (FBO != 0), it is responsible for
        // compositing the engine's 2D software framebuffer into that HW target.
        //
        // In that scenario, the engine-side overlay compositor MUST be skipped to avoid a
        // double overlay pass (mirrored/duplicated output).
        //
        // If there is no HW framebuffer, the engine may composite internally (desktop GL path).
        let frontend_has_hw_fbo = callbacks.get_hardware_framebuffer() != 0;

        if super::graphics3d::STATE_3D.lock().unwrap().enabled && !frontend_has_hw_fbo {
            let _ = super::graphics3d::flush_to_host();
        }
    }

    let (width, height, stride_pixels, fb) = {
        let s = match global().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        (
            s.video.width,
            s.video.height,
            s.video.stride_pixels,
            s.video.framebuffer.clone(),
        )
    };

    callbacks.video_refresh(&fb, width, height, stride_pixels);
}
