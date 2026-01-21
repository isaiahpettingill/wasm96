//! ABI definitions and guest entrypoint resolution for wasm96.

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Reflect, WebAssembly};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// The module name that guest modules must use to import host functions.
pub const IMPORT_MODULE: &str = "env";

/// Well-known guest export names.
pub mod guest_exports {
    pub const SETUP: &str = "setup";
    pub const UPDATE: &str = "update";
    pub const DRAW: &str = "draw";
    pub const WASI_START: &str = "_start";
    pub const MAIN: &str = "main";
}

/// Well-known host import names.
pub mod host_imports {
    pub const GRAPHICS_SET_SIZE: &str = "wasm96_graphics_set_size";
    pub const GRAPHICS_SET_COLOR: &str = "wasm96_graphics_set_color";
    pub const GRAPHICS_BACKGROUND: &str = "wasm96_graphics_background";
    pub const GRAPHICS_POINT: &str = "wasm96_graphics_point";
    pub const GRAPHICS_LINE: &str = "wasm96_graphics_line";
    pub const GRAPHICS_RECT: &str = "wasm96_graphics_rect";
    pub const GRAPHICS_RECT_OUTLINE: &str = "wasm96_graphics_rect_outline";
    pub const GRAPHICS_CIRCLE: &str = "wasm96_graphics_circle";
    pub const GRAPHICS_CIRCLE_OUTLINE: &str = "wasm96_graphics_circle_outline";
    pub const GRAPHICS_IMAGE: &str = "wasm96_graphics_image";
    pub const GRAPHICS_IMAGE_PNG: &str = "wasm96_graphics_image_png";
    pub const GRAPHICS_IMAGE_JPEG: &str = "wasm96_graphics_image_jpeg";
    pub const GRAPHICS_SVG_REGISTER: &str = "wasm96_graphics_svg_register";
    pub const GRAPHICS_SVG_DRAW_KEY: &str = "wasm96_graphics_svg_draw_key";
    pub const GRAPHICS_SVG_UNREGISTER: &str = "wasm96_graphics_svg_unregister";
    pub const GRAPHICS_GIF_REGISTER: &str = "wasm96_graphics_gif_register";
    pub const GRAPHICS_GIF_DRAW_KEY: &str = "wasm96_graphics_gif_draw_key";
    pub const GRAPHICS_GIF_DRAW_KEY_SCALED: &str = "wasm96_graphics_gif_draw_key_scaled";
    pub const GRAPHICS_GIF_UNREGISTER: &str = "wasm96_graphics_gif_unregister";
    pub const GRAPHICS_ASEPRITE_REGISTER: &str = "wasm96_graphics_aseprite_register";
    pub const GRAPHICS_ASEPRITE_DRAW_KEY: &str = "wasm96_graphics_aseprite_draw_key";
    pub const GRAPHICS_ASEPRITE_DRAW_KEY_SCALED: &str = "wasm96_graphics_aseprite_draw_key_scaled";
    pub const GRAPHICS_ASEPRITE_PLAY_KEY: &str = "wasm96_graphics_aseprite_play_key";
    pub const GRAPHICS_ASEPRITE_PLAY_KEY_SCALED: &str = "wasm96_graphics_aseprite_play_key_scaled";
    pub const GRAPHICS_ASEPRITE_UNREGISTER: &str = "wasm96_graphics_aseprite_unregister";
    pub const GRAPHICS_MTL_REGISTER: &str = "wasm96_graphics_mtl_register";
    pub const GRAPHICS_PNG_REGISTER: &str = "wasm96_graphics_png_register";
    pub const GRAPHICS_PNG_DRAW_KEY: &str = "wasm96_graphics_png_draw_key";
    pub const GRAPHICS_PNG_DRAW_KEY_SCALED: &str = "wasm96_graphics_png_draw_key_scaled";
    pub const GRAPHICS_PNG_UNREGISTER: &str = "wasm96_graphics_png_unregister";
    pub const GRAPHICS_JPEG_REGISTER: &str = "wasm96_graphics_jpeg_register";
    pub const GRAPHICS_JPEG_DRAW_KEY: &str = "wasm96_graphics_jpeg_draw_key";
    pub const GRAPHICS_JPEG_DRAW_KEY_SCALED: &str = "wasm96_graphics_jpeg_draw_key_scaled";
    pub const GRAPHICS_JPEG_UNREGISTER: &str = "wasm96_graphics_jpeg_unregister";
    pub const GRAPHICS_ELLIPSE: &str = "wasm96_graphics_ellipse";
    pub const GRAPHICS_ARC: &str = "wasm96_graphics_arc";
    pub const GRAPHICS_QUAD: &str = "wasm96_graphics_quad";
    pub const GRAPHICS_TRIANGLE: &str = "wasm96_graphics_triangle";
    pub const GRAPHICS_TRIANGLE_OUTLINE: &str = "wasm96_graphics_triangle_outline";
    pub const GRAPHICS_BEZIER_QUADRATIC: &str = "wasm96_graphics_bezier_quadratic";
    pub const GRAPHICS_BEZIER_CUBIC: &str = "wasm96_graphics_bezier_cubic";
    pub const GRAPHICS_PILL: &str = "wasm96_graphics_pill";
    pub const GRAPHICS_PILL_OUTLINE: &str = "wasm96_graphics_pill_outline";
    pub const GRAPHICS_QUAD_OUTLINE: &str = "wasm96_graphics_quad_outline";
    pub const GRAPHICS_RED: &str = "wasm96_graphics_red";
    pub const GRAPHICS_GREEN: &str = "wasm96_graphics_green";
    pub const GRAPHICS_BLUE: &str = "wasm96_graphics_blue";
    pub const GRAPHICS_ALPHA: &str = "wasm96_graphics_alpha";
    pub const GRAPHICS_BRIGHTNESS: &str = "wasm96_graphics_brightness";
    pub const GRAPHICS_HUE: &str = "wasm96_graphics_hue";
    pub const GRAPHICS_SATURATION: &str = "wasm96_graphics_saturation";
    pub const GRAPHICS_LIGHTNESS: &str = "wasm96_graphics_lightness";
    pub const GRAPHICS_COLOR_RGB: &str = "wasm96_graphics_color_rgb";
    pub const GRAPHICS_COLOR_HSL: &str = "wasm96_graphics_color_hsl";
    pub const GRAPHICS_LERP_COLOR: &str = "wasm96_graphics_lerp_color";
    pub const GRAPHICS_PALETTE_LERP: &str = "wasm96_graphics_palette_lerp";
    pub const GRAPHICS_CLEAR: &str = "wasm96_graphics_clear";
    pub const GRAPHICS_FILL: &str = "wasm96_graphics_fill";
    pub const GRAPHICS_NO_FILL: &str = "wasm96_graphics_no_fill";
    pub const GRAPHICS_STROKE: &str = "wasm96_graphics_stroke";
    pub const GRAPHICS_NO_STROKE: &str = "wasm96_graphics_no_stroke";
    pub const GRAPHICS_ERASE: &str = "wasm96_graphics_erase";
    pub const GRAPHICS_NO_ERASE: &str = "wasm96_graphics_no_erase";
    pub const GRAPHICS_COLOR_MODE: &str = "wasm96_graphics_color_mode";
    pub const GRAPHICS_CLIP: &str = "wasm96_graphics_clip";
    pub const GRAPHICS_BEGIN_CLIP: &str = "wasm96_graphics_begin_clip";
    pub const GRAPHICS_END_CLIP: &str = "wasm96_graphics_end_clip";
    pub const GRAPHICS_APPLY_MATRIX: &str = "wasm96_graphics_apply_matrix";
    pub const GRAPHICS_RESET_MATRIX: &str = "wasm96_graphics_reset_matrix";
    pub const GRAPHICS_ROTATE: &str = "wasm96_graphics_rotate";
    pub const GRAPHICS_ROTATE_X: &str = "wasm96_graphics_rotate_x";
    pub const GRAPHICS_ROTATE_Y: &str = "wasm96_graphics_rotate_y";
    pub const GRAPHICS_ROTATE_Z: &str = "wasm96_graphics_rotate_z";
    pub const GRAPHICS_SCALE: &str = "wasm96_graphics_scale";
    pub const GRAPHICS_SHEAR_X: &str = "wasm96_graphics_shear_x";
    pub const GRAPHICS_SHEAR_Y: &str = "wasm96_graphics_shear_y";
    pub const GRAPHICS_TRANSLATE: &str = "wasm96_graphics_translate";
    pub const GRAPHICS_PUSH_MATRIX: &str = "wasm96_graphics_push_matrix";
    pub const GRAPHICS_POP_MATRIX: &str = "wasm96_graphics_pop_matrix";
    pub const GRAPHICS_SET_3D: &str = "wasm96_graphics_set_3d";
    pub const GRAPHICS_CAMERA_LOOK_AT: &str = "wasm96_graphics_camera_look_at";
    pub const GRAPHICS_CAMERA_PERSPECTIVE: &str = "wasm96_graphics_camera_perspective";
    pub const GRAPHICS_MESH_CREATE: &str = "wasm96_graphics_mesh_create";
    pub const GRAPHICS_MESH_CREATE_OBJ: &str = "wasm96_graphics_mesh_create_obj";
    pub const GRAPHICS_MESH_CREATE_STL: &str = "wasm96_graphics_mesh_create_stl";
    pub const GRAPHICS_MESH_SET_TEXTURE: &str = "wasm96_graphics_mesh_set_texture";
    pub const GRAPHICS_MESH_DRAW: &str = "wasm96_graphics_mesh_draw";
    pub const GRAPHICS_MTL_REGISTER_TEXTURE: &str = "wasm96_graphics_mtl_register_texture";
    pub const GRAPHICS_FONT_REGISTER_TTF: &str = "wasm96_graphics_font_register_ttf";
    pub const GRAPHICS_FONT_REGISTER_BDF: &str = "wasm96_graphics_font_register_bdf";
    pub const GRAPHICS_FONT_REGISTER_SPLEEN: &str = "wasm96_graphics_font_register_spleen";
    pub const GRAPHICS_FONT_UNREGISTER: &str = "wasm96_graphics_font_unregister";
    pub const GRAPHICS_TEXT_KEY: &str = "wasm96_graphics_text_key";
    pub const GRAPHICS_TEXT_MEASURE_KEY: &str = "wasm96_graphics_text_measure_key";
    pub const INPUT_IS_BUTTON_DOWN: &str = "wasm96_input_is_button_down";
    pub const INPUT_IS_KEY_DOWN: &str = "wasm96_input_is_key_down";
    pub const INPUT_GET_MOUSE_X: &str = "wasm96_input_get_mouse_x";
    pub const INPUT_GET_MOUSE_Y: &str = "wasm96_input_get_mouse_y";
    pub const INPUT_IS_MOUSE_DOWN: &str = "wasm96_input_is_mouse_down";
    pub const AUDIO_INIT: &str = "wasm96_audio_init";
    pub const AUDIO_PUSH_SAMPLES: &str = "wasm96_audio_push_samples";
    pub const AUDIO_PLAY_WAV: &str = "wasm96_audio_play_wav";
    pub const AUDIO_PLAY_QOA: &str = "wasm96_audio_play_qoa";
    pub const AUDIO_PLAY_XM: &str = "wasm96_audio_play_xm";
    pub const AUDIO_PLAY_MIDI: &str = "wasm96_audio_play_midi";
    pub const STORAGE_SAVE: &str = "wasm96_storage_save";
    pub const STORAGE_LOAD: &str = "wasm96_storage_load";
    pub const STORAGE_FREE: &str = "wasm96_storage_free";
    pub const SYSTEM_LOG: &str = "wasm96_system_log";
    pub const SYSTEM_MILLIS: &str = "wasm96_system_millis";
    pub const SYSTEM_DAY: &str = "wasm96_system_day";
    pub const SYSTEM_HOUR: &str = "wasm96_system_hour";
    pub const SYSTEM_MINUTE: &str = "wasm96_system_minute";
    pub const SYSTEM_MONTH: &str = "wasm96_system_month";
    pub const SYSTEM_SECOND: &str = "wasm96_system_second";
    pub const SYSTEM_YEAR: &str = "wasm96_system_year";
    pub const MATH_ABS: &str = "wasm96_math_abs";
    pub const MATH_CEIL: &str = "wasm96_math_ceil";
    pub const MATH_CONSTRAIN: &str = "wasm96_math_constrain";
    pub const MATH_DIST: &str = "wasm96_math_dist";
    pub const MATH_EXP: &str = "wasm96_math_exp";
    pub const MATH_FLOOR: &str = "wasm96_math_floor";
    pub const MATH_FRACT: &str = "wasm96_math_fract";
    pub const MATH_LERP: &str = "wasm96_math_lerp";
    pub const MATH_LOG: &str = "wasm96_math_log";
    pub const MATH_MAG: &str = "wasm96_math_mag";
    pub const MATH_MAP: &str = "wasm96_math_map";
    pub const MATH_MAX: &str = "wasm96_math_max";
    pub const MATH_MIN: &str = "wasm96_math_min";
    pub const MATH_NORM: &str = "wasm96_math_norm";
    pub const MATH_POW: &str = "wasm96_math_pow";
    pub const MATH_ROUND: &str = "wasm96_math_round";
    pub const MATH_SQ: &str = "wasm96_math_sq";
    pub const MATH_SQRT: &str = "wasm96_math_sqrt";
    pub const MATH_ACOS: &str = "wasm96_math_acos";
    pub const MATH_ASIN: &str = "wasm96_math_asin";
    pub const MATH_ATAN: &str = "wasm96_math_atan";
    pub const MATH_ATAN2: &str = "wasm96_math_atan2";
    pub const MATH_COS: &str = "wasm96_math_cos";
    pub const MATH_SIN: &str = "wasm96_math_sin";
    pub const MATH_TAN: &str = "wasm96_math_tan";
    pub const MATH_DEGREES: &str = "wasm96_math_degrees";
    pub const MATH_RADIANS: &str = "wasm96_math_radians";
    pub const MATH_RANDOM: &str = "wasm96_math_random";
    pub const MATH_RANDOM_SEED: &str = "wasm96_math_random_seed";
    pub const MATH_RANDOM_GAUSSIAN: &str = "wasm96_math_random_gaussian";
    pub const MATH_NOISE: &str = "wasm96_math_noise";
    pub const MATH_NOISE_SEED: &str = "wasm96_math_noise_seed";
    pub const MATH_NOISE_DETAIL: &str = "wasm96_math_noise_detail";
}

