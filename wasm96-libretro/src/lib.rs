//! wasm96-libretro: Libretro frontend for wasm96-engine.
//!
//! This crate wraps the platform-agnostic wasm96-engine with libretro-specific
//! bindings and callbacks.

mod gl_renderer;
mod libretro_callbacks;
mod libretro_env;
mod libretro_glue;
mod platform;

pub use libretro_glue::*;
