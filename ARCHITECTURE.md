# Wasm96 Architecture

## Overview

Wasm96 has been decoupled into a platform-agnostic engine and platform-specific frontends. This allows the core logic to be reused across multiple platforms (libretro, desktop, web, etc.) without duplication.

## Crate Structure

```
wasm96/
├── wasm96-engine/          # Platform-agnostic core engine
├── wasm96-libretro/        # Libretro frontend wrapper
├── wasm96-core/            # Legacy monolithic core (deprecated)
├── wasm96-sdk/             # Rust SDK for guest applications
└── example/                # Example guest applications
```

## wasm96-engine

**Purpose**: Platform-agnostic core that handles WASM execution, rendering, audio, and input.

**Key Components**:
- `abi/` - Guest/host ABI definitions and entrypoint resolution
- `av/` - Audio/video rendering (graphics, graphics3d, audio mixing)
- `input/` - Input handling (abstracted through callbacks)
- `runtime/` - Wasmtime-based WASM runtime and imports
- `loader/` - WASM/WAT module loading
- `state/` - Global state management

**Public API**:
```rust
pub struct Engine { ... }

pub trait PlatformCallbacks {
    fn video_refresh(&mut self, framebuffer: &[u32], width: u32, height: u32, stride_pixels: u32);
    fn audio_batch(&mut self, samples: &[i16]);
    fn input_poll(&mut self);
    fn input_button_state(&mut self, port: u32, button: u32) -> bool;
    fn input_key_state(&mut self, key: u32) -> bool;
    fn input_mouse_x(&mut self) -> i32;
    fn input_mouse_y(&mut self) -> i32;
    fn input_mouse_button(&mut self, button: u32) -> bool;
    fn get_current_framebuffer(&mut self) -> usize;
    fn notify_geometry_changed(&mut self, width: u32, height: u32);
}
```

**Usage**:
```rust
let mut engine = Engine::new();
engine.load_game_from_bytes(&wasm_bytes)?;

// In your main loop:
engine.run_frame(&mut your_platform_callbacks);
```

**Dependencies**:
- No libretro dependencies
- No platform-specific code
- Pure Rust with standard graphics/audio libraries

## wasm96-libretro

**Purpose**: Libretro-specific frontend that wraps `wasm96-engine`.

**Key Components**:
- `libretro_glue.rs` - Libretro C API entry points
- `libretro_callbacks.rs` - `PlatformCallbacks` implementation for libretro
- `libretro_env.rs` - Libretro environment helpers (geometry, pixel format)
- `platform.rs` - Platform-specific configuration (GL context, audio rate, alignment)

**How it Works**:
1. Libretro calls `retro_*` functions
2. `LibretroCallbacks` implements `PlatformCallbacks` trait
3. Forwards to `wasm96_engine::Engine`
4. Translates engine callbacks back to libretro C callbacks

**Example Flow**:
```
RetroArch
    ↓ retro_run()
libretro_glue.rs
    ↓ engine.run_frame(&mut callbacks)
wasm96_engine::Engine
    ↓ callbacks.video_refresh(...)
LibretroCallbacks
    ↓ libretro video_refresh_cb(...)
RetroArch
```

## Thread-Local Callback Storage

To allow WASM imports (which use Wasmtime's `Caller<T>`) to access platform callbacks, we use thread-local storage:

```rust
// In Engine::run_frame()
state::set_callbacks(callbacks);  // Store for this frame
// ... run guest code ...
state::clear_callbacks();         // Clean up

// In WASM imports
state::with_callbacks(|cb| {
    cb.input_button_state(port, button)
})
```

**Safety**: The engine guarantees callbacks remain valid for the frame duration.

## Future Frontends

### Desktop Runtime (Planned)

Create `wasm96-desktop/` with:
- Window management (winit, SDL2, etc.)
- OpenGL context
- Audio output (cpal, rodio, etc.)
- Input handling

```rust
struct DesktopCallbacks {
    window: Window,
    audio_device: AudioDevice,
    // ...
}

impl PlatformCallbacks for DesktopCallbacks { ... }

fn main() {
    let mut engine = Engine::new();
    let mut callbacks = DesktopCallbacks::new();
    
    loop {
        engine.run_frame(&mut callbacks);
    }
}
```

### Web Runtime (Planned)

Create `wasm96-web/` with:
- WebAssembly target compilation
- Canvas rendering
- Web Audio API
- Browser input events

## Migration Guide

### From wasm96-core to wasm96-libretro

The API is nearly identical - the main change is that `wasm96-libretro` now depends on `wasm96-engine`:

**Before** (wasm96-core):
```toml
[dependencies]
# All dependencies bundled together
```

**After** (wasm96-libretro):
```toml
[dependencies]
wasm96-engine = { path = "../wasm96-engine" }
libretro-backend = "..."
libretro-sys = "..."
gl = "..."
```

### Building

```bash
# Build libretro core
cargo build --release -p wasm96-libretro

# Core library output
target/release/libwasm96_libretro.so
```

## Design Principles

1. **Separation of Concerns**: Engine logic is independent of rendering backend
2. **Trait-Based Callbacks**: Frontends implement `PlatformCallbacks` trait
3. **Zero-Cost Abstraction**: No runtime overhead for the abstraction layer
4. **Safety**: All unsafe code is documented with safety requirements
5. **Testability**: Engine can be tested without libretro dependencies

## Benefits

- **Code Reuse**: Core logic shared across all platforms
- **Easier Testing**: Engine can be tested independently
- **Multiple Frontends**: Easy to add desktop, web, mobile runtimes
- **Cleaner Dependencies**: Each crate has minimal, focused dependencies
- **Better Maintainability**: Platform-specific code is isolated

## Current Status

- ✅ wasm96-engine: Platform-agnostic core (complete)
- ✅ wasm96-libretro: Libretro frontend (complete, replacing wasm96-core)
- ⏳ wasm96-desktop: Desktop runtime (planned)
- ⏳ wasm96-web: Web runtime (planned)

## Notes

- `wasm96-core` is now deprecated in favor of `wasm96-libretro`
- All examples and guests work unchanged with the new architecture
- The SDK (`wasm96-sdk`) is unchanged - guests are unaffected