# wasm96-sdk

A Rust SDK for building WebAssembly applications that run under the [wasm96](https://github.com/isaiahpettingill/wasm96) libretro core.

## Overview

`wasm96-sdk` provides safe, ergonomic bindings to the wasm96 guest ABI, allowing you to write games and applications in Rust that compile to WebAssembly and run in libretro frontends like RetroArch.

Key features:
- **Immediate Mode Graphics**: Issue drawing commands (rects, circles, text, etc.) without managing framebuffers.
- **Audio Playback**: Play WAV, QOA, and XM files with host-mixed channels.
- **Input Handling**: Query joypad, keyboard, and mouse state.
- **Resource Management**: Register and draw images (PNG, GIF, SVG), fonts, and other assets by key.
- **Storage**: Save/load persistent data.
- **System Utilities**: Logging and timing.

### ABI model (mental model)

- The **host** (wasm96-core) owns the framebuffer and all rendering backends.
- The **guest** (your `.wasm`) issues drawing/audio/input calls.
- Your guest exports:
  - `setup()` (required)
  - `update()` (optional)
  - `draw()` (optional)

## Usage

Add this to your `Cargo.toml`:

```toml
[package]
name = "my-wasm96-app"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm96-sdk = "0.1.0"
```

In your `src/lib.rs`:

```rust
use wasm96_sdk::prelude::*;

// Required: Called once on startup
#[no_mangle]
pub extern "C" fn setup() {
    graphics::set_size(640, 480);
    // Register assets, initialize state, etc.
}

// Optional: Called once per frame to update logic
#[no_mangle]
pub extern "C" fn update() {
    // Handle input, update game state
}

// Optional: Called once per frame to draw
#[no_mangle]
pub extern "C" fn draw() {
    graphics::background(0, 0, 0); // Black background
    graphics::set_color(255, 255, 255, 255); // White
    graphics::rect(100, 100, 100, 100); // Draw a rectangle
}
```

Build for WebAssembly:

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

The output `.wasm` file can be loaded into the wasm96 core in RetroArch.

## Fonts & text (immense documentation)

Text rendering is one of the easiest places to get surprised by ABI and lifecycle details. This section is intentionally long.

### Keyed font model (no handles)

Fonts in wasm96 are **keyed resources**:

- You register a font under a *key*.
- You draw/measure text by providing the same key.
- No numeric font handles are returned to the guest.

In Rust, the SDK exposes string-key APIs (like `"ui"` or `"debug"`), and the SDK hashes those strings to the underlying `u64` key.

### Host fallback behavior (very important)

The host (wasm96-core) implements a **fallback** if you try to measure/draw text with a key that has not been registered:

- `graphics::text_key(...)` falls back to **built-in Spleen** at **size 16**
- `graphics::text_measure_key(...)` uses the exact same fallback

This means:
- text “just works” even if you never registered a font
- but layout can become unstable if you assumed a different font/size

Best practice: **register fonts in `setup()` and always measure/draw using those same keys**.

### Which font source should you use?

You have three practical options:

1) Built-in **Spleen** (bundled pixel font family)
- API: `graphics::font_register_spleen(key, size)`
- Great for retro UIs, debug overlays, and “it should always work” text.
- Supported sizes are finite (host-defined); common sizes are 8/16/24/32/64.

2) **BDF** (custom bitmap fonts)
- API: `graphics::font_register_bdf(key, bdf_bytes)`
- Best for pixel-perfect fonts you ship.
- Deterministic bitmap metrics.

3) **TTF/OTF** (custom scalable fonts)
- API: `graphics::font_register_ttf(key, font_bytes)`
- Best when you want scalable typography (menus, titles, readability).

### What exactly is a “key”?

A key is an arbitrary string you choose, such as:
- `"ui"`
- `"debug"`
- `"title_32"`

You should treat keys as part of your app’s “asset namespace”. They should be:
- stable (don’t generate random keys)
- consistent across measure/draw calls
- registered once, reused forever (unless you intentionally unload)

