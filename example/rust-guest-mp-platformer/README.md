# wasm96 Rust guest example: 2-player co-op platformer

This is a simple **multiplayer (2P) co-op platformer** written as a Rust **guest** WebAssembly module intended to run inside the `wasm96` libretro core.

It demonstrates:

- Two simultaneous players using **two controller ports** (`port 0` and `port 1`)
- Basic platformer physics (gravity, jump, ground detection)
- Simple collision against a small set of static platforms
- A co-op objective: **both players must reach the goal zone together**

---

## Controls (Controller)

Applies to **both** players:

- **D-Pad Left / Right**: move
- **A**: jump
- **Start**: respawn at checkpoint (either player can trigger)

Players are read from:

- Player 1: **port 0**
- Player 2: **port 1**

> Note: You’ll need your libretro frontend to have two pads connected/assigned for true co-op control.

---

## Objective / Gameplay

- Move through the level and reach the flag area.
- The win condition triggers only when **both** players are inside the goal zone at the same time.
- There is a checkpoint area roughly mid-level; reaching it updates the respawn point.

---

## Build (wasm32)

From this directory:

```sh
cargo build --release --target wasm32-unknown-unknown
```

The output `.wasm` will be at:

```text
target/wasm32-unknown-unknown/release/rust_guest_mp_platformer.wasm
```

---

## Running

1. Build the `.wasm` (see above).
2. Load the resulting `.wasm` in your libretro frontend using the `wasm96` core (as you would load a ROM).
3. Ensure you have two controllers mapped to port 0 and port 1.

---

## Implementation notes

- The guest exports the standard entrypoints expected by the core:
  - `setup()` (once)
  - `update()` (once per frame)
  - `draw()` (once per frame)
- Audio: the example pushes silence each frame to satisfy typical libretro audio expectations.
- There is no text rendering API used here; win feedback is shown via simple shapes.