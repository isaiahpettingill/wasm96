//! wasm96-core: a libretro core that loads and runs a guest WASM/WAT module.
//!
//! This crate implements an **Immediate Mode ABI**:
//! - The host owns the framebuffer and handles rendering.
//! - The guest issues drawing commands.
//! - The guest exports `setup`, `update`, and `draw`.
//!
//! The ABI surface is defined in `crate::abi` and mirrored by `wasm96-sdk`.

mod abi;
mod av;
mod input;
mod loader;
mod state;

use crate::abi::{GuestEntrypoints, IMPORT_MODULE};
use libretro_backend::{Core, CoreInfo, RuntimeHandle, libretro_core};
use wasmer::{FunctionEnv, FunctionEnvMut, Imports, Store};

/// The libretro core instance.
pub struct Wasm96Core {
    store: Store,
    module: Option<wasmer::Module>,
    instance: Option<wasmer::Instance>,
    entrypoints: Option<GuestEntrypoints>,
    env: Option<FunctionEnv<()>>,
    game_data: Option<libretro_backend::GameData>,
}

impl Default for Wasm96Core {
    fn default() -> Self {
        Self {
            store: Store::default(),
            module: None,
            instance: None,
            entrypoints: None,
            env: None,
            game_data: None,
        }
    }
}

impl Wasm96Core {
    fn build_imports(&mut self) -> Imports {
        // Wasmer needs an env to pass to host functions that read guest memory views.
        self.env = Some(FunctionEnv::new(&mut self.store, ()));
        let env = self.env.as_ref().unwrap().clone();

        // Note: all imports are under module `env` (see abi::IMPORT_MODULE),
        // because wasm32 targets typically expect `"env"` for imports.
        wasmer::imports! {
            IMPORT_MODULE => {
                // --- Graphics ---

                abi::host_imports::GRAPHICS_SET_SIZE => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, width: u32, height: u32| {
                        av::graphics_set_size(width, height);
                    }
                ),

                abi::host_imports::GRAPHICS_SET_COLOR => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, r: u32, g: u32, b: u32, a: u32| {
                        av::graphics_set_color(r, g, b, a);
                    }
                ),

                abi::host_imports::GRAPHICS_BACKGROUND => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, r: u32, g: u32, b: u32| {
                        av::graphics_background(r, g, b);
                    }
                ),

                abi::host_imports::GRAPHICS_POINT => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, x: i32, y: i32| {
                        av::graphics_point(x, y);
                    }
                ),

                abi::host_imports::GRAPHICS_LINE => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, x1: i32, y1: i32, x2: i32, y2: i32| {
                        av::graphics_line(x1, y1, x2, y2);
                    }
                ),

                abi::host_imports::GRAPHICS_RECT => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, x: i32, y: i32, w: u32, h: u32| {
                        av::graphics_rect(x, y, w, h);
                    }
                ),

                abi::host_imports::GRAPHICS_RECT_OUTLINE => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, x: i32, y: i32, w: u32, h: u32| {
                        av::graphics_rect_outline(x, y, w, h);
                    }
                ),

                abi::host_imports::GRAPHICS_CIRCLE => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, x: i32, y: i32, r: u32| {
                        av::graphics_circle(x, y, r);
                    }
                ),

                abi::host_imports::GRAPHICS_CIRCLE_OUTLINE => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, x: i32, y: i32, r: u32| {
                        av::graphics_circle_outline(x, y, r);
                    }
                ),

                abi::host_imports::GRAPHICS_IMAGE => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |env: FunctionEnvMut<()>, x: i32, y: i32, w: u32, h: u32, ptr: u32, len: u32| {
                        let _ = av::graphics_image(&env, x, y, w, h, ptr, len);
                    }
                ),

                // --- Audio ---

                abi::host_imports::AUDIO_INIT => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, sample_rate: u32| -> u32 {
                        av::audio_init(sample_rate)
                    }
                ),

                abi::host_imports::AUDIO_PUSH_SAMPLES => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |env: FunctionEnvMut<()>, ptr: u32, len: u32| {
                        let _ = av::audio_push_samples(&env, ptr, len);
                    }
                ),

                // --- Input ---

                abi::host_imports::INPUT_IS_BUTTON_DOWN => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, port: u32, btn: u32| -> u32 {
                        input::joypad_button_pressed(port, btn)
                    }
                ),

                abi::host_imports::INPUT_IS_KEY_DOWN => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, key: u32| -> u32 {
                        input::key_pressed(key)
                    }
                ),

                abi::host_imports::INPUT_GET_MOUSE_X => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>| -> i32 { input::mouse_x() }
                ),

                abi::host_imports::INPUT_GET_MOUSE_Y => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>| -> i32 { input::mouse_y() }
                ),

                abi::host_imports::INPUT_IS_MOUSE_DOWN => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, btn: u32| -> u32 {
                        // Map single button check to bitmask check if needed,
                        // or just expose raw bitmask check.
                        // For now, let's assume the input module handles this mapping or we do it here.
                        // The ABI says `is_mouse_down(btn) -> bool`.
                        // `input::mouse_buttons()` returns a bitmask.
                        let mask = input::mouse_buttons();
                        let requested = 1 << btn;
                        if (mask & requested) != 0 { 1 } else { 0 }
                    }
                ),

                // --- System ---
                // (Placeholder for now)
                abi::host_imports::SYSTEM_LOG => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>, _ptr: u32, _len: u32| {
                        // TODO: Implement logging
                    }
                ),

                abi::host_imports::SYSTEM_MILLIS => wasmer::Function::new_typed_with_env(
                    &mut self.store,
                    &env,
                    |_env: FunctionEnvMut<()>| -> u64 {
                        // TODO: Implement time
                        0
                    }
                ),
            }
        }
    }

    fn instantiate(&mut self) -> Result<(), ()> {
        // Take ownership of the module temporarily to avoid holding an immutable borrow
        // across `self.build_imports()` (which needs `&mut self`).
        let module = self.module.take().ok_or(())?;

        // Install imports and instantiate.
        let imports = self.build_imports();
        let instance = wasmer::Instance::new(&mut self.store, &module, &imports).map_err(|_| ())?;

        // Put the module back now that instantiation succeeded.
        self.module = Some(module);

        // Validate required exports + resolve entrypoints.
        abi::validate::required_exports_present(&instance).map_err(|_| ())?;
        let entrypoints = GuestEntrypoints::resolve(&instance).map_err(|_| ())?;

        // Register exported memory in global state.
        let mem = instance.exports.get_memory("memory").map_err(|_| ())?;
        state::set_guest_memory(mem);

        // Store instance/entrypoints.
        self.instance = Some(instance);
        self.entrypoints = Some(entrypoints);

        Ok(())
    }

    fn call_guest_setup(&mut self) {
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let _ = entry.setup.call(&mut self.store, &[]);
    }

    fn call_guest_update(&mut self) {
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let _ = entry.update.call(&mut self.store, &[]);
    }

    fn call_guest_draw(&mut self) {
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let _ = entry.draw.call(&mut self.store, &[]);
    }
}