### Recommended usage pattern

1) In `setup()`:
- set a resolution
- register fonts under stable keys

2) For layout:
- call `graphics::text_measure_key(font_key, text)`
- compute positions

3) For drawing:
- set a color
- call `graphics::text_key(x, y, font_key, text)`

Example (built-in Spleen):

```rust
use wasm96_sdk::graphics;

#[no_mangle]
pub extern "C" fn setup() {
    graphics::set_size(320, 240);
    // Register a predictable font key for UI.
    graphics::font_register_spleen("ui", 16);
}

#[no_mangle]
pub extern "C" fn draw() {
    graphics::background(0, 0, 0);
    graphics::set_color(255, 255, 255, 255);

    let msg = "Hello wasm96";
    let size = graphics::text_measure_key("ui", msg);

    let x = (320i32 - size.width as i32) / 2;
    let y = 20;
    graphics::text_key(x, y, "ui", msg);
}
```

Example (custom TTF/OTF):

```rust
use wasm96_sdk::graphics;

static UI_TTF: &[u8] = include_bytes!("../assets/YourFont.ttf");

#[no_mangle]
pub extern "C" fn setup() {
    graphics::set_size(640, 480);

    // Choose a stable key and register once.
    let ok = graphics::font_register_ttf("ui", UI_TTF);
    if !ok {
        // If you want, log/fallback in your app.
        // The host will still be able to render via fallback Spleen 16 for unknown keys.
    }
}

#[no_mangle]
pub extern "C" fn draw() {
    graphics::background(16, 16, 16);
    graphics::set_color(240, 240, 240, 255);
    graphics::text_key(20, 20, "ui", "TTF text");
}
```

### Measuring vs drawing: keep them consistent

Because the host fallback is defined, if you accidentally measure with `"ui"` but draw with `"UI"` (different key), you can end up measuring one font and drawing another.

Best practice:
- define constants for keys
- treat keys as case-sensitive

### Memory / lifetime rules (guest-side)

All text/font registration APIs that take byte pointers behave like “read immediately”:

- When you call `font_register_*`, the host reads the font bytes during the call and parses/copies them host-side.
- When you call `text_key` / `text_measure_key`, the host reads the UTF-8 text bytes during the call.

So:
- it is safe to pass `&str` and `&[u8]` that live only for the duration of the call
- you do not need to keep the byte buffers alive after registration completes

### Unregistering fonts

If you need to reclaim host-side resources (rare for typical games), you can unregister:

- `graphics::font_unregister(key)`

After unregistering:
- the key no longer maps to the registered font
- drawing/measuring using that key will hit the host fallback (Spleen 16) unless you register again

### Troubleshooting font issues

If text is not drawing as expected:

- Verify you call `graphics::set_color(r,g,b,a)` before drawing text.
- Verify your font key matches exactly between registration and draw/measure.
- If using `font_register_spleen`, verify you are using a supported size.
- If using `font_register_ttf`, verify the font bytes are valid and included correctly.
- Remember: the host fallback may be masking registration failures by still rendering “something”.

## Features

- `std` (default): Enables standard library features for convenience.
- `wee_alloc`: Optional global allocator for `wasm32-unknown-unknown` targets.

## Examples

See the [wasm96 repository](https://github.com/isaiahpettingill/wasm96/tree/main/example) for complete examples:

- `rust-guest/`: Basic hello-world
- `rust-guest-showcase/`: Comprehensive feature demo

## Documentation

Generate and view docs locally:

```bash
cargo doc --open
```

## ABI Compatibility

This SDK targets the wasm96 ABI as defined in the [WIT interface](https://github.com/isaiahpettingill/wasm96/blob/main/wit/wasm96.wit). Ensure your wasm96-core version matches the SDK version for compatibility.

## License

MIT License - see [LICENSE](https://github.com/isaiahpettingill/wasm96/blob/main/LICENSE) for details.

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/isaiahpettingill/wasm96) for development guidelines.
