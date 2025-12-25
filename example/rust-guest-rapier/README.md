# Rapier3D Physics Example

This example demonstrates how to use the [Rapier3D](https://rapier.rs/) physics engine within a wasm96 guest.

It features a simple game where you can throw spheres at stacks of cubes to knock them down.

## Controls

| Button | Action |
| --- | --- |
| **D-Pad** | Pan camera target (X/Z plane) |
| **X / Y** | Zoom camera in / out |
| **A** | Fire a sphere at the crosshair |
| **B** | Drop a cube from the sky |

## Building

To build this example, run the following command from the workspace root:

```bash
cargo build --package rust-guest-rapier --target wasm32-unknown-unknown
```

The resulting WASM file will be located at:
`target/wasm32-unknown-unknown/debug/rust_guest_rapier.wasm`

## Implementation Details

- **Physics**: Uses `rapier3d` for rigid body simulation.
- **Rendering**: Uses `wasm96::graphics` 3D API for rendering meshes.
- **State**: Uses `static mut` for global state (physics pipeline, bodies, etc.), which is standard for simple WASM guests that don't have a complex runtime.
- **Meshes**: Procedurally generates cube and sphere meshes at startup.