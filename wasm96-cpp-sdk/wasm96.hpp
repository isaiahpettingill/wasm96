#ifndef WASM96_HPP
#define WASM96_HPP

/*
Freestanding-friendly header:

This C++ SDK is meant to be used by WebAssembly guests that are often built as
`wasm32-freestanding` (no libc++ / no WASI). In that environment, standard C/C++
library headers (like <cstring>) may be unavailable.

- We only require <cstdint>.
- We provide tiny local helpers for string length when libc++ isn't present.

Linking notes:

All `wasm96_*` host calls are imported by the runtime (wasm96 core). To ensure
toolchains treat them as WebAssembly imports (and therefore don't require local
definitions), we declare them with an explicit import module/name when building
for wasm.
*/

/*
Avoid standard library includes.

Many wasm guest toolchains (especially when using Zig without a full libc++) won't
have C++ standard headers like <cstdint>/<cstring> available. We instead define
the minimal fixed-width types ourselves.

Assumptions (true for wasm32 + common embedded targets):
- `unsigned char` is 8-bit
- `unsigned short` is 16-bit
- `unsigned int` is 32-bit
- `unsigned long long` is 64-bit
- `int` is 32-bit
- `float` is IEEE-754 binary32
*/
typedef unsigned char uint8_t;
typedef unsigned short uint16_t;
typedef unsigned int uint32_t;
typedef unsigned long long uint64_t;
typedef signed int int32_t;
typedef signed short int16_t;

#if defined(__wasm__) || defined(__EMSCRIPTEN__) || defined(__wasi__)
  // Tell LLVM/Clang-based toolchains to generate `import` entries in the wasm.
  // (GCC-style attributes are ignored by MSVC, but MSVC doesn't target wasm here.)
  #define WASM96_WASM_IMPORT(module, name) __attribute__((import_module(module), import_name(name)))
#else
  #define WASM96_WASM_IMPORT(module, name)
#endif

// wasm96-core currently defines host imports under module name "env".
#ifndef WASM96_WASM_IMPORT_MODULE
  #define WASM96_WASM_IMPORT_MODULE "env"
#endif

static inline uint32_t wasm96_strlen_(const char* s) {
    uint32_t n = 0;
    if (!s) return 0;
    while (s[n] != '\0') n++;
    return n;
}

