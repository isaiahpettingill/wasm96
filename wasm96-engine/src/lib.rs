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
//! This crate is platform-agnostic and does not contain any libretro-specific code.
//! It can be used by various frontends: libretro, desktop, web, etc.

pub mod abi;
pub mod av;
pub mod input;
pub mod loader;
pub mod runtime;
pub mod state;
pub mod vfs;

use crate::abi::GuestEntrypoints;
use crate::runtime::{BackendRuntime, Instance, Module, Runtime};

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

    /// Initialize wgpu rendering context (if supported).
    ///
    /// Frontends using wgpu should call this to provide the device and queue.
    fn init_wgpu(
        &mut self,
        _device: std::sync::Arc<wgpu::Device>,
        _queue: std::sync::Arc<wgpu::Queue>,
        _format: wgpu::TextureFormat,
    ) -> bool {
        false
    }

    /// Get the wgpu texture view for the current frame.
    fn get_wgpu_view(&mut self) -> Option<std::sync::Arc<wgpu::TextureView>> {
        None
    }

    /// Get the size of the wgpu texture view.
    fn get_wgpu_view_size(&mut self) -> Option<(u32, u32)> {
        None
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

    /// Present a video frame using wgpu.
    ///
    /// # Arguments
    /// * `view` - The texture view to render into
    /// * `width` - Visible width
    /// * `height` - Visible height
    fn present_wgpu_frame(&mut self, _view: &wgpu::TextureView, _width: u32, _height: u32) {}

    /// Present a video frame (backward compatibility method).
    ///
    /// This method automatically selects between hardware and software rendering
    /// based on whether a hardware framebuffer is available.
    ///
    /// # Arguments
    /// * `framebuffer` - XRGB8888 pixel data (0x00RRGGBB format, top-left origin)
    /// * `width` - Visible width in pixels
    /// * `height` - Visible height in pixels
    /// * `stride_pixels` - Row stride in pixels (may be >= width for alignment)
    fn video_refresh(&mut self, framebuffer: &[u32], width: u32, height: u32, stride_pixels: u32) {
        // Check if hardware rendering is available
        if self.get_hardware_framebuffer() != 0 {
            self.present_hardware_frame(framebuffer, width, height, stride_pixels);
        } else {
            self.present_software_frame(framebuffer, width, height, stride_pixels);
        }
    }
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

/// Types of input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// A joypad button was pressed.
    JoypadPressed { port: u32, button: u32 },
    /// A joypad button was released.
    JoypadReleased { port: u32, button: u32 },
    /// A keyboard key was pressed.
    KeyPressed { key: u32 },
    /// A keyboard key was released.
    KeyReleased { key: u32 },
    /// A mouse button was pressed.
    MousePressed { button: u32, x: i32, y: i32 },
    /// A mouse button was released.
    MouseReleased { button: u32, x: i32, y: i32 },
}

/// Platform-agnostic input callbacks.
///
/// Frontends must implement this trait to handle input devices.
pub trait PlatformInput {
    /// Poll input devices (called once per frame).
    fn input_poll(&mut self);

    /// Get the next input event from the queue.
    fn input_get_event(&mut self) -> Option<InputEvent> {
        None
    }

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

    /// Get the next character from the input queue.
    fn input_get_char(&mut self) -> Option<u8> {
        None
    }

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
    rt: Option<BackendRuntime>,
    module: Option<Module>,
    instance: Option<Instance>,
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
        self.ensure_runtime()?;

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
            .map_err(|e| anyhow::anyhow!("instantiate failed: {e:?}"))?;

