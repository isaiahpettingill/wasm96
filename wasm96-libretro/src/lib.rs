//! wasm96-libretro: Libretro frontend for wasm96-engine.
//!
//! This crate wraps the platform-agnostic wasm96-engine with libretro-specific
//! bindings and callbacks.

#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

// NOTE: This crate supports wasm32 builds for RetroArch Web.
// The wasm32 build still exports the libretro C ABI entry points, and relies on
// `RETRO_ENVIRONMENT_SET_HW_RENDER` to receive a WebGL-backed context from the frontend.

// Native-only GL compositor (wasm32 uses a wgpu-based backend instead).
#[cfg(not(target_arch = "wasm32"))]
mod gl_renderer;

mod libretro_callbacks;
mod libretro_env;
mod libretro_glue;
mod platform;

pub use libretro_glue::*;
