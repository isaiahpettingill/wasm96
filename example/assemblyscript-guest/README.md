# wasm96 AssemblyScript guest example: Generated Flappy

This is a minimal **AssemblyScript guest** WebAssembly module intended to run inside the `wasm96` libretro core.

It exports the standard entrypoints expected by the core:

- `setup()` (called once at startup)
- `update()` (called once per frame)
- `draw()` (called once per frame)

This guest does **manual imports** of the wasm96 ABI functions using AssemblyScript’s `@external("env", "...")` declarations (no SDK wrapper).

---

## Controls

Gamepad (port 0):

- `A` / `B` / `Up`: flap
- `Start`: restart

---

## Build

From `wasm96/`:

```sh
cd example/assemblyscript-guest
npm install
npm run build
```

Outputs:

- `example/assemblyscript-guest/flappy.wasm` (release)

Debug build:

```sh
npm run build:debug
```

Outputs:

- `example/assemblyscript-guest/flappy.debug.wasm`

---

## Run (via just + RetroArch)

From `wasm96/`:

```sh
just run-assemblyscript-flappy
```

This builds the guest and then runs the resulting `flappy.wasm` through the wasm96 core using the existing `just run ...` workflow.

---

## Notes (ABI + text)

- wasm96 text rendering uses `ptr + len` byte slices. AssemblyScript `string` is UTF-16, so the example writes ASCII into a scratch `Uint8Array` and uses `wasm96_graphics_text_key(...)` directly.
- The font key is hashed with a small FNV-1a 64-bit routine to match the SDK behavior (e.g. for `"spleen"`).