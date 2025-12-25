# wasm96 Zig guest 3D example

This is a minimal **Zig guest** WebAssembly module intended to run inside the `wasm96` libretro core.

It exports the required entrypoints:

- `setup` (called once on startup)
- `update` (called once per frame for logic)
- `draw` (called once per frame for rendering)

It uses the **handwritten** Zig SDK located at `wasm96/wasm96-zig-sdk`.

---

## Build (wasm32)

From this directory:

```sh
zig build
```

The output `.wasm` will be at:

```text
zig-out/bin/zig-guest-3d.wasm
```

---

## Notes

- The wasm96 ABI uses **u32 offsets into guest linear memory** for buffers.
- The host may reject ABI mismatches; ensure the SDK’s ABI matches the core.
- If you see framebuffer/audio requests failing (returning `0` / `null`), the host core may still be stubbing allocation APIs; the guest example can still compile, but you won’t get video/audio until the core implements allocation.

---

## Typical usage

1. Build the `.wasm` as above.
2. Load the resulting `.wasm` in your libretro frontend using the `wasm96` core (as you would load a ROM).