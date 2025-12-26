#ifndef WASM96_H
#define WASM96_H

/*
Freestanding-friendly header:

This SDK is meant to be used by WebAssembly guests that are often built as
`wasm32-freestanding` (no libc / no WASI). In that environment, standard C
library headers (like <string.h>) may be unavailable.

- We only require fixed-width integer types and `bool`.
- We avoid `size_t` so we don't need <stddef.h> in freestanding builds.
- We provide a tiny local `strlen` replacement when libc isn't present.

Linking notes:

All `wasm96_*` host calls are imported by the runtime (wasm96 core). To ensure
toolchains treat them as WebAssembly imports (and therefore don't require local
definitions), we declare them with an explicit import module/name when building
for wasm.
*/

#include <stdint.h>
#include <stdbool.h>

#if defined(__wasm__) || defined(__EMSCRIPTEN__) || defined(__wasi__)
  // Tell LLVM/Clang-based toolchains to generate `import` entries in the wasm.
  // (GCC-style attributes are ignored by MSVC, but MSVC doesn't target wasm here.)
  #define WASM96_WASM_IMPORT(module, name) __attribute__((import_module(module), import_name(name)))
#else
  #define WASM96_WASM_IMPORT(module, name)
#endif

static inline uint32_t wasm96_strlen_(const char* s) {
    uint32_t n = 0;
    if (!s) return 0;
    while (s[n] != '\0') n++;
    return n;
}
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