        self.instance = Some(instance);
        self.entrypoints = Some(entrypoints);
        Ok(())
    }

    fn ensure_runtime(&mut self) -> Result<(), anyhow::Error> {
        self.ensure_runtime_with_args(Vec::new(), Vec::new())
    }

    fn ensure_runtime_with_args(
        &mut self,
        args: Vec<String>,
        stdin: Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        if self.rt.is_some() {
            return Ok(());
        }

        #[cfg(not(target_arch = "wasm32"))]
        let mut rt = BackendRuntime::new_with_args(args, stdin)
            .map_err(|e| anyhow::anyhow!("Failed to create Wasmtime runtime: {e:?}"))?;
        #[cfg(target_arch = "wasm32")]
        let mut rt = BackendRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Web runtime: {e:?}"))?;

        rt.define_imports()
            .map_err(|e| anyhow::anyhow!("Failed to define host imports: {e:?}"))?;
        self.rt = Some(rt);
        Ok(())
    }

    fn call_guest_setup(&mut self) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut results: [wasmtime::Val; 0] = [];
            let _ = entry.setup.call(&mut rt.store, &[], &mut results);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = entry.setup.call0(&js_sys::Object::new());
        }
    }

    fn call_guest_update(&mut self) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let Some(update) = &entry.update else { return };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut results: [wasmtime::Val; 0] = [];
            let _ = update.call(&mut rt.store, &[], &mut results);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = update.call0(&js_sys::Object::new());
        }
    }

    fn call_guest_draw(&mut self) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let Some(draw) = &entry.draw else { return };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut results: [wasmtime::Val; 0] = [];
            let _ = draw.call(&mut rt.store, &[], &mut results);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = draw.call0(&js_sys::Object::new());
        }
    }

    fn call_guest_on_key_pressed(&mut self, key: u32) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let Some(func) = &entry.on_key_pressed else {
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut results: [wasmtime::Val; 0] = [];
            let _ = func.call(
                &mut rt.store,
                &[wasmtime::Val::I32(key as i32)],
                &mut results,
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = func.call1(&js_sys::Object::new(), &key.into());
        }
    }

    fn call_guest_on_joypad_pressed(&mut self, port: u32, button: u32) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let Some(func) = &entry.on_joypad_pressed else {
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut results: [wasmtime::Val; 0] = [];
            let _ = func.call(
                &mut rt.store,
                &[
                    wasmtime::Val::I32(port as i32),
                    wasmtime::Val::I32(button as i32),
                ],
                &mut results,
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = func.call2(&js_sys::Object::new(), &port.into(), &button.into());
        }
    }

    fn call_guest_on_mouse_clicked(&mut self, button: u32, x: i32, y: i32) {
        let Some(rt) = self.rt.as_mut() else { return };
        let Some(entry) = &self.entrypoints else {
            return;
        };
        let Some(func) = &entry.on_mouse_clicked else {
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut results: [wasmtime::Val; 0] = [];
            let _ = func.call(
                &mut rt.store,
                &[
                    wasmtime::Val::I32(button as i32),
                    wasmtime::Val::I32(x),
                    wasmtime::Val::I32(y),
                ],
                &mut results,
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = func.call3(&js_sys::Object::new(), &button.into(), &x.into(), &y.into());
        }
    }

    fn clear_guest(&mut self) {
        self.module = None;
        self.instance = None;
        self.entrypoints = None;
        // Keep `rt` allocated so subsequent loads are faster.
    }

    /// Load a game from raw bytes (WASM or WAT).
    ///
    /// # Arguments
    /// * `data` - The WASM or WAT file content
    ///
    /// # Returns
    /// `Ok(())` on success, `Err` with details on failure
    /// Initialize the wgpu backend for hardware-accelerated 3D.
    pub fn init_wgpu(
        &mut self,
        device: std::sync::Arc<wgpu::Device>,
        queue: std::sync::Arc<wgpu::Queue>,
        format: wgpu::TextureFormat,
    ) {
        av::init_wgpu(device, queue, format);
    }

    pub fn load_game_from_bytes(&mut self, data: &[u8]) -> Result<(), anyhow::Error> {
        self.load_game_from_bytes_with_args(data, Vec::new(), Vec::new())
    }

    pub fn load_game_from_bytes_with_args(
        &mut self,
        data: &[u8],
        args: Vec<String>,
        stdin: Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        // Ensure runtime exists with proper args.
        // We drop the old runtime to ensure WASI is re-initialized for the new game.
        self.rt = None;
        if let Err(e) = self.ensure_runtime_with_args(args, stdin) {
            state::clear_on_unload();
            return Err(anyhow::anyhow!("Failed to initialize runtime: {e}"));
        }

        let rt = self.rt.as_ref().unwrap();

        // Compile module using the runtime's compile_module implementation.
        let module = match rt.compile_module(data) {
            Ok(m) => m,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to compile module: {:?}", e));
            }
        };

        self.module = Some(module);

        // Instantiate module + resolve entrypoints/memory.
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
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(rt) = self.rt.as_mut() {
            let _ = rt.sync_wasi_to_vfs();
        }

        self.clear_guest();
        state::clear_on_unload();
    }

    /// Run a single frame.
    ///
    /// # Arguments
    /// * `callbacks` - Platform callbacks for rendering, audio, and input
    pub fn run_frame(&mut self, callbacks: &mut dyn PlatformCallbacks) {
        // Check for pending cartridge load
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pending = {
                let mut gs = state::global().lock().unwrap();
                gs.pending_cartridge.take()
            };

            if let Some(pending) = pending {
                self.unload();
                let _ =
                    self.load_game_from_bytes_with_args(&pending.data, pending.args, pending.stdin);
            }
        }

        // Set callbacks in thread-local storage so WASM imports can access them
        state::set_callbacks(callbacks);

        // Prepare rendering context (FBO, viewport, etc.) before guest code runs.
        let (width, height, fbo) = {
            let s = state::global().lock().unwrap();
            (
                s.video.width,
                s.video.height,
                callbacks.get_hardware_framebuffer(),
            )
        };

        #[cfg(not(target_arch = "wasm32"))]
        av::prepare_frame(fbo);

        callbacks.prepare_frame(width, height);

        if !self.setup_called {
            self.call_guest_setup();
            self.setup_called = true;
        }

        // Snapshot inputs once per frame for determinism.
        input::snapshot_per_frame(callbacks);

        // Process input events.
        while let Some(event) = callbacks.input_get_event() {
            match event {
                InputEvent::KeyPressed { key } => self.call_guest_on_key_pressed(key),
                InputEvent::JoypadPressed { port, button } => {
                    self.call_guest_on_joypad_pressed(port, button)
                }
                InputEvent::MousePressed { button, x, y } => {
                    self.call_guest_on_mouse_clicked(button, x, y)
                }
                _ => {}
            }
        }

        // Run guest update loop.
        self.call_guest_update();

        // Run guest draw loop.
        self.call_guest_draw();

        // Present video and drain audio.
        if let Some(view) = callbacks.get_wgpu_view() {
            let (width, height, sw_fb) = {
                let s = state::global().lock().unwrap();
                (s.video.width, s.video.height, s.video.framebuffer.clone())
            };

            // Avoid wgpu panic if the view size doesn't match the engine resolution yet.
            // This can happen during resolution changes.
            let size_ok = if let Some((vw, vh)) = callbacks.get_wgpu_view_size() {
                vw == width && vh == height
            } else {
                true
            };

            if size_ok {
                av::wgpu_present(&view, width, height, &sw_fb);
                callbacks.present_wgpu_frame(&view, width, height);
            } else {
                av::video_present_host(callbacks);
            }
        } else {
            av::video_present_host(callbacks);
        }
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wasmtime_engine(&self) -> Option<&wasmtime::Engine> {
        self.rt.as_ref().map(|rt| &rt.engine)
    }
}
