//! wasm96-engine: Platform-agnostic core engine for running WASM/WAT modules.
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
//!
//! This crate is platform-agnostic and does not contain any libretro-specific code.
//! It can be used by various frontends: libretro, desktop, web, etc.

pub mod abi;
pub mod av;
pub mod input;
pub mod loader;
pub mod runtime;
pub mod state;

use crate::abi::GuestEntrypoints;

/// Platform-agnostic graphics callbacks.
///
/// Frontends must implement this trait to handle rendering.
/// The engine maintains a software framebuffer and delegates all platform-specific
/// rendering (OpenGL, software blitting, etc.) to the frontend.
pub trait PlatformGraphics {
    /// Initialize hardware rendering context (if supported).
    ///
    /// Called once when the core loads. Returns true if HW rendering is available.
    fn init_hardware_context(&mut self) -> bool {
        false
    }

    /// Get the current hardware framebuffer object.
    ///
    /// # Returns
    /// OpenGL FBO handle, or 0 if hardware rendering is not available
    fn get_hardware_framebuffer(&mut self) -> usize {
        0
    }

    /// Prepare to render a frame (called before guest code runs).
    ///
    /// For HW rendering: binds the FBO and sets viewport.
    /// For SW rendering: can be a no-op.
    fn prepare_frame(&mut self, width: u32, height: u32);

    /// Present a video frame using software rendering.
    ///
    /// # Arguments
    /// * `framebuffer` - XRGB8888 pixel data (0x00RRGGBB format, top-left origin)
    /// * `width` - Visible width in pixels
    /// * `height` - Visible height in pixels
    /// * `stride_pixels` - Row stride in pixels (may be >= width for alignment)
    fn present_software_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        stride_pixels: u32,
    );

    /// Present a video frame using hardware rendering.
    ///
    /// The frontend should upload the software framebuffer to a texture,
    /// composite it to the HW FBO, and present the result.
    ///
    /// # Arguments
    /// * `framebuffer` - XRGB8888 pixel data (0x00RRGGBB format, top-left origin)
    /// * `width` - Visible width in pixels
    /// * `height` - Visible height in pixels
    /// * `stride_pixels` - Row stride in pixels (may be >= width for alignment)
    fn present_hardware_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        stride_pixels: u32,
    );

    /// Notify that geometry has changed.
    ///
    /// # Arguments
    /// * `width` - New visible width
    /// * `height` - New visible height
    fn notify_geometry_changed(&mut self, width: u32, height: u32);
}

/// Platform-agnostic audio callbacks.
///
/// Frontends must implement this trait to handle audio output.
pub trait PlatformAudio {
    /// Submit audio samples.
    ///
    /// # Arguments
    /// * `samples` - Interleaved stereo i16 samples (L, R, L, R, ...)
    fn audio_batch(&mut self, samples: &[i16]);
}

/// Platform-agnostic input callbacks.
///
/// Frontends must implement this trait to handle input devices.
pub trait PlatformInput {
    /// Poll input devices (called once per frame).
    fn input_poll(&mut self);

    /// Query button state.
    ///
    /// # Arguments
    /// * `port` - Controller port (0-based)
    /// * `button` - Button index
    ///
    /// # Returns
    /// `true` if the button is pressed, `false` otherwise
    fn input_button_state(&mut self, port: u32, button: u32) -> bool;

    /// Query keyboard key state.
    ///
    /// # Arguments
    /// * `key` - Key code
    ///
    /// # Returns
    /// `true` if the key is pressed, `false` otherwise
    fn input_key_state(&mut self, key: u32) -> bool;

    /// Get mouse X position.
    fn input_mouse_x(&mut self) -> i32;

    /// Get mouse Y position.
    fn input_mouse_y(&mut self) -> i32;

    /// Query mouse button state.
    ///
    /// # Arguments
    /// * `button` - Mouse button index (0 = left, 1 = right, 2 = middle)
    ///
    /// # Returns
    /// `true` if the button is pressed, `false` otherwise
    fn input_mouse_button(&mut self, button: u32) -> bool;
}

/// Combined platform callbacks trait.
///
/// Frontends must implement all three sub-traits (graphics, audio, input).
/// This trait provides a unified interface for the engine to interact with the platform.
pub trait PlatformCallbacks: PlatformGraphics + PlatformAudio + PlatformInput {}

/// The core engine instance.
pub struct Engine {
    rt: Option<runtime::WasmtimeRuntime>,
    module: Option<wasmtime::Module>,
    instance: Option<wasmtime::Instance>,
    entrypoints: Option<GuestEntrypoints>,
    setup_called: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Create a new engine instance.
    pub fn new() -> Self {
        Self {
            rt: None,
            module: None,
            instance: None,
            entrypoints: None,
            setup_called: false,
        }
    }

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
        // Keep `rt` allocated so subsequent loads are faster; it's safe because imports are pure host fns.
    }

    /// Load a game from raw bytes (WASM or WAT).
    ///
    /// # Arguments
    /// * `data` - The WASM or WAT file content
    ///
    /// # Returns
    /// `Ok(())` on success, `Err` with details on failure
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

        self.setup_called = false;

        Ok(())
    }

    /// Unload the current game and reset state.
    pub fn unload(&mut self) {
        self.clear_guest();
        state::clear_on_unload();
    }

    /// Run a single frame.
    ///
    /// # Arguments
    /// * `callbacks` - Platform callbacks for rendering, audio, and input
    pub fn run_frame(&mut self, callbacks: &mut dyn PlatformCallbacks) {
        // Set callbacks in thread-local storage so WASM imports can access them
        state::set_callbacks(callbacks);

        if !self.setup_called {
            self.call_guest_setup();
            self.setup_called = true;
        }

        // Snapshot inputs once per frame for determinism.
        input::snapshot_per_frame(callbacks);

        // Run guest update loop.
        self.call_guest_update();

        // Run guest draw loop.
        self.call_guest_draw();

        // Present video and drain audio.
        av::video_present_host(callbacks);
        av::audio_drain_host(callbacks);

        // Clear callbacks from thread-local storage
        state::clear_callbacks();
    }

    /// Reset the engine (re-call setup on next frame).
    pub fn reset(&mut self) {
        self.setup_called = false;
    }

    /// Get the current video state (for querying dimensions, etc).
    pub fn video_state(&self) -> state::VideoStateSnapshot {
        let s = state::global().lock().unwrap();
        state::VideoStateSnapshot {
            width: s.video.width,
            height: s.video.height,
            stride_pixels: s.video.stride_pixels,
            geometry_dirty: s.video.geometry_dirty,
        }
    }

    /// Get the Wasmtime engine (for pre-compiling modules).
    pub fn wasmtime_engine(&self) -> Option<&wasmtime::Engine> {
        self.rt.as_ref().map(|rt| &rt.engine)
    }
}
