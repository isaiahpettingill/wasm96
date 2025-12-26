#include "wasm96.hpp"

extern "C" void setup() {
    wasm96::Graphics::setSize(640, 480);
    wasm96::Graphics::setColor(255, 255, 255, 255);
}

extern "C" void update() {
    // Simple animation logic
}

extern "C" void draw() {
    wasm96::Graphics::background(0, 0, 50); // Dark blue background

    // Draw a moving rectangle
    static int x = 0;
    x = (x + 1) % 640;
    wasm96::Graphics::rect(x, 200, 50, 50);

    // Draw some text
    wasm96::Graphics::textKey(10, 10, "default", "WASM96 C++ Example");
}