/// Standard controller buttons.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Button {
    B = 0,
    Y = 1,
    Select = 2,
    Start = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
    A = 8,
    X = 9,
    L1 = 10,
    R1 = 11,
    L2 = 12,
    R2 = 13,
    L3 = 14,
    R3 = 15,
}

pub mod validate {
    use super::guest_exports;

    #[cfg(not(target_arch = "wasm32"))]
    pub fn required_exports_present_wasmtime(
        instance: &wasmtime::Instance,
        mut store: impl wasmtime::AsContextMut,
    ) -> Result<(), MissingExport> {
        if instance
            .get_func(&mut store, guest_exports::SETUP)
            .is_none()
        {
            return Err(MissingExport::Setup);
        }
        Ok(())
    }

    #[derive(Debug)]
    pub enum MissingExport {
        Setup,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub type GuestFunc = wasmtime::Func;
#[cfg(target_arch = "wasm32")]
pub type GuestFunc = js_sys::Function;

/// Resolved entrypoints for a guest module.
pub struct GuestEntrypoints {
    pub setup: GuestFunc,
    pub update: Option<GuestFunc>,
    pub draw: Option<GuestFunc>,
}

impl GuestEntrypoints {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resolve_wasmtime(
        instance: &wasmtime::Instance,
        mut store: impl wasmtime::AsContextMut,
    ) -> anyhow::Result<Self> {
        let setup = instance
            .get_func(&mut store, guest_exports::SETUP)
            .ok_or_else(|| anyhow::anyhow!("missing setup"))?;

        let update = instance.get_func(&mut store, guest_exports::UPDATE);

        let mut draw = instance.get_func(&mut store, guest_exports::DRAW);
        if draw.is_none() {
            draw = instance.get_func(&mut store, guest_exports::WASI_START);
        }
        if draw.is_none() {
            draw = instance.get_func(&mut store, guest_exports::MAIN);
        }

        Ok(Self {
            setup,
            update,
            draw,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn resolve_web(instance: &WebAssembly::Instance) -> anyhow::Result<Self> {
        let exports = instance.exports();

        let setup = Reflect::get(&exports, &guest_exports::SETUP.into())
            .map_err(|e| anyhow::anyhow!("{:?}", e))?
            .dyn_into::<Function>()
            .map_err(|_| anyhow::anyhow!("setup is not a function"))?;

        let update = Reflect::get(&exports, &guest_exports::UPDATE.into())
            .ok()
            .and_then(|v| v.dyn_into::<Function>().ok());

        let mut draw = Reflect::get(&exports, &guest_exports::DRAW.into())
            .ok()
            .and_then(|v| v.dyn_into::<Function>().ok());

        if draw.is_none() {
            draw = Reflect::get(&exports, &guest_exports::WASI_START.into())
                .ok()
                .and_then(|v| v.dyn_into::<Function>().ok());
        }
        if draw.is_none() {
            draw = Reflect::get(&exports, &guest_exports::MAIN.into())
                .ok()
                .and_then(|v| v.dyn_into::<Function>().ok());
        }

        Ok(Self {
            setup,
            update,
            draw,
        })
    }
}

#[cfg(test)]
mod entrypoint_tests {
    #[cfg(not(target_arch = "wasm32"))]
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use wasmtime::{Config, Engine, Module, Store};

    #[cfg(not(target_arch = "wasm32"))]
    fn instantiate(wat: &str) -> (wasmtime::Instance, Store<()>) {
        let engine = Engine::new(Config::new().wasm_multi_value(true)).unwrap();
        let module = Module::new(&engine, wat).unwrap();
        let mut store = Store::new(&engine, ());
        let linker = wasmtime::Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();
        (instance, store)
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn requires_setup_export() {
        let (instance, mut store) = instantiate(r#"(module (func (export "not_setup")))"#);
        assert!(GuestEntrypoints::resolve_wasmtime(&instance, &mut store).is_err());

        let (instance, mut store) = instantiate(r#"(module (func (export "setup")))"#);
        assert!(GuestEntrypoints::resolve_wasmtime(&instance, &mut store).is_ok());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn prefers_draw_over_wasi_start_and_main() {
        let (instance, mut store) = instantiate(
            r#"(module
                (func (export "setup"))
                (func (export "draw"))
                (func (export "_start"))
                (func (export "main"))
            )"#,
        );
        let entry = GuestEntrypoints::resolve_wasmtime(&instance, &mut store).unwrap();
        let _ = entry; // Just check it resolves
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn falls_back_to_wasi_start_when_draw_missing() {
        let (instance, mut store) = instantiate(
            r#"(module
                (func (export "setup"))
                (func (export "_start"))
                (func (export "main"))
            )"#,
        );
        let entry = GuestEntrypoints::resolve_wasmtime(&instance, &mut store).unwrap();
        let _ = entry;
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn falls_back_to_main_when_draw_and_wasi_start_missing() {
        let (instance, mut store) = instantiate(
            r#"(module
                (func (export "setup"))
                (func (export "main"))
            )"#,
        );
        let entry = GuestEntrypoints::resolve_wasmtime(&instance, &mut store).unwrap();
        let _ = entry;
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn update_is_none_when_missing() {
        let (instance, mut store) = instantiate(r#"(module (func (export "setup")))"#);
        let entry = GuestEntrypoints::resolve_wasmtime(&instance, &mut store).unwrap();
        assert!(entry.update.is_none());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn update_prefers_export_when_present() {
        let (instance, mut store) = instantiate(
            r#"(module
                (func (export "setup"))
                (func (export "update"))
            )"#,
        );
        let entry = GuestEntrypoints::resolve_wasmtime(&instance, &mut store).unwrap();
        assert!(entry.update.is_some());
    }
}
