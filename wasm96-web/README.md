# wasm96-web

A web-based player for [wasm96](https://github.com/isaiahjp/wasm96) games, built with Rust and Yew.

This crate provides a WebAssembly frontend that runs in the browser, allowing users to load and play `.w96` or `.wasm` cartridges.

## Features

- **Rendering**: Supports both software rendering (Canvas 2D) and hardware acceleration (WebGPU/WebGL via `wgpu`).
- **Audio**: Low-latency audio playback using `AudioContext`.
- **Input**:
  - Keyboard support with remappable controls.
  - Gamepad support via the Web Gamepad API.
  - Mouse support.
- **Cartridge Loading**: Load games directly from your local file system via a file picker.

## Building and Running

This project uses `trunk` to build and bundle the Rust WASM application.

1. **Install Trunk**:
   ```bash
   cargo install --locked trunk
   ```

2. **Add WASM Target**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Run Development Server**:
   ```bash
   # From the wasm96-web directory
   trunk serve
   ```
   This will start a local server (usually at `http://127.0.0.1:8080`) and rebuild on file changes.

## Usage

1. Open the player in your browser.
2. Click the file input to select a `.wasm` or `.w96` cartridge file.
3. The game should start automatically.
4. **Controls**:
   - Default mapping:
     - Arrow Keys: D-Pad
     - Z: B Button
     - X: A Button
     - A: Y Button
     - S: X Button
     - Enter: Start
     - Space: Select
   - Click the "Remap" buttons below the game view to change key bindings.

## Architecture

- **`src/lib.rs`**: Main Yew application component handling UI, events, and the game loop.
- **`src/platform/`**:
  - **`audio.rs`**: Web Audio API implementation.
  - **`input.rs`**: Input handling (Keyboard, Mouse, Gamepad) and mapping logic.
  - **`mod.rs`**: The `WebPlatform` struct which implements `PlatformCallbacks` for the `wasm96-engine`.

## License

MIT