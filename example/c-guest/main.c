#include "wasm96.h"

void setup() {
    wasm96_graphics_set_size(640, 480);
    wasm96_graphics_set_color_rgba(255, 255, 255, 255);
}

void update() {
    // Simple animation logic
}

void draw() {
    wasm96_graphics_background_rgb(0, 0, 50); // Dark blue background

    // Draw a moving rectangle
    static int x = 0;
    x = (x + 1) % 640;
    wasm96_graphics_rect(x, 200, 50, 50);

    // Draw some text
    wasm96_graphics_text_key_str(10, 10, "default", "WASM96 C Example");
}