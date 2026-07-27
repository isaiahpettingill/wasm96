# wasm96

`wasm96` is a libretro fantasy-console core for 32-bit WebAssembly cartridges.
It mirrors the `risc96` video, audio, controller, timing, SRAM, and debug ABI semantics while using [`wasmz`](https://github.com/Ray-D-Song/wasmz) as the embedded engine.

## Runtime Limits

- Cartridge size: 128 MiB maximum
- Guest RAM: 128 MiB maximum
- SRAM: 64 MiB
- Framebuffer: `0x00RRGGBB`, default `320x224`, maximum `1024x1024`
- Audio: signed 16-bit stereo PCM, 48 kHz, 800 stereo frames per video frame
- Controllers: 4 ports, 12 buttons, 2-bit pressure levels packed into 3 bytes

## Cartridge Model

Cartridges are reactor-style WebAssembly modules. The core calls an exported `wasm96_update` function once per libretro video frame.

Imported host functions live in module `env`:

- `get_framebuffer() -> i32`
- `get_audiobuffer() -> i32`
- `controller_1() -> i32`
- `controller_2() -> i32`
- `controller_3() -> i32`
- `controller_4() -> i32`
- `present()`
- `set_resolution(i32 width, i32 height) -> i32`
- `sram_size() -> i32`
- `sram_read(i32 offset, i32 dst, i32 len) -> i32`
- `sram_write(i32 offset, i32 src, i32 len) -> i32`
- `controller_count() -> i32`
- `controller_info(i32 port) -> i32`
- `time_ms() -> i32`
- `delta_ms() -> i32`
- `debug_log(i32 src, i32 len) -> i32`
- `debug_trace(i32 a, i32 b, i32 c)`
- `debug_mem_read(i32 src, i32 dst, i32 len) -> i32`
- `debug_mem_write(i32 dst, i32 src, i32 len) -> i32`
- `exit(i32 code)`
- `exit_group(i32 code)`

## Build

```sh
git submodule update --init --recursive
just libretro-build
just cartridge-c-build
```

The libretro core is emitted under `zig-out/lib/`.

## Save States

Save states serialize frame-boundary runtime state: guest linear memory, globals, table contents, data/element drop flags, framebuffer/audio snapshots, SRAM, timing, controller allocation pointers, and host allocation addresses.
