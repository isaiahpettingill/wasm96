// Minimal wasm96 Rust guest example (Immediate Mode).
//
// This crate is meant to be compiled to `wasm32-unknown-unknown` and loaded by `wasm96-core`.
//
// The host calls:
// - `setup()` once at startup.
// - `update()` once per frame.
// - `draw()` once per frame.

use std::sync::Mutex;
use wasm96_sdk::prelude::*;

struct GameState {
    rect_x: i32,
    rect_y: i32,
    vel_x: i32,
    vel_y: i32,
}

static GAME_STATE: Mutex<GameState> = Mutex::new(GameState {
    rect_x: 10,
    rect_y: 10,
    vel_x: 2,
    vel_y: 2,
});

// Keyed resources: the host identifies fonts by string keys.
const FONT_KEY_SPLEEN_16: &str = "font/spleen/16";

#[unsafe(no_mangle)]
pub extern "C" fn setup() {
    // Initialize screen size
    graphics::set_size(320, 240);

    // Register a built-in Spleen font under a stable key.
    // Guests can reuse the same key every run; the host manages the resource table.
    graphics::font_register_spleen(FONT_KEY_SPLEEN_16, 16);

    // Initialize audio (optional)
    audio::init(44100);
}

#[unsafe(no_mangle)]
pub extern "C" fn update() {
    // Update game state
    {
        let mut state = GAME_STATE.lock().unwrap();
        state.rect_x += state.vel_x;
        state.rect_y += state.vel_y;

        if state.rect_x <= 0 || state.rect_x >= 290 {
            state.vel_x = -state.vel_x;
        }
        if state.rect_y <= 0 || state.rect_y >= 210 {
            state.vel_y = -state.vel_y;
        }
    }

    // NOTE:
    // The core is responsible for padding/handling audio when the guest produces too little.
    // Guests shouldn't need to push silence just to keep the runtime happy.
}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    // 1. Clear background
    graphics::background(20, 20, 40);
    graphics::text_key(100, 100, FONT_KEY_SPLEEN_16, "Hello");

    // 2. Draw moving rectangle
    graphics::set_color(255, 100, 100, 255);
    {
        let state = GAME_STATE.lock().unwrap();
        graphics::rect(state.rect_x, state.rect_y, 30, 30);

        // Draw outline
        graphics::set_color(255, 255, 255, 255);
        graphics::rect_outline(state.rect_x, state.rect_y, 30, 30);
    }

    // 3. Draw circle at mouse position
    let mx = input::get_mouse_x();
    let my = input::get_mouse_y();

    if input::is_mouse_down(0) {
        graphics::set_color(255, 255, 0, 255); // Yellow if clicked
    } else {
        graphics::set_color(100, 255, 100, 255); // Green otherwise
    }
    graphics::circle(mx, my, 15);

    // Draw crosshair lines
    graphics::set_color(255, 255, 255, 100);
    graphics::line(mx - 20, my, mx + 20, my);
    graphics::line(mx, my - 20, mx, my + 20);

    // 4. Check joypad input
    if input::is_button_down(0, Button::A) {
        graphics::set_color(0, 0, 255, 255);
        graphics::rect(280, 200, 20, 20);
    }
}
