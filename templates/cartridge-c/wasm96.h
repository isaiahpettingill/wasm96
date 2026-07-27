#ifndef WASM96_H
#define WASM96_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define WASM96_FB_WIDTH 320u
#define WASM96_FB_HEIGHT 224u
#define WASM96_FB_MAX_WIDTH 1024u
#define WASM96_FB_MAX_HEIGHT 1024u
#define WASM96_AUDIO_SAMPLE_RATE 48000u
#define WASM96_AUDIO_CHANNELS 2u
#define WASM96_AUDIO_FRAMES_PER_VIDEO_FRAME 800u
#define WASM96_AUDIO_SAMPLES (WASM96_AUDIO_FRAMES_PER_VIDEO_FRAME * WASM96_AUDIO_CHANNELS)
#define WASM96_CONTROLLER_BUTTONS 12u
#define WASM96_CONTROLLER_PACKED_BYTES 3u

typedef uint32_t wasm96_pixel;

typedef enum {
    WASM96_BUTTON_UP = 0,
    WASM96_BUTTON_DOWN = 1,
    WASM96_BUTTON_LEFT = 2,
    WASM96_BUTTON_RIGHT = 3,
    WASM96_BUTTON_A = 4,
    WASM96_BUTTON_B = 5,
    WASM96_BUTTON_X = 6,
    WASM96_BUTTON_Y = 7,
    WASM96_BUTTON_L1 = 8,
    WASM96_BUTTON_R1 = 9,
    WASM96_BUTTON_START = 10,
    WASM96_BUTTON_SELECT = 11,
} wasm96_button;

typedef enum {
    WASM96_CONTROLLER_NONE = 0,
    WASM96_CONTROLLER_KEYBOARD = 1,
    WASM96_CONTROLLER_GAMEPAD = 2,
    WASM96_CONTROLLER_JOYSTICK = 3,
    WASM96_CONTROLLER_LIBRETRO_JOYPAD = 4,
} wasm96_controller_type;

__attribute__((import_module("env"), import_name("get_framebuffer"))) extern uintptr_t wasm96_host_get_framebuffer(void);
__attribute__((import_module("env"), import_name("get_audiobuffer"))) extern uintptr_t wasm96_host_get_audiobuffer(void);
__attribute__((import_module("env"), import_name("controller_1"))) extern uintptr_t wasm96_host_controller_1(void);
__attribute__((import_module("env"), import_name("controller_2"))) extern uintptr_t wasm96_host_controller_2(void);
__attribute__((import_module("env"), import_name("controller_3"))) extern uintptr_t wasm96_host_controller_3(void);
__attribute__((import_module("env"), import_name("controller_4"))) extern uintptr_t wasm96_host_controller_4(void);
__attribute__((import_module("env"), import_name("present"))) extern void wasm96_host_present(void);
__attribute__((import_module("env"), import_name("set_resolution"))) extern uint32_t wasm96_host_set_resolution(uint32_t width, uint32_t height);
__attribute__((import_module("env"), import_name("sram_size"))) extern uint32_t wasm96_host_sram_size(void);
__attribute__((import_module("env"), import_name("sram_read"))) extern uint32_t wasm96_host_sram_read(uint32_t offset, void* dst, uint32_t len);
__attribute__((import_module("env"), import_name("sram_write"))) extern uint32_t wasm96_host_sram_write(uint32_t offset, const void* src, uint32_t len);
__attribute__((import_module("env"), import_name("controller_count"))) extern uint32_t wasm96_host_controller_count(void);
__attribute__((import_module("env"), import_name("controller_info"))) extern uint32_t wasm96_host_controller_info(uint32_t port);
__attribute__((import_module("env"), import_name("time_ms"))) extern uint32_t wasm96_host_time_ms(void);
__attribute__((import_module("env"), import_name("delta_ms"))) extern uint32_t wasm96_host_delta_ms(void);
__attribute__((import_module("env"), import_name("debug_log"))) extern uint32_t wasm96_host_debug_log(const void* src, uint32_t len);
__attribute__((import_module("env"), import_name("debug_trace"))) extern void wasm96_host_debug_trace(uint32_t a, uint32_t b, uint32_t c);
__attribute__((import_module("env"), import_name("debug_mem_read"))) extern uint32_t wasm96_host_debug_mem_read(const void* src, void* dst, uint32_t len);
__attribute__((import_module("env"), import_name("debug_mem_write"))) extern uint32_t wasm96_host_debug_mem_write(void* dst, const void* src, uint32_t len);
__attribute__((import_module("env"), import_name("exit"))) extern void wasm96_host_exit(int code);

