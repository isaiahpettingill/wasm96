# wasm96 Rust guest text rendering example

This is a **Rust guest** WebAssembly module demonstrating text rendering features in the `wasm96` libretro core at higher resolution (1280x960) for improved visibility.

It exports the required entrypoints:

- `setup()`: Initializes the application, registers fonts, and sets screen size.
- `update()`: Updates game logic, handles menu navigation with controller input.
- `draw()`: Renders a menu interface and various text examples based on selection.

It uses the **handwritten** Rust SDK located at `wasm96/wasm96-sdk`.

---

## Features Demonstrated

- Higher resolution display (1280x960) with larger text rendering
- Menu interface with controller navigation (Up/Down buttons)
- Registering built-in Spleen fonts of various sizes (8px to 64px)
- Registering and using TTF fonts for advanced text rendering
- Rendering text with various colors and transparency
- Special characters and symbols (arrows, math, currency)
- Rendering Unicode symbols with TTF fonts (monochrome only)
- Demonstrating font rendering limitations (color emojis not supported)
- Dynamic content switching based on menu selection

---

## Build (wasm32)

From this directory:

```sh
cargo build --release --target wasm32-unknown-unknown
```

Use the D-pad Up/Down buttons to navigate the menu and see different text rendering examples.

The output `.wasm` will be at:

```text
target/wasm32-unknown-unknown/release/rust_guest_text.wasm
```

---

## Notes

- The example uses a higher resolution screen (1280x960) compared to typical wasm96 applications (640x480) for better text visibility.
- The wasm96 ABI uses **u64 keys** for resources like fonts (automatically hashed from strings).
- Text measurement returns a `TextSize` struct with `width` and `height` fields.
- Colors are set using `graphics::set_color(r, g, b, b, a)` before drawing text.
- Fonts must be registered in `setup()` before use in `draw()`.
- TTF fonts support Unicode symbols but not color emojis (monochrome rendering only).
- Menu navigation uses debounced input (press Up/Down to change selection).

---

## Typical usage

1. Build the `.wasm` as above.
2. Load the resulting `.wasm` in your libretro frontend using the `wasm96` core.