#![no_std]

// Text rendering example for wasm96.
//
// This example demonstrates how to render text using different fonts,
// colors, and positions. It shows both built-in Spleen fonts and TTF fonts
// (including Noto Color Emoji for full emoji support), special characters,
// and a simple menu interface.

use wasm96_sdk::prelude::*;

// Load TTF fonts from files
const NERD_FONT_DATA: &[u8] = include_bytes!("assets/3270NerdFont-Regular.ttf");
const NOTO_EMOJI_DATA: &[u8] = include_bytes!("assets/NotoColorEmoji-Regular.ttf");

// Font keys for keyed resources
const FONT_SPLEEN_8: &str = "font/spleen/8";
const FONT_SPLEEN_16: &str = "font/spleen/16";
const FONT_SPLEEN_24: &str = "font/spleen/24";
const FONT_SPLEEN_32: &str = "font/spleen/32";
const FONT_SPLEEN_64: &str = "font/spleen/64";
const FONT_NERD: &str = "font/ttf/3270-nerd";
const FONT_NOTO_EMOJI: &str = "font/ttf/noto-emoji";

static mut FRAME: u32 = 0;
static mut MENU_SELECTION: u32 = 0;
static mut LAST_UP_DOWN: bool = false;
static mut LAST_DOWN_DOWN: bool = false;

#[unsafe(no_mangle)]
pub extern "C" fn setup() {
    // Set screen size (higher resolution)
    graphics::set_size(960, 720);

    // Register built-in Spleen fonts of different sizes
    // Supported sizes in wasm96 core: 8, 16, 24, 32, 64
    graphics::font_register_spleen(FONT_SPLEEN_8, 8);
    graphics::font_register_spleen(FONT_SPLEEN_16, 16);
    graphics::font_register_spleen(FONT_SPLEEN_24, 24);
    graphics::font_register_spleen(FONT_SPLEEN_32, 32);
    graphics::font_register_spleen(FONT_SPLEEN_64, 64);

    // Register TTF fonts
    let _nerd_registered = graphics::font_register_ttf(FONT_NERD, NERD_FONT_DATA);
    let _noto_registered = graphics::font_register_ttf(FONT_NOTO_EMOJI, NOTO_EMOJI_DATA);
}