static inline wasm96_pixel wasm96_rgb(uint8_t r, uint8_t g, uint8_t b) {
    return ((uint32_t)r << 16) | ((uint32_t)g << 8) | (uint32_t)b;
}

static inline volatile wasm96_pixel* get_framebuffer(void) {
    return (volatile wasm96_pixel*)wasm96_host_get_framebuffer();
}

static inline volatile int16_t* get_audiobuffer(void) {
    return (volatile int16_t*)wasm96_host_get_audiobuffer();
}

static inline volatile uint8_t* controller_1(void) { return (volatile uint8_t*)wasm96_host_controller_1(); }
static inline volatile uint8_t* controller_2(void) { return (volatile uint8_t*)wasm96_host_controller_2(); }
static inline volatile uint8_t* controller_3(void) { return (volatile uint8_t*)wasm96_host_controller_3(); }
static inline volatile uint8_t* controller_4(void) { return (volatile uint8_t*)wasm96_host_controller_4(); }
static inline void wasm96_present(void) { wasm96_host_present(); }
static inline uint32_t wasm96_set_resolution(uint32_t width, uint32_t height) { return wasm96_host_set_resolution(width, height); }
static inline uint32_t wasm96_resolution_width(uint32_t packed_resolution) { return packed_resolution & 0xFFFFu; }
static inline uint32_t wasm96_resolution_height(uint32_t packed_resolution) { return packed_resolution >> 16u; }
static inline uint32_t wasm96_sram_size(void) { return wasm96_host_sram_size(); }
static inline uint32_t wasm96_sram_read(uint32_t offset, void* dst, size_t len) { return wasm96_host_sram_read(offset, dst, (uint32_t)len); }
static inline uint32_t wasm96_sram_write(uint32_t offset, const void* src, size_t len) { return wasm96_host_sram_write(offset, src, (uint32_t)len); }
static inline uint32_t wasm96_controller_count(void) { return wasm96_host_controller_count(); }
static inline uint32_t wasm96_controller_info(uint32_t port) { return wasm96_host_controller_info(port); }
static inline bool wasm96_controller_connected(uint32_t info) { return (info & 1u) != 0u; }
static inline wasm96_controller_type wasm96_controller_type_from_info(uint32_t info) { return (wasm96_controller_type)((info >> 8u) & 0xFFu); }
static inline uint32_t wasm96_time_ms(void) { return wasm96_host_time_ms(); }
static inline uint32_t wasm96_delta_ms(void) { return wasm96_host_delta_ms(); }
static inline uint32_t wasm96_debug_log(const void* src, size_t len) { return wasm96_host_debug_log(src, (uint32_t)len); }
static inline void wasm96_debug_trace(uint32_t a, uint32_t b, uint32_t c) { wasm96_host_debug_trace(a, b, c); }
static inline uint32_t wasm96_debug_mem_read(const void* src, void* dst, size_t len) { return wasm96_host_debug_mem_read(src, dst, (uint32_t)len); }
static inline uint32_t wasm96_debug_mem_write(void* dst, const void* src, size_t len) { return wasm96_host_debug_mem_write(dst, src, (uint32_t)len); }

static inline uint8_t controller_button_level(const volatile uint8_t* packed, wasm96_button button) {
    const uint32_t button_index = (uint32_t)button;
    if (button_index >= WASM96_CONTROLLER_BUTTONS) return 0;
    const uint32_t bit = button_index * 2u;
    return (uint8_t)((packed[bit >> 3] >> (bit & 7u)) & 0x3u);
}

static inline bool controller_button_pressed(const volatile uint8_t* packed, wasm96_button button) {
    return controller_button_level(packed, button) > 0u;
}

static inline void wasm96_exit(int code) {
    wasm96_host_exit(code);
    __builtin_trap();
}

#endif
