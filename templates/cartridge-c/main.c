#include "wasm96.h"

static volatile wasm96_pixel* fb;
static volatile int16_t* audio;
static volatile uint8_t* pad;
static uint32_t width;
static uint32_t height;
static int32_t player_x;
static int32_t player_y;
static uint32_t frame;
static bool initialized;

static void fill_rect(volatile wasm96_pixel* target, uint32_t w, uint32_t h,
                      int32_t x, int32_t y, int32_t rw, int32_t rh, wasm96_pixel color) {
    int32_t x0 = x < 0 ? 0 : x;
    int32_t y0 = y < 0 ? 0 : y;
    int32_t x1 = x + rw > (int32_t)w ? (int32_t)w : x + rw;
    int32_t y1 = y + rh > (int32_t)h ? (int32_t)h : y + rh;
    for (int32_t py = y0; py < y1; py++) {
        for (int32_t px = x0; px < x1; px++) {
            target[(uint32_t)py * w + (uint32_t)px] = color;
        }
    }
}

static void init_once(void) {
    if (initialized) return;
    fb = get_framebuffer();
    audio = get_audiobuffer();
    pad = controller_1();
    uint32_t resolution = wasm96_set_resolution(640, 360);
    width = wasm96_resolution_width(resolution);
    height = wasm96_resolution_height(resolution);
    player_x = (int32_t)width / 2 - 12;
    player_y = (int32_t)height / 2 - 12;
    initialized = true;
}

__attribute__((export_name("wasm96_update"))) void wasm96_update(void) {
    init_once();

    if (controller_button_pressed(pad, WASM96_BUTTON_LEFT)) player_x -= 3;
    if (controller_button_pressed(pad, WASM96_BUTTON_RIGHT)) player_x += 3;
    if (controller_button_pressed(pad, WASM96_BUTTON_UP)) player_y -= 3;
    if (controller_button_pressed(pad, WASM96_BUTTON_DOWN)) player_y += 3;

    if (player_x < 0) player_x = 0;
    if (player_y < 0) player_y = 0;
    if (player_x > (int32_t)width - 24) player_x = (int32_t)width - 24;
    if (player_y > (int32_t)height - 24) player_y = (int32_t)height - 24;

    uint32_t pulse = (frame / 2u) & 0x3Fu;
    for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++) {
            uint8_t r = (uint8_t)((x + frame) & 0x3Fu);
            uint8_t g = (uint8_t)((y + pulse) & 0x3Fu);
            uint8_t b = (uint8_t)(0x30u + ((x ^ y ^ frame) & 0x1Fu));
            fb[y * width + x] = wasm96_rgb(r, g, b);
        }
    }

    for (uint32_t x = 0; x < width; x += 32) {
        fill_rect(fb, width, height, (int32_t)x, (int32_t)((x + frame * 3u) % height), 14, 14, wasm96_rgb(0x20, 0xD8, 0xFF));
    }

    int32_t orb_x = (int32_t)((frame * 5u) % (width - 48u));
    int32_t orb_y = 42 + (int32_t)(((frame * 3u) % 96u));
    fill_rect(fb, width, height, orb_x, orb_y, 48, 48, wasm96_rgb(0xFF, 0xC8, 0x18));
    fill_rect(fb, width, height, player_x, player_y, 24, 24, wasm96_rgb(0xFF, 0x2E, 0x63));
    fill_rect(fb, width, height, 12, 12, (int32_t)((frame * 4u) % (width - 24u)), 6, wasm96_rgb(0xF8, 0xF8, 0xF2));

    int16_t sample = (frame & 0x80u) ? 9000 : -9000;
    for (uint32_t i = 0; i < WASM96_AUDIO_FRAMES_PER_VIDEO_FRAME; i++) {
        audio[i * 2] = sample;
        audio[i * 2 + 1] = sample;
    }

    wasm96_present();
    frame++;
}
