//! Host import definitions for the Wasmtime and Web runtimes.
//!
//! This module defines all the host functions imported by guest modules under the "env" module.

use crate::{
    abi::{IMPORT_MODULE, host_imports},
    av, input,
};

#[cfg(not(target_arch = "wasm32"))]
use wasmtime::{Caller, Linker};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
fn read_guest_string(caller: &mut Caller<'_, ()>, ptr: u32, len: u32) -> Result<String, String> {
    av::utils::read_guest_string(caller, ptr, len)
        .map_err(|_| String::from("Failed to read guest string"))
}

#[cfg(target_arch = "wasm32")]
fn read_guest_string_web(ptr: u32, len: u32) -> Result<String, String> {
    av::utils::read_guest_string(ptr, len).map_err(|_| String::from("Failed to read guest string"))
}

/// Define all host imports expected by guests under module `"env"` for Wasmtime.
#[cfg(not(target_arch = "wasm32"))]
pub fn define_imports(linker: &mut Linker<()>) -> anyhow::Result<()> {
    // --- Graphics ---
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SET_SIZE,
        |_caller: Caller<'_, ()>, width: u32, height: u32| {
            av::graphics_set_size(width, height);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_APPLY_MATRIX,
        |_caller: Caller<'_, ()>,
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
         m33: f32| {
            av::graphics_apply_matrix(
                m00, m01, m02, m03, m10, m11, m12, m13, m20, m21, m22, m23, m30, m31, m32, m33,
            );
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_RESET_MATRIX,
        |_caller: Caller<'_, ()>| {
            av::graphics_reset_matrix();
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ROTATE,
        |_caller: Caller<'_, ()>, angle: f32| {
            av::graphics_rotate(angle);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ROTATE_X,
        |_caller: Caller<'_, ()>, angle: f32| {
            av::graphics_rotate_x(angle);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ROTATE_Y,
        |_caller: Caller<'_, ()>, angle: f32| {
            av::graphics_rotate_y(angle);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ROTATE_Z,
        |_caller: Caller<'_, ()>, angle: f32| {
            av::graphics_rotate_z(angle);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SCALE,
        |_caller: Caller<'_, ()>, sx: f32, sy: f32, sz: f32| {
            av::graphics_scale(sx, sy, sz);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SHEAR_X,
        |_caller: Caller<'_, ()>, angle: f32| {
            av::graphics_shear_x(angle);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SHEAR_Y,
        |_caller: Caller<'_, ()>, angle: f32| {
            av::graphics_shear_y(angle);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_TRANSLATE,
        |_caller: Caller<'_, ()>, x: f32, y: f32, z: f32| {
            av::graphics_translate(x, y, z);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PUSH_MATRIX,
        |_caller: Caller<'_, ()>| {
            av::graphics_push_matrix();
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_POP_MATRIX,
        |_caller: Caller<'_, ()>| {
            av::graphics_pop_matrix();
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SET_COLOR,
        |_caller: Caller<'_, ()>, r: u32, g: u32, b: u32, a: u32| {
            av::graphics_set_color(r, g, b, a);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_BACKGROUND,
        |_caller: Caller<'_, ()>, r: u32, g: u32, b: u32| {
            av::graphics_background(r, g, b);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_POINT,
        |_caller: Caller<'_, ()>, x: i32, y: i32| {
            av::graphics_point(x, y);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_LINE,
        |_caller: Caller<'_, ()>, x1: i32, y1: i32, x2: i32, y2: i32| {
            av::graphics_line(x1, y1, x2, y2);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_RECT,
        |_caller: Caller<'_, ()>, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_rect(x, y, w, h);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_RECT_OUTLINE,
        |_caller: Caller<'_, ()>, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_rect_outline(x, y, w, h);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_CIRCLE,
        |_caller: Caller<'_, ()>, x: i32, y: i32, r: u32| {
            av::graphics_circle(x, y, r);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_CIRCLE_OUTLINE,
        |_caller: Caller<'_, ()>, x: i32, y: i32, r: u32| {
            av::graphics_circle_outline(x, y, r);
        },
    )?;

    // Raw RGBA blit: (x,y,w,h,ptr,len)
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_IMAGE,
        |mut caller: Caller<'_, ()>, x: i32, y: i32, w: u32, h: u32, ptr: u32, len: u32| {
            let _ = av::graphics_image(&mut caller, x, y, w, h, ptr, len);
        },
    )?;

    // One-shot PNG decode+draw: (x,y,ptr,len)
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_IMAGE_PNG,
        |mut caller: Caller<'_, ()>, x: i32, y: i32, ptr: u32, len: u32| {
            let _ = av::graphics_image_png(&mut caller, x, y, ptr, len);
        },
    )?;

    // One-shot JPEG decode+draw: (x,y,ptr,len)
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_IMAGE_JPEG,
        |mut caller: Caller<'_, ()>, x: i32, y: i32, ptr: u32, len: u32| {
            let _ = av::graphics_image_jpeg(&mut caller, x, y, ptr, len);
        },
    )?;

    // --- Keyed resources (SVG/GIF/PNG/JPEG) ---
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SVG_REGISTER,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| -> u32 {
            av::graphics_svg_register(&mut caller, key, data_ptr, data_len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SVG_DRAW_KEY,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_svg_draw_key(key, x, y, w, h)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SVG_UNREGISTER,
        |_caller: Caller<'_, ()>, key: u64| {
            av::graphics_svg_unregister(key);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_GIF_REGISTER,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| -> u32 {
            av::graphics_gif_register(&mut caller, key, data_ptr, data_len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_GIF_DRAW_KEY,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32| av::graphics_gif_draw_key(key, x, y),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_GIF_DRAW_KEY_SCALED,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_gif_draw_key_scaled(key, x, y, w, h)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_GIF_UNREGISTER,
        |_caller: Caller<'_, ()>, key: u64| {
            av::graphics_gif_unregister(key);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PNG_REGISTER,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| -> u32 {
            av::graphics_png_register(&mut caller, key, data_ptr, data_len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PNG_DRAW_KEY,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32| av::graphics_png_draw_key(key, x, y),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PNG_DRAW_KEY_SCALED,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_png_draw_key_scaled(key, x, y, w, h)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PNG_UNREGISTER,
        |_caller: Caller<'_, ()>, key: u64| {
            av::graphics_png_unregister(key);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_JPEG_REGISTER,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| -> u32 {
            av::graphics_jpeg_register(&mut caller, key, data_ptr, data_len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_JPEG_DRAW_KEY,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32| av::graphics_jpeg_draw_key(key, x, y),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_JPEG_DRAW_KEY_SCALED,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_jpeg_draw_key_scaled(key, x, y, w, h)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_JPEG_UNREGISTER,
        |_caller: Caller<'_, ()>, key: u64| {
            av::graphics_jpeg_unregister(key);
        },
    )?;

    // Keyed resources: Aseprite
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ASEPRITE_REGISTER,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| -> u32 {
            av::graphics_aseprite_register(&mut caller, key, data_ptr, data_len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ASEPRITE_DRAW_KEY,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32, frame: u32| {
            av::graphics_aseprite_draw_key(key, x, y, frame)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ASEPRITE_DRAW_KEY_SCALED,
        |_caller: Caller<'_, ()>, key: u64, x: i32, y: i32, frame: u32, w: u32, h: u32| {
            av::graphics_aseprite_draw_key_scaled(key, x, y, frame, w, h)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ASEPRITE_PLAY_KEY,
        |mut caller: Caller<'_, ()>, key: u64, x: i32, y: i32, tag_ptr: u32, tag_len: u32| {
            let tag = match read_guest_string(&mut caller, tag_ptr, tag_len) {
                Ok(s) => s,
                Err(_) => return,
            };
            av::graphics_aseprite_play_key(key, x, y, &tag);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ASEPRITE_PLAY_KEY_SCALED,
        |mut caller: Caller<'_, ()>,
         key: u64,
         x: i32,
         y: i32,
         tag_ptr: u32,
         tag_len: u32,
         w: u32,
         h: u32| {
            let tag = match read_guest_string(&mut caller, tag_ptr, tag_len) {
                Ok(s) => s,
                Err(_) => return,
            };
            av::graphics_aseprite_play_key_scaled(key, x, y, &tag, w, h);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ASEPRITE_UNREGISTER,
        |_caller: Caller<'_, ()>, key: u64| {
            av::graphics_aseprite_unregister(key);
        },
    )?;

    // Fonts (keyed)
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_FONT_REGISTER_TTF,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| -> u32 {
            av::graphics_font_register_ttf(&mut caller, key, data_ptr, data_len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_FONT_REGISTER_BDF,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| -> u32 {
            av::graphics_font_register_bdf(&mut caller, key, data_ptr, data_len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_FONT_REGISTER_SPLEEN,
        |_caller: Caller<'_, ()>, key: u64, size: u32| -> u32 {
            av::graphics_font_register_spleen(key, size)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_FONT_UNREGISTER,
        |_caller: Caller<'_, ()>, key: u64| {
            av::graphics_font_unregister(key);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_TEXT_KEY,
        |mut caller: Caller<'_, ()>,
         x: i32,
         y: i32,
         font_key: u64,
         text_ptr: u32,
         text_len: u32| {
            av::graphics_text_key(x, y, &mut caller, font_key, text_ptr, text_len);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_TEXT_MEASURE_KEY,
        |mut caller: Caller<'_, ()>, font_key: u64, text_ptr: u32, text_len: u32| -> u64 {
            av::graphics_text_measure_key(&mut caller, font_key, text_ptr, text_len)
        },
    )?;

    // Color functions
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_RED,
        |_caller: Caller<'_, ()>| -> u32 { av::graphics_red() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_GREEN,
        |_caller: Caller<'_, ()>| -> u32 { av::graphics_green() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_BLUE,
        |_caller: Caller<'_, ()>| -> u32 { av::graphics_blue() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ALPHA,
        |_caller: Caller<'_, ()>| -> u32 { av::graphics_alpha() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_BRIGHTNESS,
        |_caller: Caller<'_, ()>| -> u32 { av::graphics_brightness() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_HUE,
        |_caller: Caller<'_, ()>| -> f32 { av::graphics_hue() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SATURATION,
        |_caller: Caller<'_, ()>| -> f32 { av::graphics_saturation() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_LIGHTNESS,
        |_caller: Caller<'_, ()>| -> f32 { av::graphics_lightness() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_COLOR_RGB,
        |_caller: Caller<'_, ()>, r: u32, g: u32, b: u32, a: u32| {
            av::graphics_color_rgb(r, g, b, a);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_COLOR_HSL,
        |_caller: Caller<'_, ()>, h: f32, s: f32, l: f32, a: u32| {
            av::graphics_color_hsl(h, s, l, a);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_LERP_COLOR,
        |_caller: Caller<'_, ()>,
         r1: u32,
         g1: u32,
         b1: u32,
         a1: u32,
         r2: u32,
         g2: u32,
         b2: u32,
         a2: u32,
         t: f32|
         -> u32 { av::graphics_lerp_color(r1, g1, b1, a1, r2, g2, b2, a2, t) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PALETTE_LERP,
        |_caller: Caller<'_, ()>, c1: u32, c2: u32, t: f32| -> u32 {
            av::graphics_palette_lerp(c1, c2, t)
        },
    )?;

    // State functions
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_CLEAR,
        |_caller: Caller<'_, ()>| av::graphics_clear(),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_FILL,
        |_caller: Caller<'_, ()>, r: u32, g: u32, b: u32, a: u32| {
            av::graphics_fill(r, g, b, a);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_NO_FILL,
        |_caller: Caller<'_, ()>| av::graphics_no_fill(),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_STROKE,
        |_caller: Caller<'_, ()>, r: u32, g: u32, b: u32, a: u32| {
            av::graphics_stroke(r, g, b, a);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_NO_STROKE,
        |_caller: Caller<'_, ()>| av::graphics_no_stroke(),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ERASE,
        |_caller: Caller<'_, ()>| av::graphics_erase(),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_NO_ERASE,
        |_caller: Caller<'_, ()>| av::graphics_no_erase(),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_COLOR_MODE,
        |_caller: Caller<'_, ()>, mode: u32| av::graphics_color_mode(mode),
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_CLIP,
        |_caller: Caller<'_, ()>, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_clip(x, y, w, h);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_BEGIN_CLIP,
        |_caller: Caller<'_, ()>, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_begin_clip(x, y, w, h);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_END_CLIP,
        |_caller: Caller<'_, ()>| av::graphics_end_clip(),
    )?;

    // 3D
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ELLIPSE,
        |_caller: Caller<'_, ()>, cx: i32, cy: i32, w: u32, h: u32| {
            av::graphics_ellipse(cx, cy, w, h);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_ARC,
        |_caller: Caller<'_, ()>, cx: i32, cy: i32, w: u32, h: u32, start: f32, end: f32| {
            av::graphics_arc(cx, cy, w, h, start, end);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_QUAD,
        |_caller: Caller<'_, ()>,
         x1: i32,
         y1: i32,
         x2: i32,
         y2: i32,
         x3: i32,
         y3: i32,
         x4: i32,
         y4: i32| {
            av::graphics_quad(x1, y1, x2, y2, x3, y3, x4, y4);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_TRIANGLE,
        |_caller: Caller<'_, ()>, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32| {
            av::graphics_triangle(x1, y1, x2, y2, x3, y3);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_TRIANGLE_OUTLINE,
        |_caller: Caller<'_, ()>, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32| {
            av::graphics_triangle_outline(x1, y1, x2, y2, x3, y3);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_BEZIER_QUADRATIC,
        |_caller: Caller<'_, ()>,
         x1: i32,
         y1: i32,
         cx: i32,
         cy: i32,
         x2: i32,
         y2: i32,
         segments: u32| {
            av::graphics_bezier_quadratic(x1, y1, cx, cy, x2, y2, segments);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_BEZIER_CUBIC,
        |_caller: Caller<'_, ()>,
         x1: i32,
         y1: i32,
         cx1: i32,
         cy1: i32,
         cx2: i32,
         cy2: i32,
         x2: i32,
         y2: i32,
         segments: u32| {
            av::graphics_bezier_cubic(x1, y1, cx1, cy1, cx2, cy2, x2, y2, segments);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PILL,
        |_caller: Caller<'_, ()>, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_pill(x, y, w, h);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_PILL_OUTLINE,
        |_caller: Caller<'_, ()>, x: i32, y: i32, w: u32, h: u32| {
            av::graphics_pill_outline(x, y, w, h);
        },
    )?;

    // 3D
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_SET_3D,
        |_caller: Caller<'_, ()>, enable: u32| {
            av::graphics_set_3d(enable != 0);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_CAMERA_LOOK_AT,
        |_caller: Caller<'_, ()>,
         eye_x: f32,
         eye_y: f32,
         eye_z: f32,
         target_x: f32,
         target_y: f32,
         target_z: f32,
         up_x: f32,
         up_y: f32,
         up_z: f32| {
            av::graphics_camera_look_at(
                eye_x, eye_y, eye_z, target_x, target_y, target_z, up_x, up_y, up_z,
            );
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_CAMERA_PERSPECTIVE,
        |_caller: Caller<'_, ()>, fovy: f32, aspect: f32, near: f32, far: f32| {
            av::graphics_camera_perspective(fovy, aspect, near, far);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_MESH_CREATE,
        |mut caller: Caller<'_, ()>,
         key: u64,
         v_ptr: u32,
         v_len: u32,
         i_ptr: u32,
         i_len: u32|
         -> u32 { av::graphics_mesh_create(&mut caller, key, v_ptr, v_len, i_ptr, i_len) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_MESH_CREATE_OBJ,
        |mut caller: Caller<'_, ()>, key: u64, ptr: u32, len: u32| -> u32 {
            av::graphics_mesh_create_obj(&mut caller, key, ptr, len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_MESH_CREATE_STL,
        |mut caller: Caller<'_, ()>, key: u64, ptr: u32, len: u32| -> u32 {
            av::graphics_mesh_create_stl(&mut caller, key, ptr, len)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_MESH_SET_TEXTURE,
        |_caller: Caller<'_, ()>, mesh_key: u64, image_key: u64| -> u32 {
            av::graphics_mesh_set_texture(mesh_key, image_key)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_MESH_DRAW,
        |_caller: Caller<'_, ()>,
         key: u64,
         x: f32,
         y: f32,
         z: f32,
         rx: f32,
         ry: f32,
         rz: f32,
         sx: f32,
         sy: f32,
         sz: f32| {
            av::graphics_mesh_draw(key, x, y, z, rx, ry, rz, sx, sy, sz);
        },
    )?;

    // Materials / textures (OBJ+MTL workflows)
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::GRAPHICS_MTL_REGISTER_TEXTURE,
        |mut caller: Caller<'_, ()>,
         texture_key: u64,
         mtl_ptr: u32,
         mtl_len: u32,
         tex_filename_ptr: u32,
         tex_filename_len: u32,
         tex_ptr: u32,
         tex_len: u32|
         -> u32 {
            av::graphics_mtl_register_texture(
                &mut caller,
                texture_key,
                mtl_ptr,
                mtl_len,
                tex_filename_ptr,
                tex_filename_len,
                tex_ptr,
                tex_len,
            )
        },
    )?;

    // --- Input ---
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::INPUT_IS_BUTTON_DOWN,
        |_caller: Caller<'_, ()>, port: u32, btn: u32| -> u32 {
            input::joypad_button_pressed(port, btn)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::INPUT_IS_KEY_DOWN,
        |_caller: Caller<'_, ()>, key: u32| -> u32 { input::key_pressed(key) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::INPUT_GET_MOUSE_X,
        |_caller: Caller<'_, ()>| -> i32 { input::mouse_x() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::INPUT_GET_MOUSE_Y,
        |_caller: Caller<'_, ()>| -> i32 { input::mouse_y() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::INPUT_IS_MOUSE_DOWN,
        |_caller: Caller<'_, ()>, btn: u32| -> u32 {
            let mask = input::mouse_buttons();
            let requested = 1u32 << btn;
            if (mask & requested) != 0 { 1 } else { 0 }
        },
    )?;

    // --- Audio ---
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::AUDIO_INIT,
        |_caller: Caller<'_, ()>, sample_rate: u32| -> u32 { av::audio_init(sample_rate) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::AUDIO_PUSH_SAMPLES,
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let _ = av::audio_push_samples(&mut caller, ptr, len);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::AUDIO_PLAY_WAV,
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            av::audio_play_wav(&mut caller, ptr, len);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::AUDIO_PLAY_QOA,
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            av::audio_play_qoa(&mut caller, ptr, len);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::AUDIO_PLAY_XM,
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            av::audio_play_xm(&mut caller, ptr, len);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::AUDIO_PLAY_MIDI,
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            av::audio_play_midi(&mut caller, ptr, len);
        },
    )?;

    // --- System ---
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_LOG,
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let memory = caller.get_export("memory").and_then(|e| e.into_memory());
            let Some(memory) = memory else {
                return;
            };

            let mut buf = vec![0u8; len as usize];
            if memory.read(&caller, ptr as usize, &mut buf).is_ok()
                && let Ok(msg) = core::str::from_utf8(&buf)
            {
                println!("[wasm96] {msg}");
            }
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_MILLIS,
        |_caller: Caller<'_, ()>| -> u64 { crate::av::utils::system_millis() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_DAY,
        |_caller: Caller<'_, ()>| -> u32 { av::system_day() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_HOUR,
        |_caller: Caller<'_, ()>| -> u32 { av::system_hour() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_MINUTE,
        |_caller: Caller<'_, ()>| -> u32 { av::system_minute() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_MONTH,
        |_caller: Caller<'_, ()>| -> u32 { av::system_month() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_SECOND,
        |_caller: Caller<'_, ()>| -> u32 { av::system_second() },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::SYSTEM_YEAR,
        |_caller: Caller<'_, ()>| -> u32 { av::system_year() },
    )?;

    // --- Math ---
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_ABS,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_abs(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_CEIL,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_ceil(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_CONSTRAIN,
        |_caller: Caller<'_, ()>, n: f32, low: f32, high: f32| -> f32 {
            av::math_constrain(n, low, high)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_DIST,
        |_caller: Caller<'_, ()>, x1: f32, y1: f32, x2: f32, y2: f32| -> f32 {
            av::math_dist(x1, y1, x2, y2)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_EXP,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_exp(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_FLOOR,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_floor(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_FRACT,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_fract(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_LERP,
        |_caller: Caller<'_, ()>, start: f32, stop: f32, amt: f32| -> f32 {
            av::math_lerp(start, stop, amt)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_LOG,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_log(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_MAG,
        |_caller: Caller<'_, ()>, x: f32, y: f32| -> f32 { av::math_mag(x, y) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_MAP,
        |_caller: Caller<'_, ()>,
         value: f32,
         start1: f32,
         stop1: f32,
         start2: f32,
         stop2: f32|
         -> f32 { av::math_map(value, start1, stop1, start2, stop2) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_MAX,
        |_caller: Caller<'_, ()>, a: f32, b: f32| -> f32 { av::math_max(a, b) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_MIN,
        |_caller: Caller<'_, ()>, a: f32, b: f32| -> f32 { av::math_min(a, b) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_NORM,
        |_caller: Caller<'_, ()>, value: f32, start: f32, stop: f32| -> f32 {
            av::math_norm(value, start, stop)
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_POW,
        |_caller: Caller<'_, ()>, n: f32, e: f32| -> f32 { av::math_pow(n, e) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_ROUND,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_round(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_SQ,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_sq(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_SQRT,
        |_caller: Caller<'_, ()>, n: f32| -> f32 { av::math_sqrt(n) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_ACOS,
        |_caller: Caller<'_, ()>, value: f32| -> f32 { av::math_acos(value) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_ASIN,
        |_caller: Caller<'_, ()>, value: f32| -> f32 { av::math_asin(value) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_ATAN,
        |_caller: Caller<'_, ()>, value: f32| -> f32 { av::math_atan(value) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_ATAN2,
        |_caller: Caller<'_, ()>, y: f32, x: f32| -> f32 { av::math_atan2(y, x) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_COS,
        |_caller: Caller<'_, ()>, angle: f32| -> f32 { av::math_cos(angle) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_SIN,
        |_caller: Caller<'_, ()>, angle: f32| -> f32 { av::math_sin(angle) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_TAN,
        |_caller: Caller<'_, ()>, angle: f32| -> f32 { av::math_tan(angle) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_DEGREES,
        |_caller: Caller<'_, ()>, radians: f32| -> f32 { av::math_degrees(radians) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_RADIANS,
        |_caller: Caller<'_, ()>, degrees: f32| -> f32 { av::math_radians(degrees) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_RANDOM,
        |_caller: Caller<'_, ()>, min: f32, max: f32| -> f32 { av::math_random(min, max) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_RANDOM_SEED,
        |_caller: Caller<'_, ()>, seed: u32| {
            av::math_random_seed(seed);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_RANDOM_GAUSSIAN,
        |_caller: Caller<'_, ()>, mean: f32, sd: f32| -> f32 { av::math_random_gaussian(mean, sd) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_NOISE,
        |_caller: Caller<'_, ()>, x: f32, y: f32, z: f32| -> f32 { av::math_noise(x, y, z) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_NOISE_SEED,
        |_caller: Caller<'_, ()>, seed: u32| {
            av::math_noise_seed(seed);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::MATH_NOISE_DETAIL,
        |_caller: Caller<'_, ()>, lod: u32, falloff: f32| {
            av::math_noise_detail(lod, falloff);
        },
    )?;

    // --- Storage ---
    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::STORAGE_SAVE,
        |mut caller: Caller<'_, ()>, key: u64, data_ptr: u32, data_len: u32| {
            av::storage_save(&mut caller, key, data_ptr, data_len);
        },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::STORAGE_LOAD,
        |mut caller: Caller<'_, ()>, key: u64| -> u64 { av::storage_load(&mut caller, key) },
    )?;

    linker.func_wrap(
        IMPORT_MODULE,
        host_imports::STORAGE_FREE,
        |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            av::storage_free(&mut caller, ptr, len);
        },
    )?;

    Ok(())
}

/// Define all host imports expected by guests under module `"env"` for the Web runtime.
#[cfg(target_arch = "wasm32")]
pub fn define_web_imports(imports: &js_sys::Object) -> anyhow::Result<()> {
    let env = js_sys::Object::new();

    macro_rules! reg {
        ($name:expr, $func:expr) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 2) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 3) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 4) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 5) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 6) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _, _, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, RET) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut() -> _>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 1, RET) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_) -> _>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 2, RET) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _) -> _>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 3, RET) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _) -> _>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 4, RET) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _, _) -> _>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 8) => {{
            let closure = Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _, _, _, _, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 9) => {{
            let closure =
                Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _, _, _, _, _, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 10) => {{
            let closure =
                Closure::wrap(Box::new($func) as Box<dyn FnMut(_, _, _, _, _, _, _, _, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
        ($name:expr, $func:expr, 16) => {{
            let closure =
                Closure::wrap(Box::new($func)
                    as Box<dyn FnMut(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _)>);
            js_sys::Reflect::set(&env, &$name.into(), closure.as_ref().unchecked_ref())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            closure.forget();
        }};
    }

    // --- Graphics ---
    reg!(
        host_imports::GRAPHICS_SET_SIZE,
        |w, h| av::graphics_set_size(w, h),
        2
    );
    reg!(
        host_imports::GRAPHICS_APPLY_MATRIX,
        |m00, m01, m02, m03, m10, m11, m12, m13, m20, m21, m22, m23, m30, m31, m32, m33| {
            av::graphics_apply_matrix(
                m00, m01, m02, m03, m10, m11, m12, m13, m20, m21, m22, m23, m30, m31, m32, m33,
            );
        },
        16
    );
    reg!(
        host_imports::GRAPHICS_RESET_MATRIX,
        || av::graphics_reset_matrix(),
        RET
    );
    reg!(
        host_imports::GRAPHICS_ROTATE,
        |angle| av::graphics_rotate(angle),
        1
    );
    reg!(
        host_imports::GRAPHICS_ROTATE_X,
        |angle| av::graphics_rotate_x(angle),
        1
    );
    reg!(
        host_imports::GRAPHICS_ROTATE_Y,
        |angle| av::graphics_rotate_y(angle),
        1
    );
    reg!(
        host_imports::GRAPHICS_ROTATE_Z,
        |angle| av::graphics_rotate_z(angle),
        1
    );
    reg!(
        host_imports::GRAPHICS_SCALE,
        |sx, sy, sz| av::graphics_scale(sx, sy, sz),
        3
    );
    reg!(
        host_imports::GRAPHICS_SHEAR_X,
        |angle| av::graphics_shear_x(angle),
        1
    );
    reg!(
        host_imports::GRAPHICS_SHEAR_Y,
        |angle| av::graphics_shear_y(angle),
        1
    );
    reg!(
        host_imports::GRAPHICS_TRANSLATE,
        |x, y, z| av::graphics_translate(x, y, z),
        3
    );
    reg!(
        host_imports::GRAPHICS_PUSH_MATRIX,
        || av::graphics_push_matrix(),
        RET
    );
    reg!(
        host_imports::GRAPHICS_POP_MATRIX,
        || av::graphics_pop_matrix(),
        RET
    );
    reg!(
        host_imports::GRAPHICS_SET_COLOR,
        |r, g, b, a| av::graphics_set_color(r, g, b, a),
        4
    );
    reg!(
        host_imports::GRAPHICS_BACKGROUND,
        |r, g, b| av::graphics_background(r, g, b),
        3
    );
    reg!(
        host_imports::GRAPHICS_POINT,
        |x, y| av::graphics_point(x, y),
        2
    );
    reg!(
        host_imports::GRAPHICS_LINE,
        |x1, y1, x2, y2| av::graphics_line(x1, y1, x2, y2),
        4
    );
    reg!(
        host_imports::GRAPHICS_RECT,
        |x, y, w, h| av::graphics_rect(x, y, w, h),
        4
    );
    reg!(
        host_imports::GRAPHICS_RECT_OUTLINE,
        |x, y, w, h| av::graphics_rect_outline(x, y, w, h),
        4
    );
    reg!(
        host_imports::GRAPHICS_CIRCLE,
        |x, y, r| av::graphics_circle(x, y, r),
        3
    );
    reg!(
        host_imports::GRAPHICS_CIRCLE_OUTLINE,
        |x, y, r| av::graphics_circle_outline(x, y, r),
        3
    );
    reg!(
        host_imports::GRAPHICS_ELLIPSE,
        |cx, cy, w, h| av::graphics_ellipse(cx, cy, w, h),
        4
    );
    reg!(
        host_imports::GRAPHICS_ARC,
        |cx, cy, w, h, start, end| av::graphics_arc(cx, cy, w, h, start, end),
        6
    );
    reg!(
        host_imports::GRAPHICS_QUAD,
        |x1, y1, x2, y2, x3, y3, x4, y4| av::graphics_quad(x1, y1, x2, y2, x3, y3, x4, y4),
        8
    );
    reg!(
        host_imports::GRAPHICS_TRIANGLE,
        |x1, y1, x2, y2, x3, y3| av::graphics_triangle(x1, y1, x2, y2, x3, y3),
        6
    );
    reg!(
        host_imports::GRAPHICS_TRIANGLE_OUTLINE,
        |x1, y1, x2, y2, x3, y3| av::graphics_triangle_outline(x1, y1, x2, y2, x3, y3),
        6
    );
    reg!(
        host_imports::GRAPHICS_PILL,
        |x, y, w, h| av::graphics_pill(x, y, w, h),
        4
    );
    reg!(
        host_imports::GRAPHICS_PILL_OUTLINE,
        |x, y, w, h| av::graphics_pill_outline(x, y, w, h),
        4
    );

    reg!(
        host_imports::GRAPHICS_IMAGE,
        |x, y, w, h, ptr, len| {
            if let Ok(data) = av::utils::read_guest_bytes(ptr, len) {
                av::utils::graphics_image_from_host(x, y, w, h, &data);
            }
        },
        6
    );

    // --- 3D Stubs (not yet supported on Web) ---
    reg!(host_imports::GRAPHICS_SET_3D, |_enable| { /* stub */ }, 1);
    reg!(
        host_imports::GRAPHICS_CAMERA_LOOK_AT,
        |_ex, _ey, _ez, _tx, _ty, _tz, _ux, _uy, _uz| { /* stub */ },
        9
    );
    reg!(
        host_imports::GRAPHICS_CAMERA_PERSPECTIVE,
        |_fovy, _aspect, _near, _far| { /* stub */ },
        4
    );
    reg!(
        host_imports::GRAPHICS_MESH_CREATE,
        |_key, _vptr, _vlen, _iptr, _ilen| 0u32,
        5,
        RET
    );
    reg!(
        host_imports::GRAPHICS_MESH_CREATE_OBJ,
        |_key, _ptr, _len| 0u32,
        3,
        RET
    );
    reg!(
        host_imports::GRAPHICS_MESH_CREATE_STL,
        |_key, _ptr, _len| 0u32,
        3,
        RET
    );
    reg!(
        host_imports::GRAPHICS_MESH_SET_TEXTURE,
        |_mkey, _ikey| 0u32,
        2,
        RET
    );
    reg!(
        host_imports::GRAPHICS_MESH_DRAW,
        |_key, _x, _y, _z, _rx, _ry, _rz, _sx, _sy, _sz| { /* stub */ },
        10
    );

    reg!(host_imports::GRAPHICS_CLEAR, || av::graphics_clear(), RET);
    reg!(
        host_imports::GRAPHICS_FILL,
        |r, g, b, a| av::graphics_fill(r, g, b, a),
        4
    );
    reg!(
        host_imports::GRAPHICS_NO_FILL,
        || av::graphics_no_fill(),
        RET
    );
    reg!(
        host_imports::GRAPHICS_STROKE,
        |r, g, b, a| av::graphics_stroke(r, g, b, a),
        4
    );
    reg!(
        host_imports::GRAPHICS_NO_STROKE,
        || av::graphics_no_stroke(),
        RET
    );

    reg!(
        host_imports::GRAPHICS_FONT_REGISTER_SPLEEN,
        |key, size| -> u32 { av::graphics_font_register_spleen(key, size) },
        2,
        RET
    );
    reg!(
        host_imports::GRAPHICS_TEXT_KEY,
        |x, y, font_key, ptr, len| {
            // wasm32/web: text is read from the globally-registered WebAssembly memory.
            av::graphics_text_key(x, y, font_key, ptr, len);
        },
        5
    );

    // --- Input ---
    reg!(
        host_imports::INPUT_IS_BUTTON_DOWN,
        |p, b| -> u32 { input::joypad_button_pressed(p, b) as u32 },
        2,
        RET
    );
    reg!(
        host_imports::INPUT_IS_KEY_DOWN,
        |k| -> u32 { input::key_pressed(k) as u32 },
        1,
        RET
    );
    reg!(host_imports::INPUT_GET_MOUSE_X, || input::mouse_x(), RET);
    reg!(host_imports::INPUT_GET_MOUSE_Y, || input::mouse_y(), RET);
    reg!(
        host_imports::INPUT_IS_MOUSE_DOWN,
        |b: u32| -> u32 {
            let mask: u32 = input::mouse_buttons();
            let requested: u32 = 1u32 << b;
            if (mask & requested) != 0 { 1 } else { 0 }
        },
        1,
        RET
    );

    // --- Audio ---
    reg!(
        host_imports::AUDIO_INIT,
        |sr| -> u32 { av::audio_init(sr) },
        1,
        RET
    );
    reg!(
        host_imports::AUDIO_PLAY_WAV,
        |ptr, len| {
            // wasm32/web: `audio_play_wav` reads bytes from the globally-registered WebAssembly memory.
            av::audio_play_wav(ptr, len);
        },
        2
    );

    // --- System ---
    reg!(
        host_imports::SYSTEM_LOG,
        |ptr, len| {
            if let Ok(msg) = read_guest_string_web(ptr, len) {
                web_sys::console::log_1(&msg.into());
            }
        },
        2
    );
    reg!(
        host_imports::SYSTEM_MILLIS,
        || av::utils::system_millis(),
        RET
    );

    // --- Math ---
    reg!(
        host_imports::MATH_RANDOM,
        |min, max| av::math_random(min, max),
        2,
        RET
    );
    reg!(
        host_imports::MATH_NOISE,
        |x, y, z| av::math_noise(x, y, z),
        3,
        RET
    );

    // --- Storage ---
    reg!(
        host_imports::STORAGE_SAVE,
        |key, ptr, len| {
            if let Ok(data) = av::utils::read_guest_bytes(ptr, len) {
                av::storage_save_raw(key, &data);
            }
        },
        3
    );
    reg!(
        host_imports::STORAGE_LOAD,
        |key| -> u64 { av::storage_load_raw(key) },
        1,
        RET
    );

    js_sys::Reflect::set(imports, &IMPORT_MODULE.into(), &env.into())
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    Ok(())
}