#[unsafe(no_mangle)]
pub extern "C" fn update() {
    unsafe {
        FRAME += 1;

        // Menu navigation with debouncing
        let up_down = input::is_button_down(0, Button::Up);
        let down_down = input::is_button_down(0, Button::Down);

        if up_down && !LAST_UP_DOWN {
            if MENU_SELECTION > 0 {
                MENU_SELECTION -= 1;
            }
        }
        if down_down && !LAST_DOWN_DOWN {
            if MENU_SELECTION < 3 {
                MENU_SELECTION += 1;
            }
        }
        LAST_UP_DOWN = up_down;
        LAST_DOWN_DOWN = down_down;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    // Clear background
    graphics::background(30, 30, 50);

    // Draw title
    graphics::set_color(255, 255, 255, 255);
    graphics::text_key(30, 30, FONT_SPLEEN_32, "Text Rendering Example");

    // Draw menu
    graphics::set_color(255, 255, 255, 255);
    graphics::text_key(30, 90, FONT_SPLEEN_24, "Menu:");
    graphics::text_key(30, 120, FONT_SPLEEN_24, "1. Spleen Fonts");
    graphics::text_key(30, 150, FONT_SPLEEN_24, "2. TTF Font");
    graphics::text_key(30, 180, FONT_SPLEEN_24, "3. Special Characters");
    graphics::text_key(30, 210, FONT_SPLEEN_24, "4. Unicode Symbols");

    // Draw selection indicator
    let selection = unsafe { MENU_SELECTION };
    let y_pos = 120 + (selection as i32 * 30);
    graphics::set_color(255, 255, 0, 255);
    graphics::text_key(15, y_pos, FONT_SPLEEN_24, ">");

    // Draw content based on selection
    match selection {
        0 => draw_spleen_fonts(),
        1 => draw_ttf_font(),
        2 => draw_special_chars(),
        3 => draw_emojis(),
        _ => {}
    }
}

fn draw_spleen_fonts() {
    // Draw different font sizes
    graphics::set_color(200, 200, 255, 255);
    graphics::text_key(375, 120, FONT_SPLEEN_16, "Small text (16px Spleen)");
    graphics::text_key(375, 150, FONT_SPLEEN_24, "Medium text (24px Spleen)");
    graphics::text_key(375, 195, FONT_SPLEEN_32, "Large text (32px Spleen)");
    graphics::text_key(375, 240, FONT_SPLEEN_64, "Huge text (64px Spleen)");

    // Draw colored text
    graphics::set_color(255, 100, 100, 255);
    graphics::text_key(375, 360, FONT_SPLEEN_24, "Red text");

    graphics::set_color(100, 255, 100, 255);
    graphics::text_key(375, 390, FONT_SPLEEN_24, "Green text");

    graphics::set_color(100, 100, 255, 255);
    graphics::text_key(375, 420, FONT_SPLEEN_24, "Blue text");

    // Draw text with alpha
    graphics::set_color(255, 255, 255, 128);
    graphics::text_key(375, 450, FONT_SPLEEN_24, "Semi-transparent text");
}

fn draw_ttf_font() {
    graphics::set_color(255, 215, 0, 255);
    graphics::text_key(375, 160, FONT_NERD, "TTF Font: 3270 Nerd Font");
    graphics::text_key(375, 220, FONT_NERD, "Monospace programming font");
    graphics::text_key(375, 280, FONT_NERD, "Includes programming ligatures");
    graphics::text_key(375, 340, FONT_NERD, "Great for code and terminals");
}

fn draw_special_chars() {
    graphics::set_color(255, 255, 255, 255);
    graphics::text_key(375, 120, FONT_SPLEEN_24, "Special Characters:");
    graphics::text_key(375, 150, FONT_SPLEEN_24, "Arrows: ← ↑ → ↓ ↔ ↕");
    graphics::text_key(375, 180, FONT_SPLEEN_24, "Math: ± × ÷ √ ∞ ≈ ≠");
    graphics::text_key(375, 210, FONT_SPLEEN_24, "Symbols: © ® ™ ° § ¶");
    graphics::text_key(375, 240, FONT_SPLEEN_24, "Currency: $ € £ ¥ ¢");

    // Using TTF for better symbol rendering
    graphics::set_color(255, 165, 0, 255);
    graphics::text_key(375, 285, FONT_NERD, "♠ ♥ ♦ ♣");
    graphics::text_key(375, 330, FONT_NERD, "α β γ δ ε");
}

fn draw_emojis() {
    graphics::set_color(255, 255, 255, 255);
    graphics::text_key(375, 120, FONT_SPLEEN_24, "Unicode Symbols:");
    graphics::text_key(375, 150, FONT_SPLEEN_24, "TTF fonts support monochrome");
    graphics::text_key(375, 180, FONT_SPLEEN_24, "glyphs, not color emojis");

    // Test basic ASCII with TTF font
    graphics::set_color(255, 215, 0, 255);
    graphics::text_key(375, 225, FONT_NERD, "ABC123 Test");

    // Unicode symbols that work (monochrome)
    graphics::text_key(375, 270, FONT_NERD, "Playing cards: ♠ ♥ ♦ ♣");
    graphics::text_key(375, 315, FONT_NERD, "Arrows: ← ↑ → ↓ ↔ ↕");
    graphics::text_key(375, 360, FONT_NERD, "Math: ± × ÷ √ ∞ ≈ ≠");
    graphics::text_key(375, 405, FONT_NERD, "Currency: $ € £ ¥ ¢");

    // Try emoji with Noto Color Emoji (will show tofu - not supported)
    graphics::set_color(255, 255, 0, 255);
    graphics::text_key(375, 450, FONT_NOTO_EMOJI, "Emoji attempt: 😀 🚀 ⭐");

    // Color emojis don't work - limitation explanation
    graphics::set_color(255, 255, 255, 255);
    graphics::text_key(
        375,
        495,
        FONT_SPLEEN_24,
        "Color emojis (😀🚀⭐) don't render",
    );
    graphics::text_key(375, 525, FONT_SPLEEN_24, "because wasm96 uses monochrome");
    graphics::text_key(375, 555, FONT_SPLEEN_24, "font rendering only");
}
