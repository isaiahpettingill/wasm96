//! wasm96-core: a libretro core that loads and runs a guest WASM/WAT module.
//!
//! This crate implements an **Immediate Mode ABI**:
//! - The host owns the framebuffer and handles rendering.
//! - The guest issues drawing commands.
//! - The guest exports `setup`, and may export `update`/`draw`.
//! - WASI-style guests are supported: if `draw` is missing, `_start` or `main` may be used.
//!
//! The ABI surface is defined in `crate::abi` and mirrored by `wasm96-sdk`.
//!
//! Runtime backend: Wasmtime (see `crate::runtime`).

mod abi;
mod av;
mod input;
mod libretro_glue;
mod loader;
mod runtime;
mod state;

use crate::abi::GuestEntrypoints;

/// The libretro core instance.
#[derive(Default)]
pub struct Wasm96Core {
    rt: Option<runtime::WasmtimeRuntime>,
    module: Option<wasmtime::Module>,
    instance: Option<wasmtime::Instance>,
    entrypoints: Option<GuestEntrypoints>,
    setup_called: bool,
}

impl Wasm96Core {
    fn instantiate_with_details(&mut self) -> Result<(), anyhow::Error> {
        self.ensure_runtime()
            .map_err(|_| anyhow::anyhow!("Failed to initialize runtime"))?;

        let rt = self
            .rt
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Runtime missing after init"))?;
        let module = self
            .module
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Guest module missing (compile step did not set it)"))?;

        let (instance, entrypoints) = rt
            .instantiate(module)
            .map_err(|e| anyhow::anyhow!("Wasmtime instantiate failed: {e:?}"))?;

        self.instance = Some(instance);
        self.entrypoints = Some(entrypoints);
        Ok(())
    }

    fn ensure_runtime(&mut self) -> Result<(), ()> {
        if self.rt.is_some() {
            return Ok(());
        }

        let mut rt = runtime::WasmtimeRuntime::new().map_err(|_| ())?;
        rt.define_imports().map_err(|_| ())?;
        self.rt = Some(rt);
        Ok(())
    }

    fn call_guest_setup(&mut self) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };

        // Wasmtime's `Func::call` requires an output buffer even if there are no returns.
        let mut results: [wasmtime::Val; 0] = [];
        let _ = entry.setup.call(&mut rt.store, &[], &mut results);
    }

    fn call_guest_update(&mut self) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let Some(update) = &entry.update else { return };

        let mut results: [wasmtime::Val; 0] = [];
        let _ = update.call(&mut rt.store, &[], &mut results);
    }

    fn call_guest_draw(&mut self) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let Some(draw) = &entry.draw else { return };

        let mut results: [wasmtime::Val; 0] = [];
        let _ = draw.call(&mut rt.store, &[], &mut results);
    }

    fn clear_guest(&mut self) {
        self.module = None;
        self.instance = None;
        self.entrypoints = None;
        // Keep `rt` allocated so subsequent loads are faster; it’s safe because imports are pure host fns.
    }

    // Public API for libretro_glue

    pub fn load_game_from_bytes(&mut self, data: &[u8]) -> Result<(), anyhow::Error> {
        // Ensure runtime exists so we have an Engine to compile against.
        if self.ensure_runtime().is_err() {
            state::clear_on_unload();
            return Err(anyhow::anyhow!("Failed to initialize runtime"));
        }

        let rt = self.rt.as_ref().unwrap();

        // Compile module (WASM or WAT) using Wasmtime Engine.
        let module = match loader::compile_module(&rt.engine, data) {
            Ok(m) => m,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to compile module: {:?}", e));
            }
        };

        self.module = Some(module);

        // Instantiate module + resolve entrypoints/memory (with detailed errors).
        if let Err(e) = self.instantiate_with_details() {
            state::clear_on_unload();
            self.clear_guest();
            return Err(anyhow::anyhow!("Failed to instantiate module: {e:?}"));
        }

        // Call setup
        // self.call_guest_setup();
        self.setup_called = false;

        Ok(())
    }

    pub fn unload(&mut self) {
        self.clear_guest();
        state::clear_on_unload();
    }

    pub fn run_frame(&mut self) {
        if !self.setup_called {
            self.call_guest_setup();
            self.setup_called = true;
        }

        // Snapshot inputs once per frame for determinism.
        input::snapshot_per_frame();

        // Run guest update loop.
        self.call_guest_update();

        // Run guest draw loop.
        self.call_guest_draw();

        // Present video and drain audio.
        av::video_present_host();
        av::audio_drain_host(0);
    }

    pub fn reset(&mut self) {
        self.setup_called = false;
    }
}