impl Core for Wasm96Core {
    fn save_memory(&mut self) -> Option<&mut [u8]> {
        None
    }

    fn rtc_memory(&mut self) -> Option<&mut [u8]> {
        None
    }

    fn system_memory(&mut self) -> Option<&mut [u8]> {
        None
    }

    fn video_memory(&mut self) -> Option<&mut [u8]> {
        None
    }

    fn info() -> CoreInfo {
        CoreInfo::new("Wasm96", "1.0.0")
            .supports_roms_with_extension("wasm")
            .supports_roms_with_extension("wat")
    }

    fn on_load_game(
        &mut self,
        game_data: libretro_backend::GameData,
    ) -> libretro_backend::LoadGameResult {
        self.game_data = Some(game_data);

        let data = match self.game_data.as_ref().unwrap().data() {
            Some(d) => d,
            None => {
                return libretro_backend::LoadGameResult::Failed(self.game_data.take().unwrap());
            }
        };

        // Compile module (WASM or WAT).
        let module = match loader::compile_module(&self.store, data) {
            Ok(m) => m,
            Err(_) => {
                return libretro_backend::LoadGameResult::Failed(self.game_data.take().unwrap());
            }
        };

        self.module = Some(module);

        // Instantiate module + resolve entrypoints/memory.
        if self.instantiate().is_err() {
            state::clear_on_unload();
            self.module = None;
            self.instance = None;
            self.entrypoints = None;
            self.env = None;
            return libretro_backend::LoadGameResult::Failed(self.game_data.take().unwrap());
        }

        // Call setup
        self.call_guest_setup();

        // For now we return default AV info. The guest controls the actual buffer size via ABI calls.
        libretro_backend::LoadGameResult::Success(libretro_backend::AudioVideoInfo::new())
    }

    fn on_unload_game(&mut self) -> libretro_backend::GameData {
        self.module = None;
        self.instance = None;
        self.entrypoints = None;
        self.env = None;

        state::clear_on_unload();

        self.game_data.take().unwrap()
    }

    fn on_run(&mut self, handle: &mut RuntimeHandle) {
        // Update global handle pointer first.
        state::set_runtime_handle(handle);

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

    fn on_reset(&mut self) {
        // Re-run setup on reset? Or add a reset export?
        // For now, let's just re-run setup.
        self.call_guest_setup();
    }
}

libretro_core!(Wasm96Core);