extern "C" {

// Joypad button ids.
typedef enum {
    WASM96_BUTTON_B = 0,
    WASM96_BUTTON_Y = 1,
    WASM96_BUTTON_SELECT = 2,
    WASM96_BUTTON_START = 3,
    WASM96_BUTTON_UP = 4,
    WASM96_BUTTON_DOWN = 5,
    WASM96_BUTTON_LEFT = 6,
    WASM96_BUTTON_RIGHT = 7,
    WASM96_BUTTON_A = 8,
    WASM96_BUTTON_X = 9,
    WASM96_BUTTON_L1 = 10,
    WASM96_BUTTON_R1 = 11,
    WASM96_BUTTON_L2 = 12,
    WASM96_BUTTON_R2 = 13,
    WASM96_BUTTON_L3 = 14,
    WASM96_BUTTON_R3 = 15
} wasm96_button_t;

// Text size dimensions.
typedef struct {
    uint32_t width;
    uint32_t height;
} wasm96_text_size_t;

typedef struct {
    float x;
    float y;
} wasm96_vector2_t;

typedef struct {
    float x;
    float y;
    float z;
} wasm96_vector3_t;

// Low-level raw ABI imports.
extern void wasm96_graphics_set_size(uint32_t width, uint32_t height) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_set_size");
extern void wasm96_graphics_set_color(uint32_t r, uint32_t g, uint32_t b, uint32_t a) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_set_color");
extern void wasm96_graphics_background(uint32_t r, uint32_t g, uint32_t b) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_background");
extern void wasm96_graphics_point(int32_t x, int32_t y) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_point");
extern void wasm96_graphics_line(int32_t x1, int32_t y1, int32_t x2, int32_t y2) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_line");
extern void wasm96_graphics_rect(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_rect");
extern void wasm96_graphics_rect_outline(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_rect_outline");
extern void wasm96_graphics_circle(int32_t x, int32_t y, uint32_t r) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_circle");
extern void wasm96_graphics_circle_outline(int32_t x, int32_t y, uint32_t r) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_circle_outline");
extern void wasm96_graphics_image(int32_t x, int32_t y, uint32_t w, uint32_t h, const uint8_t* data, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_image");
extern void wasm96_graphics_image_png(int32_t x, int32_t y, const uint8_t* data, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_image_png");
extern void wasm96_graphics_image_jpeg(int32_t x, int32_t y, const uint8_t* data, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_image_jpeg");
extern void wasm96_graphics_ellipse(int32_t cx, int32_t cy, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_ellipse");
extern void wasm96_graphics_arc(int32_t cx, int32_t cy, uint32_t w, uint32_t h, float start, float end) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_arc");
extern void wasm96_graphics_quad(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3, int32_t x4, int32_t y4) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_quad");
extern void wasm96_graphics_triangle(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_triangle");
extern void wasm96_graphics_triangle_outline(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_triangle_outline");
extern void wasm96_graphics_bezier_quadratic(int32_t x1, int32_t y1, int32_t cx, int32_t cy, int32_t x2, int32_t y2, uint32_t segments) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_bezier_quadratic");
extern void wasm96_graphics_bezier_cubic(int32_t x1, int32_t y1, int32_t cx1, int32_t cy1, int32_t cx2, int32_t cy2, int32_t x2, int32_t y2, uint32_t segments) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_bezier_cubic");
extern void wasm96_graphics_pill(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_pill");
extern void wasm96_graphics_pill_outline(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_pill_outline");

extern void wasm96_graphics_apply_matrix(float m00, float m01, float m02, float m03, float m10, float m11, float m12, float m13, float m20, float m21, float m22, float m23, float m30, float m31, float m32, float m33) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_apply_matrix");
extern void wasm96_graphics_reset_matrix(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_reset_matrix");
extern void wasm96_graphics_rotate(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_rotate");
extern void wasm96_graphics_rotate_x(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_rotate_x");
extern void wasm96_graphics_rotate_y(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_rotate_y");
extern void wasm96_graphics_rotate_z(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_rotate_z");
extern void wasm96_graphics_scale(float sx, float sy, float sz) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_scale");
extern void wasm96_graphics_shear_x(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_shear_x");
extern void wasm96_graphics_shear_y(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_shear_y");
extern void wasm96_graphics_translate(float x, float y, float z) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_translate");
extern void wasm96_graphics_push_matrix(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_push_matrix");
extern void wasm96_graphics_pop_matrix(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_pop_matrix");

// 3D Graphics
extern void wasm96_graphics_set_3d(uint32_t enable) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_set_3d");
extern void wasm96_graphics_camera_look_at(float eye_x, float eye_y, float eye_z, float target_x, float target_y, float target_z, float up_x, float up_y, float up_z) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_camera_look_at");
extern void wasm96_graphics_camera_perspective(float fovy, float aspect, float near, float far) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_camera_perspective");
extern uint32_t wasm96_graphics_mesh_create(uint64_t key, const float* v_ptr, uint32_t v_len, const uint32_t* i_ptr, uint32_t i_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_create");
extern uint32_t wasm96_graphics_mesh_create_obj(uint64_t key, const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_create_obj");
extern uint32_t wasm96_graphics_mesh_create_stl(uint64_t key, const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_create_stl");
extern void wasm96_graphics_mesh_draw(uint64_t key, float x, float y, float z, float rx, float ry, float rz, float sx, float sy, float sz) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_draw");
extern uint32_t wasm96_graphics_mesh_set_texture(uint64_t mesh_key, uint64_t image_key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_set_texture");

// Materials / textures (OBJ+MTL workflows)
// Given an `.mtl` file bytes + one encoded texture blob (PNG/JPEG) + its filename,
// the host will decode and register the texture under `texture_key` *iff* the filename
// appears as a `map_Kd` entry in the provided `.mtl`. Returns 1 on success, 0 otherwise.
extern uint32_t wasm96_graphics_mtl_register_texture(
    uint64_t texture_key,
    const uint8_t* mtl_ptr,
    uint32_t mtl_len,
    const uint8_t* tex_filename_ptr,
    uint32_t tex_filename_len,
    const uint8_t* tex_ptr,
    uint32_t tex_len
) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mtl_register_texture");

extern uint32_t wasm96_graphics_svg_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_svg_register");
extern void wasm96_graphics_svg_draw_key(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_svg_draw_key");
extern void wasm96_graphics_svg_unregister(uint64_t key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_svg_unregister");

extern uint32_t wasm96_graphics_gif_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_gif_register");
extern void wasm96_graphics_gif_draw_key(uint64_t key, int32_t x, int32_t y) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_gif_draw_key");
extern void wasm96_graphics_gif_draw_key_scaled(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_gif_draw_key_scaled");
extern void wasm96_graphics_gif_unregister(uint64_t key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_gif_unregister");

extern uint32_t wasm96_graphics_png_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_png_register");
extern void wasm96_graphics_png_draw_key(uint64_t key, int32_t x, int32_t y) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_png_draw_key");
extern void wasm96_graphics_png_draw_key_scaled(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_png_draw_key_scaled");
extern void wasm96_graphics_png_unregister(uint64_t key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_png_unregister");

extern uint32_t wasm96_graphics_jpeg_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_jpeg_register");
extern void wasm96_graphics_jpeg_draw_key(uint64_t key, int32_t x, int32_t y) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_jpeg_draw_key");
extern void wasm96_graphics_jpeg_draw_key_scaled(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_jpeg_draw_key_scaled");
extern void wasm96_graphics_jpeg_unregister(uint64_t key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_jpeg_unregister");

extern uint32_t wasm96_graphics_font_register_ttf(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_font_register_ttf");
extern uint32_t wasm96_graphics_font_register_bdf(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_font_register_bdf");
extern uint32_t wasm96_graphics_font_register_spleen(uint64_t key, uint32_t size) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_font_register_spleen");
extern void wasm96_graphics_font_unregister(uint64_t key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_font_unregister");
extern void wasm96_graphics_text_key(int32_t x, int32_t y, uint64_t font_key, const uint8_t* text_ptr, uint32_t text_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_text_key");
extern uint64_t wasm96_graphics_text_measure_key(uint64_t font_key, const uint8_t* text_ptr, uint32_t text_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_text_measure_key");

// Input
typedef enum {
    WASM96_INPUT_MODE_GAME = 0,
    WASM96_INPUT_MODE_COMPUTER = 1,
} wasm96_input_mode_t;

extern uint32_t wasm96_input_is_button_down(uint32_t port, uint32_t btn) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_input_is_button_down");
extern void wasm96_input_set_mode(uint32_t mode) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_input_set_mode");
extern uint32_t wasm96_input_is_key_down(uint32_t key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_input_is_key_down");
extern int32_t wasm96_input_get_mouse_x(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_input_get_mouse_x");
extern int32_t wasm96_input_get_mouse_y(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_input_get_mouse_y");
extern uint32_t wasm96_input_is_mouse_down(uint32_t btn) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_input_is_mouse_down");

// Audio
extern uint32_t wasm96_audio_init(uint32_t sample_rate) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_audio_init");
extern void wasm96_audio_push_samples(const int16_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_audio_push_samples");
extern void wasm96_audio_play_wav(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_audio_play_wav");
extern void wasm96_audio_play_qoa(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_audio_play_qoa");
extern void wasm96_audio_play_xm(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_audio_play_xm");

// Storage
extern void wasm96_storage_save(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_storage_save");
extern uint64_t wasm96_storage_load(uint64_t key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_storage_load");
extern void wasm96_storage_free(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_storage_free");

// System
extern void wasm96_system_log(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_log");
extern uint64_t wasm96_system_millis(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_millis");
extern uint32_t wasm96_system_day(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_day");
extern uint32_t wasm96_system_hour(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_hour");
extern uint32_t wasm96_system_minute(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_minute");
extern uint32_t wasm96_system_month(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_month");
extern uint32_t wasm96_system_second(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_second");
extern uint32_t wasm96_system_year(void) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_system_year");

// Math - Calculation
extern float wasm96_math_abs(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_abs");
extern float wasm96_math_ceil(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_ceil");
extern float wasm96_math_constrain(float n, float low, float high) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_constrain");
extern float wasm96_math_dist(float x1, float y1, float x2, float y2) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_dist");
extern float wasm96_math_exp(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_exp");
extern float wasm96_math_floor(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_floor");
extern float wasm96_math_fract(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_fract");
extern float wasm96_math_lerp(float start, float stop, float amt) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_lerp");
extern float wasm96_math_log(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_log");
extern float wasm96_math_mag(float x, float y) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_mag");
extern float wasm96_math_map(float value, float start1, float stop1, float start2, float stop2) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_map");
extern float wasm96_math_max(float a, float b) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_max");
extern float wasm96_math_min(float a, float b) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_min");
extern float wasm96_math_norm(float value, float start, float stop) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_norm");
extern float wasm96_math_pow(float n, float e) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_pow");
extern float wasm96_math_round(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_round");
extern float wasm96_math_sq(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_sq");
extern float wasm96_math_sqrt(float n) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_sqrt");

// Math - Trigonometry
extern float wasm96_math_acos(float value) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_acos");
extern float wasm96_math_asin(float value) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_asin");
extern float wasm96_math_atan(float value) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_atan");
extern float wasm96_math_atan2(float y, float x) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_atan2");
extern float wasm96_math_cos(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_cos");
extern float wasm96_math_sin(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_sin");
extern float wasm96_math_tan(float angle) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_tan");
extern float wasm96_math_degrees(float radians) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_degrees");
extern float wasm96_math_radians(float degrees) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_radians");

// Math - Random & Noise
extern float wasm96_math_random(float min, float max) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_random");
extern void wasm96_math_random_seed(uint32_t seed) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_random_seed");
extern float wasm96_math_random_gaussian(float mean, float sd) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_random_gaussian");
extern float wasm96_math_noise(float x, float y, float z) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_noise");
extern void wasm96_math_noise_seed(uint32_t seed) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_noise_seed");
extern void wasm96_math_noise_detail(uint32_t lod, float falloff) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_math_noise_detail");

} // extern "C"

// Hash function
static inline uint64_t wasm96_hash_key(const char* key) {
    uint64_t hash = 0xcbf29ce484222325ULL;
    uint32_t i = 0;
    while (key[i] != '\0') {
        hash ^= (uint64_t)key[i];
        hash *= 0x100000001b3ULL;
        i++;
    }
    return hash;
}

// Graphics API
namespace wasm96 {

    class Vector2 {
    public:
        float x, y;
        Vector2(float x = 0, float y = 0) : x(x), y(y) {}
        void add(Vector2 other) { x += other.x; y += other.y; }
        void sub(Vector2 other) { x -= other.x; y -= other.y; }
        void mult(float n) { x *= n; y *= n; }
        void div(float n) { if (n != 0) { x /= n; y /= n; } }
        float mag() const { return wasm96_math_mag(x, y); }
        float dist(Vector2 other) const { return wasm96_math_dist(x, y, other.x, other.y); }
        float dot(Vector2 other) const { return x * other.x + y * other.y; }
        void normalize() { float m = mag(); if (m != 0) div(m); }
        void limit(float max) { if (mag() > max) { normalize(); mult(max); } }
        float heading() const { return wasm96_math_atan2(y, x); }
        void rotate(float angle) {
            float c = wasm96_math_cos(angle);
            float s = wasm96_math_sin(angle);
            float nx = x * c - y * s;
            float ny = x * s + y * c;
            x = nx; y = ny;
        }
        void lerp(Vector2 target, float amt) {
            x = wasm96_math_lerp(x, target.x, amt);
            y = wasm96_math_lerp(y, target.y, amt);
        }
    };

    class Vector3 {
    public:
        float x, y, z;
        Vector3(float x = 0, float y = 0, float z = 0) : x(x), y(y), z(z) {}
        void add(Vector3 other) { x += other.x; y += other.y; z += other.z; }
        void sub(Vector3 other) { x -= other.x; y -= other.y; z -= other.z; }
        void mult(float n) { x *= n; y *= n; z *= n; }
        void div(float n) { if (n != 0) { x /= n; y /= n; z /= n; } }
        float mag() const { return wasm96_math_sqrt(x * x + y * y + z * z); }
        float dot(Vector3 other) const { return x * other.x + y * other.y + z * other.z; }
        Vector3 cross(Vector3 other) const {
            return Vector3(
                y * other.z - z * other.y,
                z * other.x - x * other.z,
                x * other.y - y * other.x
            );
        }
        void normalize() { float m = mag(); if (m != 0) div(m); }
        void lerp(Vector3 target, float amt) {
            x = wasm96_math_lerp(x, target.x, amt);
            y = wasm96_math_lerp(y, target.y, amt);
            z = wasm96_math_lerp(z, target.z, amt);
        }
    };

    class Graphics {
public:
    static void setSize(uint32_t width, uint32_t height) { wasm96_graphics_set_size(width, height); }
    static void setColor(uint8_t r, uint8_t g, uint8_t b, uint8_t a) { wasm96_graphics_set_color(r, g, b, a); }
    static void background(uint8_t r, uint8_t g, uint8_t b) { wasm96_graphics_background(r, g, b); }
    static void point(int32_t x, int32_t y) { wasm96_graphics_point(x, y); }
    static void line(int32_t x1, int32_t y1, int32_t x2, int32_t y2) { wasm96_graphics_line(x1, y1, x2, y2); }
    static void rect(int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_rect(x, y, w, h); }
    static void rectOutline(int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_rect_outline(x, y, w, h); }
    static void applyMatrix(float m00, float m01, float m02, float m03, float m10, float m11, float m12, float m13, float m20, float m21, float m22, float m23, float m30, float m31, float m32, float m33) { wasm96_graphics_apply_matrix(m00, m01, m02, m03, m10, m11, m12, m13, m20, m21, m22, m23, m30, m31, m32, m33); }
    static void resetMatrix() { wasm96_graphics_reset_matrix(); }
    static void rotate(float angle) { wasm96_graphics_rotate(angle); }
    static void rotateX(float angle) { wasm96_graphics_rotate_x(angle); }
    static void rotateY(float angle) { wasm96_graphics_rotate_y(angle); }
    static void rotateZ(float angle) { wasm96_graphics_rotate_z(angle); }
    static void scale(float sx, float sy, float sz) { wasm96_graphics_scale(sx, sy, sz); }
    static void shearX(float angle) { wasm96_graphics_shear_x(angle); }
    static void shearY(float angle) { wasm96_graphics_shear_y(angle); }
    static void translate(float x, float y, float z) { wasm96_graphics_translate(x, y, z); }
    static void pushMatrix() { wasm96_graphics_push_matrix(); }
    static void popMatrix() { wasm96_graphics_pop_matrix(); }
    static void circle(int32_t x, int32_t y, uint32_t r) { wasm96_graphics_circle(x, y, r); }
    static void circleOutline(int32_t x, int32_t y, uint32_t r) { wasm96_graphics_circle_outline(x, y, r); }
    static void image(int32_t x, int32_t y, uint32_t w, uint32_t h, const uint8_t* data, uint32_t len) { wasm96_graphics_image(x, y, w, h, data, len); }
    static void imagePng(int32_t x, int32_t y, const uint8_t* data, uint32_t len) { wasm96_graphics_image_png(x, y, data, len); }
    static void imageJpeg(int32_t x, int32_t y, const uint8_t* data, uint32_t len) { wasm96_graphics_image_jpeg(x, y, data, len); }
    static void ellipse(int32_t cx, int32_t cy, uint32_t w, uint32_t h) { wasm96_graphics_ellipse(cx, cy, w, h); }
    static void arc(int32_t cx, int32_t cy, uint32_t w, uint32_t h, float start, float end) { wasm96_graphics_arc(cx, cy, w, h, start, end); }
    static void quad(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3, int32_t x4, int32_t y4) { wasm96_graphics_quad(x1, y1, x2, y2, x3, y3, x4, y4); }
    static void triangle(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) { wasm96_graphics_triangle(x1, y1, x2, y2, x3, y3); }
    static void triangleOutline(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) { wasm96_graphics_triangle_outline(x1, y1, x2, y2, x3, y3); }
    static void bezierQuadratic(int32_t x1, int32_t y1, int32_t cx, int32_t cy, int32_t x2, int32_t y2, uint32_t segments) { wasm96_graphics_bezier_quadratic(x1, y1, cx, cy, x2, y2, segments); }
    static void bezierCubic(int32_t x1, int32_t y1, int32_t cx1, int32_t cy1, int32_t cx2, int32_t cy2, int32_t x2, int32_t y2, uint32_t segments) { wasm96_graphics_bezier_cubic(x1, y1, cx1, cy1, cx2, cy2, x2, y2, segments); }
    static void pill(int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_pill(x, y, w, h); }
    static void pillOutline(int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_pill_outline(x, y, w, h); }

    static void set3d(bool enable) { wasm96_graphics_set_3d(enable ? 1 : 0); }
    static void cameraLookAt(float ex, float ey, float ez, float tx, float ty, float tz, float ux, float uy, float uz) { wasm96_graphics_camera_look_at(ex, ey, ez, tx, ty, tz, ux, uy, uz); }
    static void cameraPerspective(float fovy, float aspect, float near, float far) { wasm96_graphics_camera_perspective(fovy, aspect, near, far); }
    static bool meshCreate(const char* key, const float* vertices, uint32_t v_len, const uint32_t* indices, uint32_t i_len) { return wasm96_graphics_mesh_create(wasm96_hash_key(key), vertices, v_len, indices, i_len) != 0; }
    static bool meshCreateObj(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_mesh_create_obj(wasm96_hash_key(key), data, len) != 0; }
    static bool meshCreateStl(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_mesh_create_stl(wasm96_hash_key(key), data, len) != 0; }
    static void meshDraw(const char* key, float x, float y, float z, float rx, float ry, float rz, float sx, float sy, float sz) { wasm96_graphics_mesh_draw(wasm96_hash_key(key), x, y, z, rx, ry, rz, sx, sy, sz); }
    static bool meshSetTexture(const char* mesh_key, const char* image_key) { return wasm96_graphics_mesh_set_texture(wasm96_hash_key(mesh_key), wasm96_hash_key(image_key)) != 0; }

    // Register an encoded texture referenced by an `.mtl` file (`map_Kd`) under `texture_key`.
    //
    // Returns `true` if it registered (filename matched + decode succeeded), else `false`.
    static bool mtlRegisterTexture(
        const char* texture_key,
        const uint8_t* mtl_bytes,
        uint32_t mtl_len,
        const char* tex_filename,
        const uint8_t* tex_bytes,
        uint32_t tex_len
    ) {
        return wasm96_graphics_mtl_register_texture(
            wasm96_hash_key(texture_key),
            mtl_bytes,
            mtl_len,
            (const uint8_t*)tex_filename,
            wasm96_strlen_(tex_filename),
            tex_bytes,
            tex_len
        ) != 0;
    }

    static bool svgRegister(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_svg_register(wasm96_hash_key(key), data, len) != 0; }
    static void svgDrawKey(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_svg_draw_key(wasm96_hash_key(key), x, y, w, h); }
    static void svgUnregister(const char* key) { wasm96_graphics_svg_unregister(wasm96_hash_key(key)); }

    static bool gifRegister(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_gif_register(wasm96_hash_key(key), data, len) != 0; }
    static void gifDrawKey(const char* key, int32_t x, int32_t y) { wasm96_graphics_gif_draw_key(wasm96_hash_key(key), x, y); }
    static void gifDrawKeyScaled(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_gif_draw_key_scaled(wasm96_hash_key(key), x, y, w, h); }
    static void gifUnregister(const char* key) { wasm96_graphics_gif_unregister(wasm96_hash_key(key)); }

    static bool pngRegister(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_png_register(wasm96_hash_key(key), data, len) != 0; }
    static void pngDrawKey(const char* key, int32_t x, int32_t y) { wasm96_graphics_png_draw_key(wasm96_hash_key(key), x, y); }
    static void pngDrawKeyScaled(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_png_draw_key_scaled(wasm96_hash_key(key), x, y, w, h); }
    static void pngUnregister(const char* key) { wasm96_graphics_png_unregister(wasm96_hash_key(key)); }

    static bool jpegRegister(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_jpeg_register(wasm96_hash_key(key), data, len) != 0; }
    static void jpegDrawKey(const char* key, int32_t x, int32_t y) { wasm96_graphics_jpeg_draw_key(wasm96_hash_key(key), x, y); }
    static void jpegDrawKeyScaled(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_jpeg_draw_key_scaled(wasm96_hash_key(key), x, y, w, h); }
    static void jpegUnregister(const char* key) { wasm96_graphics_jpeg_unregister(wasm96_hash_key(key)); }

    static bool fontRegisterTtf(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_font_register_ttf(wasm96_hash_key(key), data, len) != 0; }
    static bool fontRegisterBdf(const char* key, const uint8_t* data, uint32_t len) { return wasm96_graphics_font_register_bdf(wasm96_hash_key(key), data, len) != 0; }
    static bool fontRegisterSpleen(const char* key, uint32_t size) { return wasm96_graphics_font_register_spleen(wasm96_hash_key(key), size) != 0; }
    static void fontUnregister(const char* key) { wasm96_graphics_font_unregister(wasm96_hash_key(key)); }
    static void textKey(int32_t x, int32_t y, const char* font_key, const char* text) {
        uint32_t len = wasm96_strlen_(text);
        wasm96_graphics_text_key(x, y, wasm96_hash_key(font_key), (const uint8_t*)text, len);
    }
    static wasm96_text_size_t textMeasureKey(const char* font_key, const char* text) {
        uint32_t len = wasm96_strlen_(text);
        uint64_t packed = wasm96_graphics_text_measure_key(wasm96_hash_key(font_key), (const uint8_t*)text, len);
        wasm96_text_size_t ts;
        ts.width = (uint32_t)(packed >> 32);
        ts.height = (uint32_t)(packed & 0xFFFFFFFFULL);
        return ts;
    }
};

class Input {
public:
    static void setMode(wasm96_input_mode_t mode) { wasm96_input_set_mode(static_cast<uint32_t>(mode)); }
    static bool isButtonDown(uint32_t port, wasm96_button_t btn) { return wasm96_input_is_button_down(port, static_cast<uint32_t>(btn)) != 0; }
    static bool isKeyDown(uint32_t key) { return wasm96_input_is_key_down(key) != 0; }
    static int32_t getMouseX() { return wasm96_input_get_mouse_x(); }
    static int32_t getMouseY() { return wasm96_input_get_mouse_y(); }
    static bool isMouseDown(uint32_t btn) { return wasm96_input_is_mouse_down(btn) != 0; }
};

class Audio {
public:
    static uint32_t init(uint32_t sample_rate) { return wasm96_audio_init(sample_rate); }
    static void pushSamples(const int16_t* samples, uint32_t len) { wasm96_audio_push_samples(samples, len); }
    static void playWav(const uint8_t* data, uint32_t len) { wasm96_audio_play_wav(data, len); }
    static void playQoa(const uint8_t* data, uint32_t len) { wasm96_audio_play_qoa(data, len); }
    static void playXm(const uint8_t* data, uint32_t len) { wasm96_audio_play_xm(data, len); }
};

class Storage {
public:
    static void save(const char* key, const uint8_t* data, uint32_t len) {
        wasm96_storage_save(wasm96_hash_key(key), data, len);
    }
};

class Math {
public:
    static float abs(float n) { return wasm96_math_abs(n); }
    static float ceil(float n) { return wasm96_math_ceil(n); }
    static float constrain(float n, float low, float high) { return wasm96_math_constrain(n, low, high); }
    static float dist(float x1, float y1, float x2, float y2) { return wasm96_math_dist(x1, y1, x2, y2); }
    static float exp(float n) { return wasm96_math_exp(n); }
    static float floor(float n) { return wasm96_math_floor(n); }
    static float fract(float n) { return wasm96_math_fract(n); }
    static float lerp(float start, float stop, float amt) { return wasm96_math_lerp(start, stop, amt); }
    static float log(float n) { return wasm96_math_log(n); }
    static float mag(float x, float y) { return wasm96_math_mag(x, y); }
    static float map(float value, float start1, float stop1, float start2, float stop2) { return wasm96_math_map(value, start1, stop1, start2, stop2); }
    static float max(float a, float b) { return wasm96_math_max(a, b); }
    static float min(float a, float b) { return wasm96_math_min(a, b); }
    static float norm(float value, float start, float stop) { return wasm96_math_norm(value, start, stop); }
    static float pow(float n, float e) { return wasm96_math_pow(n, e); }
    static float round(float n) { return wasm96_math_round(n); }
    static float sq(float n) { return wasm96_math_sq(n); }
    static float sqrt(float n) { return wasm96_math_sqrt(n); }

    static float acos(float value) { return wasm96_math_acos(value); }
    static float asin(float value) { return wasm96_math_asin(value); }
    static float atan(float value) { return wasm96_math_atan(value); }
    static float atan2(float y, float x) { return wasm96_math_atan2(y, x); }
    static float cos(float angle) { return wasm96_math_cos(angle); }
    static float sin(float angle) { return wasm96_math_sin(angle); }
    static float tan(float angle) { return wasm96_math_tan(angle); }
    static float degrees(float radians) { return wasm96_math_degrees(radians); }
    static float radians(float degrees) { return wasm96_math_radians(degrees); }

    static float random(float min, float max) { return wasm96_math_random(min, max); }
    static void randomSeed(uint32_t seed) { wasm96_math_random_seed(seed); }
    static float randomGaussian(float mean, float sd) { return wasm96_math_random_gaussian(mean, sd); }
    static float noise(float x, float y = 0, float z = 0) { return wasm96_math_noise(x, y, z); }
    static void noiseSeed(uint32_t seed) { wasm96_math_noise_seed(seed); }
    static void noiseDetail(uint32_t lod, float falloff) { wasm96_math_noise_detail(lod, falloff); }

    static Vector2 createVector(float x, float y) { return Vector2(x, y); }
    static Vector3 createVector3(float x, float y, float z) { return Vector3(x, y, z); }
};

class System {
public:
    static void log(const char* message) {
        uint32_t len = wasm96_strlen_(message);
        wasm96_system_log((const uint8_t*)message, len);
    }
    static uint64_t millis() { return wasm96_system_millis(); }
    static uint32_t day() { return wasm96_system_day(); }
    static uint32_t hour() { return wasm96_system_hour(); }
    static uint32_t minute() { return wasm96_system_minute(); }
    static uint32_t month() { return wasm96_system_month(); }
    static uint32_t second() { return wasm96_system_second(); }
    static uint32_t year() { return wasm96_system_year(); }
};

} // namespace wasm96

// User must implement these functions
extern "C" {
void setup();
void update();
void draw();
}

#endif // WASM96_HPP
