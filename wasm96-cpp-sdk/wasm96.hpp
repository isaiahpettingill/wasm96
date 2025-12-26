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
extern void wasm96_graphics_triangle(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_triangle");
extern void wasm96_graphics_triangle_outline(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t x3, int32_t y3) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_triangle_outline");
extern void wasm96_graphics_bezier_quadratic(int32_t x1, int32_t y1, int32_t cx, int32_t cy, int32_t x2, int32_t y2, uint32_t segments) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_bezier_quadratic");
extern void wasm96_graphics_bezier_cubic(int32_t x1, int32_t y1, int32_t cx1, int32_t cy1, int32_t cx2, int32_t cy2, int32_t x2, int32_t y2, uint32_t segments) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_bezier_cubic");
extern void wasm96_graphics_pill(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_pill");
extern void wasm96_graphics_pill_outline(int32_t x, int32_t y, uint32_t w, uint32_t h) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_pill_outline");

// 3D Graphics
extern void wasm96_graphics_set_3d(uint32_t enable) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_set_3d");
extern void wasm96_graphics_camera_look_at(float eye_x, float eye_y, float eye_z, float target_x, float target_y, float target_z, float up_x, float up_y, float up_z) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_camera_look_at");
extern void wasm96_graphics_camera_perspective(float fovy, float aspect, float near, float far) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_camera_perspective");
extern uint32_t wasm96_graphics_mesh_create(uint64_t key, const float* v_ptr, uint32_t v_len, const uint32_t* i_ptr, uint32_t i_len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_create");
extern uint32_t wasm96_graphics_mesh_create_obj(uint64_t key, const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_create_obj");
extern uint32_t wasm96_graphics_mesh_create_stl(uint64_t key, const uint8_t* ptr, uint32_t len) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_create_stl");
extern void wasm96_graphics_mesh_draw(uint64_t key, float x, float y, float z, float rx, float ry, float rz, float sx, float sy, float sz) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_draw");
extern uint32_t wasm96_graphics_mesh_set_texture(uint64_t mesh_key, uint64_t image_key) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_graphics_mesh_set_texture");

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
extern uint32_t wasm96_input_is_button_down(uint32_t port, uint32_t btn) WASM96_WASM_IMPORT(WASM96_WASM_IMPORT_MODULE, "wasm96_input_is_button_down");
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

class Graphics {
public:
    static void setSize(uint32_t width, uint32_t height) { wasm96_graphics_set_size(width, height); }
    static void setColor(uint8_t r, uint8_t g, uint8_t b, uint8_t a) { wasm96_graphics_set_color(r, g, b, a); }
    static void background(uint8_t r, uint8_t g, uint8_t b) { wasm96_graphics_background(r, g, b); }
    static void point(int32_t x, int32_t y) { wasm96_graphics_point(x, y); }
    static void line(int32_t x1, int32_t y1, int32_t x2, int32_t y2) { wasm96_graphics_line(x1, y1, x2, y2); }
    static void rect(int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_rect(x, y, w, h); }
    static void rectOutline(int32_t x, int32_t y, uint32_t w, uint32_t h) { wasm96_graphics_rect_outline(x, y, w, h); }
    static void circle(int32_t x, int32_t y, uint32_t r) { wasm96_graphics_circle(x, y, r); }
    static void circleOutline(int32_t x, int32_t y, uint32_t r) { wasm96_graphics_circle_outline(x, y, r); }
    static void image(int32_t x, int32_t y, uint32_t w, uint32_t h, const uint8_t* data, uint32_t len) { wasm96_graphics_image(x, y, w, h, data, len); }
    static void imagePng(int32_t x, int32_t y, const uint8_t* data, uint32_t len) { wasm96_graphics_image_png(x, y, data, len); }
    static void imageJpeg(int32_t x, int32_t y, const uint8_t* data, uint32_t len) { wasm96_graphics_image_jpeg(x, y, data, len); }
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
    static void save(const char* key, const uint8_t* data, uint32_t len) { wasm96_storage_save(wasm96_hash_key(key), data, len); }
    // load would need allocation, similar to Rust
};

class System {
public:
    static void log(const char* message) {
        uint32_t len = wasm96_strlen_(message);
        wasm96_system_log((const uint8_t*)message, len);
    }
    static uint64_t millis() { return wasm96_system_millis(); }
};

} // namespace wasm96

// User must implement these functions
extern "C" {
void setup();
void update();
void draw();
}

#endif // WASM96_HPP