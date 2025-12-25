# WAT Guest Example

This is a simple 2D example written in WebAssembly Text Format (WAT). It demonstrates a controllable rectangle that can be moved and colored using input.

## Features

- Sets up a 320x240 screen
- Draws a blue background
- Displays a 30x30 rectangle controllable with input

## Controls

- D-pad: Move the rectangle around the screen
- A button: Change color to green
- B button: Change color to blue
- Default color: Red

## Building

To compile the WAT file to WebAssembly:

```bash
wat2wasm main.wat -o wat-guest.wasm
```

## Running

Load the `wat-guest.wasm` file into the wasm96 libretro core in your libretro frontend (e.g., RetroArch).

Or use the justfile recipe:

```bash
just run-wat-guest
```

This will compile and run the example.