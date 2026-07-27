# wasm96 C Cartridge

This template builds a freestanding `wasm32` cartridge that exports `wasm96_update`.
The host calls `wasm96_update` once per libretro frame.

```sh
make
```

Run the resulting `cartridge.wasm` with the `wasm96_libretro` core in RetroArch.