// Low-level raw ABI imports.
extern void wasm96_graphics_set_size(uint32_t width, uint32_t height) WASM96_WASM_IMPORT("env", "wasm96_graphics_set_size");
extern void wasm96_graphics_set_color(uint32_t r, uint32_t g, uint32_t b, uint32_t a) WASM96_WASM_IMPORT("env", "wasm96_graphics_set_color");
extern void wasm96_graphics_background(uint32_t r, uint32_t g, uint32_t b) WASM96_WASM_IMPORT("env", "wasm96_graphics_background");
extern void wasm96_graphics_point(int32_t x, int32_t y) WASM96_WASM_IMPORT("env", "wasm96_graphics_point");
extern void wasm96_graphics_line(int32_t x1, int32_t y1, int32_t x2, int32_t y2) WASM96_WASM_IMPORT("env", "wasm96_graphics_line");
extern void wasm96_graphics_rect(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_rect");
extern void wasm96_graphics_rect_outline(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_rect_outline");
extern void wasm96_graphics_circle(int32_t x, int32_t y, uint32_t r) WASM96_WASM_IMPORT("env", "wasm96_graphics_circle");
extern void wasm96_graphics_circle_outline(int32_t x, int32_t y, uint32_t r) WASM96_WASM_IMPORT("env", "wasm96_graphics_circle_outline");
extern void wasm96_graphics_image(int32_t x, int32_t y, uint32_t w, uint32_t h, const uint8_t* data, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_graphics_image");
extern void wasm96_graphics_image_png(int32_t x, int32_t y, const uint8_t* data, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_graphics_image_png");
extern void wasm96_graphics_image_jpeg(int32_t x, int32_t y, const uint8_t* data, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_graphics_image_jpeg");
extern void wasm96_graphics_triangle(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) WASM96_WASM_IMPORT("env", "wasm96_graphics_triangle");
extern void wasm96_graphics_triangle_outline(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) WASM96_WASM_IMPORT("env", "wasm96_graphics_triangle_outline");
extern void wasm96_graphics_bezier_quadratic(int32_t x1, int32_t y1, int32_t cx, int32_t cy, int32_t x2, int32_t y2, uint32_t segments) WASM96_WASM_IMPORT("env", "wasm96_graphics_bezier_quadratic");
extern void wasm96_graphics_bezier_cubic(int32_t x1, int32_t y1, int32_t cx1, int32_t cy1, int32_t cx2, int32_t cy2, int32_t x2, int32_t y2, uint32_t segments) WASM96_WASM_IMPORT("env", "wasm96_graphics_bezier_cubic");
extern void wasm96_graphics_pill(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_pill");
extern void wasm96_graphics_pill_outline(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_pill_outline");

// 3D Graphics
extern void wasm96_graphics_set_3d(uint32_t enable) WASM96_WASM_IMPORT("env", "wasm96_graphics_set_3d");
extern void wasm96_graphics_camera_look_at(float eye_x, float eye_y, float eye_z, float target_x, float target_y, float target_z, float up_x, float up_y, float up_z) WASM96_WASM_IMPORT("env", "wasm96_graphics_camera_look_at");
extern void wasm96_graphics_camera_perspective(float fovy, float aspect, float near, float far) WASM96_WASM_IMPORT("env", "wasm96_graphics_camera_perspective");
extern uint32_t wasm96_graphics_mesh_create(uint64_t key, const float* v_ptr, uint32_t v_len, const uint32_t* i_ptr, uint32_t i_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_mesh_create");
extern uint32_t wasm96_graphics_mesh_create_obj(uint64_t key, const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_graphics_mesh_create_obj");
extern uint32_t wasm96_graphics_mesh_create_stl(uint64_t key, const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_graphics_mesh_create_stl");
extern void wasm96_graphics_mesh_draw(uint64_t key, float x, float y, float z, float rx, float ry, float rz, float sx, float sy, float sz) WASM96_WASM_IMPORT("env", "wasm96_graphics_mesh_draw");
extern uint32_t wasm96_graphics_mesh_set_texture(uint64_t mesh_key, uint64_t image_key) WASM96_WASM_IMPORT("env", "wasm96_graphics_mesh_set_texture");

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
) WASM96_WASM_IMPORT("env", "wasm96_graphics_mtl_register_texture");

extern uint32_t wasm96_graphics_svg_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_svg_register");
extern void wasm96_graphics_svg_draw_key(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_svg_draw_key");
extern void wasm96_graphics_svg_unregister(uint64_t key) WASM96_WASM_IMPORT("env", "wasm96_graphics_svg_unregister");

extern uint32_t wasm96_graphics_gif_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_gif_register");
extern void wasm96_graphics_gif_draw_key(uint64_t key, int32_t x, int32_t y) WASM96_WASM_IMPORT("env", "wasm96_graphics_gif_draw_key");
extern void wasm96_graphics_gif_draw_key_scaled(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_gif_draw_key_scaled");
extern void wasm96_graphics_gif_unregister(uint64_t key) WASM96_WASM_IMPORT("env", "wasm96_graphics_gif_unregister");

extern uint32_t wasm96_graphics_png_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_png_register");
extern void wasm96_graphics_png_draw_key(uint64_t key, int32_t x, int32_t y) WASM96_WASM_IMPORT("env", "wasm96_graphics_png_draw_key");
extern void wasm96_graphics_png_draw_key_scaled(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_png_draw_key_scaled");
extern void wasm96_graphics_png_unregister(uint64_t key) WASM96_WASM_IMPORT("env", "wasm96_graphics_png_unregister");

extern uint32_t wasm96_graphics_jpeg_register(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_jpeg_register");
extern void wasm96_graphics_jpeg_draw_key(uint64_t key, int32_t x, int32_t y) WASM96_WASM_IMPORT("env", "wasm96_graphics_jpeg_draw_key");
extern void wasm96_graphics_jpeg_draw_key_scaled(uint64_t key, int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT("env", "wasm96_graphics_jpeg_draw_key_scaled");
extern void wasm96_graphics_jpeg_unregister(uint64_t key) WASM96_WASM_IMPORT("env", "wasm96_graphics_jpeg_unregister");

extern uint32_t wasm96_graphics_font_register_ttf(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_font_register_ttf");
extern uint32_t wasm96_graphics_font_register_bdf(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_font_register_bdf");
extern uint32_t wasm96_graphics_font_register_spleen(uint64_t key, uint32_t size) WASM96_WASM_IMPORT("env", "wasm96_graphics_font_register_spleen");
extern void wasm96_graphics_font_unregister(uint64_t key) WASM96_WASM_IMPORT("env", "wasm96_graphics_font_unregister");
extern void wasm96_graphics_text_key(int32_t x, int32_t y, uint64_t font_key, const uint8_t* text_ptr, uint32_t text_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_text_key");
extern uint64_t wasm96_graphics_text_measure_key(uint64_t font_key, const uint8_t* text_ptr, uint32_t text_len) WASM96_WASM_IMPORT("env", "wasm96_graphics_text_measure_key");

// Input
extern uint32_t wasm96_input_is_button_down(uint32_t port, uint32_t btn) WASM96_WASM_IMPORT("env", "wasm96_input_is_button_down");
extern uint32_t wasm96_input_is_key_down(uint32_t key) WASM96_WASM_IMPORT("env", "wasm96_input_is_key_down");
extern int32_t wasm96_input_get_mouse_x(void) WASM96_WASM_IMPORT("env", "wasm96_input_get_mouse_x");
extern int32_t wasm96_input_get_mouse_y(void) WASM96_WASM_IMPORT("env", "wasm96_input_get_mouse_y");
extern uint32_t wasm96_input_is_mouse_down(uint32_t btn) WASM96_WASM_IMPORT("env", "wasm96_input_is_mouse_down");

// Audio
extern uint32_t wasm96_audio_init(uint32_t sample_rate) WASM96_WASM_IMPORT("env", "wasm96_audio_init");
extern void wasm96_audio_push_samples(const int16_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_audio_push_samples");
extern void wasm96_audio_play_wav(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_audio_play_wav");
extern void wasm96_audio_play_qoa(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_audio_play_qoa");
extern void wasm96_audio_play_xm(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_audio_play_xm");

// Storage
extern void wasm96_storage_save(uint64_t key, const uint8_t* data_ptr, uint32_t data_len) WASM96_WASM_IMPORT("env", "wasm96_storage_save");
extern uint64_t wasm96_storage_load(uint64_t key) WASM96_WASM_IMPORT("env", "wasm96_storage_load");
extern void wasm96_storage_free(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_storage_free");

// System
extern void wasm96_system_log(const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT("env", "wasm96_system_log");
extern uint64_t wasm96_system_millis(void) WASM96_WASM_IMPORT("env", "wasm96_system_millis");

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
static inline void wasm96_graphics_set_color_rgba(uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    wasm96_graphics_set_color((uint32_t)r, (uint32_t)g, (uint32_t)b, (uint32_t)a);
}

static inline void wasm96_graphics_background_rgb(uint8_t r, uint8_t g, uint8_t b) {
    wasm96_graphics_background((uint32_t)r, (uint32_t)g, (uint32_t)b);
}

static inline bool wasm96_graphics_mesh_create_str(const char* key, const float* vertices, uint32_t v_len, const uint32_t* indices, uint32_t i_len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_mesh_create(k, vertices, v_len, indices, i_len) != 0;
}

static inline bool wasm96_graphics_mesh_create_obj_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_mesh_create_obj(k, data, len) != 0;
}

static inline bool wasm96_graphics_mesh_create_stl_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_mesh_create_stl(k, data, len) != 0;
}

static inline void wasm96_graphics_mesh_draw_str(const char* key, float x, float y, float z, float rx, float ry, float rz, float sx, float sy, float sz) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_mesh_draw(k, x, y, z, rx, ry, rz, sx, sy, sz);
}

static inline bool wasm96_graphics_mesh_set_texture_str(const char* mesh_key, const char* image_key) {
    uint64_t mk = wasm96_hash_key(mesh_key);
    uint64_t ik = wasm96_hash_key(image_key);
    return wasm96_graphics_mesh_set_texture(mk, ik) != 0;
}

static inline bool wasm96_graphics_mtl_register_texture_str(
    const char* texture_key,
    const uint8_t* mtl_bytes,
    uint32_t mtl_len,
    const char* tex_filename,
    const uint8_t* tex_bytes,
    uint32_t tex_len
) {
    uint64_t tk = wasm96_hash_key(texture_key);
    return wasm96_graphics_mtl_register_texture(
        tk,
        mtl_bytes,
        mtl_len,
        (const uint8_t*)tex_filename,
        wasm96_strlen_(tex_filename),
        tex_bytes,
        tex_len
    ) != 0;
}

static inline bool wasm96_graphics_svg_register_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_svg_register(k, data, len) != 0;
}

static inline void wasm96_graphics_svg_draw_key_str(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_svg_draw_key(k, x, y, w, h);
}

static inline void wasm96_graphics_svg_unregister_str(const char* key) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_svg_unregister(k);
}

static inline bool wasm96_graphics_gif_register_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_gif_register(k, data, len) != 0;
}

static inline void wasm96_graphics_gif_draw_key_str(const char* key, int32_t x, int32_t y) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_gif_draw_key(k, x, y);
}

static inline void wasm96_graphics_gif_draw_key_scaled_str(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_gif_draw_key_scaled(k, x, y, w, h);
}

static inline void wasm96_graphics_gif_unregister_str(const char* key) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_gif_unregister(k);
}

static inline bool wasm96_graphics_png_register_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_png_register(k, data, len) != 0;
}

static inline void wasm96_graphics_png_draw_key_str(const char* key, int32_t x, int32_t y) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_png_draw_key(k, x, y);
}

static inline void wasm96_graphics_png_draw_key_scaled_str(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_png_draw_key_scaled(k, x, y, w, h);
}

static inline void wasm96_graphics_png_unregister_str(const char* key) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_png_unregister(k);
}

static inline bool wasm96_graphics_jpeg_register_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_jpeg_register(k, data, len) != 0;
}

static inline void wasm96_graphics_jpeg_draw_key_str(const char* key, int32_t x, int32_t y) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_jpeg_draw_key(k, x, y);
}

static inline void wasm96_graphics_jpeg_draw_key_scaled_str(const char* key, int32_t x, int32_t y, uint32_t w, uint32_t h) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_jpeg_draw_key_scaled(k, x, y, w, h);
}

static inline void wasm96_graphics_jpeg_unregister_str(const char* key) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_jpeg_unregister(k);
}

static inline bool wasm96_graphics_font_register_ttf_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_font_register_ttf(k, data, len) != 0;
}

static inline bool wasm96_graphics_font_register_bdf_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_font_register_bdf(k, data, len) != 0;
}

static inline bool wasm96_graphics_font_register_spleen_str(const char* key, uint32_t size) {
    uint64_t k = wasm96_hash_key(key);
    return wasm96_graphics_font_register_spleen(k, size) != 0;
}

static inline void wasm96_graphics_font_unregister_str(const char* key) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_graphics_font_unregister(k);
}

static inline void wasm96_graphics_text_key_str(int32_t x, int32_t y, const char* font_key, const char* text) {
    uint64_t fk = wasm96_hash_key(font_key);
#if WASM96_HAS_STRING_H
    uint32_t len = (uint32_t)strlen(text);
#else
    uint32_t len = wasm96_strlen_(text);
#endif
    wasm96_graphics_text_key(x, y, fk, (const uint8_t*)text, len);
}

static inline wasm96_text_size_t wasm96_graphics_text_measure_key_str(const char* font_key, const char* text) {
    uint64_t fk = wasm96_hash_key(font_key);
#if WASM96_HAS_STRING_H
    uint32_t len = (uint32_t)strlen(text);
#else
    uint32_t len = wasm96_strlen_(text);
#endif
    uint64_t packed = wasm96_graphics_text_measure_key(fk, (const uint8_t*)text, len);
    wasm96_text_size_t ts;
    ts.width = (uint32_t)(packed >> 32);
    ts.height = (uint32_t)(packed & 0xFFFFFFFFULL);
    return ts;
}

// Input API
static inline bool wasm96_input_is_button_down_enum(uint32_t port, wasm96_button_t btn) {
    return wasm96_input_is_button_down(port, (uint32_t)btn) != 0;
}

static inline bool wasm96_input_is_key_down_bool(uint32_t key) {
    return wasm96_input_is_key_down(key) != 0;
}

static inline bool wasm96_input_is_mouse_down_bool(uint32_t btn) {
    return wasm96_input_is_mouse_down(btn) != 0;
}

// Audio API
// (functions are direct)

// Storage API
static inline void wasm96_storage_save_str(const char* key, const uint8_t* data, uint32_t len) {
    uint64_t k = wasm96_hash_key(key);
    wasm96_storage_save(k, data, len);
}

// For load, need to handle the packed return
// User needs to implement allocation

// System API
static inline void wasm96_system_log_str(const char* message) {
#if WASM96_HAS_STRING_H
    uint32_t len = (uint32_t)strlen(message);
#else
    uint32_t len = wasm96_strlen_(message);
#endif
    wasm96_system_log((const uint8_t*)message, len);
}

// User must implement these functions
void setup(void);
void update(void);
void draw(void);

#endif // WASM96_H
