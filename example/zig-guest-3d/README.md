# wasm96 Zig guest 3D example (rolling sphere game)

## 3D texturing validation

This guest also binds a **ground texture** (PNG preferred, JPEG fallback) to the ground plane using the keyed-image APIs (`pngRegister` / `jpegRegister`) plus `meshSetTexture(...)` so you can verify that **3D UV texturing** is working end-to-end.

This example is a **Zig guest** WebAssembly module intended to run inside the `wasm96` libretro core.

Instead of a static cube demo, it’s a small 3D “rolling sphere on a plane” game loop:

- You control a **sphere** that accelerates and rolls around on a **flat plane**.
- The scene uses the **models and textures included in this directory** (e.g. the OBJ + MTL + `Textures/`).

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

- D-Pad: rotate camera pitch
  - Up/Down: pitch
- L1/R1: yaw (rotate camera left/right)
- A: accelerate forward (relative to camera yaw)
- B: brake
- Start: reset
- Y: jump

Notes:
- Controls are implemented via `wasm96.input.isButtonDown(...)` (joypad), not keyboard.

---

## Assets

This example includes an OBJ scene and associated materials/textures:

- `src/Castelia City.obj`
- `src/Castelia City.mtl`
- `src/Textures/` (diffuse textures referenced by the MTL)

Keep these with the project when building/running so the guest can embed or reference them as intended.

---

## Notes

- The wasm96 ABI uses **u32 offsets into guest linear memory** for buffers.
- The host may reject ABI mismatches; ensure the SDK’s ABI matches the core.
- If you see framebuffer/audio requests failing (returning `0` / `null`), the host core may still be stubbing allocation APIs; the guest example can still compile, but you won’t get video/audio until the core implements allocation.

---

## Typical usage

1. Build the `.wasm` as above.
2. Load the resulting `.wasm` in your libretro frontend using the `wasm96` core (as you would load a ROM).