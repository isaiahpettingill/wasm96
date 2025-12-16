# Clone the repository
git clone https://github.com/isaiahpettingill/wasm96.git
cd wasm96

# Build the libretro core
cargo build --release --package wasm96-core
```

The core library will be at `target/release/libwasm96_core.so` (or equivalent for your platform).

### Writing a Guest
1. Use the Rust SDK (`wasm96-sdk`), Zig SDK (`wasm96-zig-sdk`), or Go SDK (`wasm96-go-sdk`) for ergonomic bindings.
2. Implement the required exports:
   - `setup()`: Initialize your application (e.g., set screen size)
   - `update()`: Update game logic (called once per frame)
   - `draw()`: Issue drawing commands (called once per frame)
3. Compile to WASM32 target
4. Load the .wasm file into the wasm96 core via your libretro frontend

### Running
Load the wasm96 core in your libretro frontend and select a .wasm file as the "game". The core will instantiate the WASM module and start calling the guest's entrypoints.

## SDK

### Rust SDK (`wasm96-sdk/`)
- Handwritten bindings matching the WIT interface
- Safe wrappers around raw `extern "C"` imports
- Entry point: `wasm96_sdk::prelude::*`
- Supports `no_std` for minimal WASM builds
- Optional wee_alloc for custom allocator

### Zig SDK (`wasm96-zig-sdk/`)
- Handwritten bindings matching the WIT interface
- Safe wrappers around raw extern functions
- Entry point: `@import("wasm96")`
- Compiles directly to WASM32

### Go SDK (`wasm96-go-sdk/`)
- Handwritten bindings matching the WIT interface
- Safe wrappers around raw WebAssembly imports
- Entry point: `import "wasm96-go-sdk"`
- Compiles directly to WASM using GOOS=js GOARCH=wasm

## Examples

The `example/` directory contains guest applications:

- `rust-guest/`: Basic hello-world example (Rust)
- `rust-guest-mp-platformer/`: Multiplayer platformer game (Rust)
- `rust-guest-showcase/`: Comprehensive demo of all features (Rust)
- `zig-guest/`: Basic hello-world example (Zig)
- `go-guest/`: Basic hello-world example (Go)

To build a Rust example:
```bash
cargo build --package <example-name> --target wasm32-unknown-unknown
```

To build the Zig example:
```bash
cd example/zig-guest && zig build
```

To build the Go example:
```bash
cd example/go-guest && GOOS=js GOARCH=wasm go build -o go-guest.wasm
```

## Project Structure

```
wasm96/
├── wasm96-core/          # Libretro core implementation
├── wasm96-sdk/           # Handwritten Rust SDK
├── wasm96-zig-sdk/       # Handwritten Zig SDK
├── wasm96-go-sdk/        # Handwritten Go SDK
├── wit/                  # WIT interface definitions
├── example/              # Guest examples
├── Cargo.toml            # Workspace configuration
├── SDKs.md               # Outdated; describes planned multi-language SDKs for old ABI
├── AGENTS.md             # Development guidelines
└── README.md             # This file
```

## Development

### Guidelines
- Follow test-driven development (TDD)
- Ensure all code compiles and passes tests
- See `AGENTS.md` for agent-specific rules

### Contributing
- The ABI is handwritten; update bindings in `wasm96-core/src/abi/mod.rs`, `wasm96-sdk/src/lib.rs`, `wasm96-zig-sdk/src/main.zig`, and `wasm96-go-sdk/wasm96.go` in lockstep
- Update `wit/wasm96.wit` to reflect interface changes
- SDKs.md is outdated and describes a different (upload-based) ABI; it may be removed or updated in the future

### Building Everything
```bash
# Build all workspace members
cargo build --workspace

# Run tests
cargo test --workspace
```

## License

MIT License - see `LICENSE` for details.

## Repository

- **GitHub**: https://github.com/isaiahpettingill/wasm96
- **Author**: isaiahpettingill

## Notes

- The project is in active development; some WIT-defined features (e.g., storage) are not yet implemented in the SDK.
- WAV playback is implemented using the hound library for decoding and mixing.
- QOA playback is implemented using the qoaudio library for decoding and mixing.
- XM playback is implemented using the xmrsplayer library for decoding and playing XM tracker music.
- SDKs.md contains information about planned multi-language SDKs for an older ABI version and may not reflect the current state.