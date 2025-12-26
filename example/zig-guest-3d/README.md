# wasm96 Zig guest 3D example (grid + rolling sphere + bird props)

This example is a **Zig guest** WebAssembly module intended to run inside the `wasm96` libretro core.

It’s a small 3D sandbox:

- You control a **sphere** that accelerates and rolls around on a **flat ground**.
- The ground is a **procedural grid** (major/minor lines + highlighted axes) so you can immediately judge scale, movement, and camera.
- Two lightweight **bird OBJ** models are loaded and rendered as props near the origin.

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

## Controls (joypad)

- D-Pad Up/Down: camera pitch
- L1/R1: camera yaw (rotate left/right)
- A: accelerate forward (relative to camera yaw)
- B: brake
- Y: jump
- Start: reset

Notes:
- Controls are implemented via `wasm96.input.isButtonDown(...)` (joypad), not keyboard.

---

## Assets

Bird OBJ models (rendered as props):

- `src/12248_Bird_v1_L2.obj`
- `src/12249_Bird_v1_L2.obj`

Notes:
- These OBJ files reference an accompanying `.mtl`, which in turn references a diffuse texture (`*_diff.jpg`) via `map_Kd`/`map_Ka`.
- The current example binds the diffuse textures manually (register JPEG bytes, then `meshSetTexture(mesh_key, image_key)`), rather than relying on automatic MTL parsing/loading.
- If `meshCreateObj(...)` is stubbed/unsupported in your current core build, the birds won’t render (but you can still roll on the grid).

---

## Notes

- The wasm96 ABI uses **u32 offsets into guest linear memory** for buffers.
- The host may reject ABI mismatches; ensure the SDK’s ABI matches the core.
- If you see framebuffer/audio requests failing (returning `0` / `null`), the host core may still be stubbing allocation APIs; the guest example can still compile, but you won’t get video/audio until the core implements allocation.

---

## Typical usage

1. Build the `.wasm` as above.
2. Load the resulting `.wasm` in your libretro frontend using the `wasm96` core (as you would load a ROM).