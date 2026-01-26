pub mod audio;
pub mod input;

use self::audio::WebAudio;
use self::input::WebInput;
use std::sync::Arc;
use wasm96_engine::{
    InputEvent, PlatformAudio, PlatformCallbacks, PlatformGraphics, PlatformInput,
};
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

pub struct WebPlatform {
    // Audio
    pub audio: WebAudio,

    // Input
    pub input: WebInput,

    // Graphics - Software
    pub canvas: HtmlCanvasElement,
    pub ctx_2d: Option<CanvasRenderingContext2d>,
    pub width: u32,
    pub height: u32,

    // Graphics - Hardware (WGPU)
    pub device: Option<Arc<wgpu::Device>>,
    pub queue: Option<Arc<wgpu::Queue>>,
    pub surface: Option<wgpu::Surface<'static>>,
    pub surface_format: Option<wgpu::TextureFormat>,
    pub current_view: Option<Arc<wgpu::TextureView>>,
    pub current_view_size: Option<(u32, u32)>,
}

impl WebPlatform {
    pub fn new(canvas: HtmlCanvasElement, audio: WebAudio, input: WebInput) -> Self {
        // Try to get 2D context for software fallback
        let ctx_2d = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|x| x.dyn_into::<CanvasRenderingContext2d>().ok());

        Self {
            audio,
            input,
            canvas,
            ctx_2d,
            width: 320,
            height: 240,
            device: None,
            queue: None,
            surface: None,
            surface_format: None,
            current_view: None,
            current_view_size: None,
        }
    }

    pub fn set_wgpu_view(&mut self, view: Arc<wgpu::TextureView>, width: u32, height: u32) {
        self.current_view = Some(view);
        self.current_view_size = Some((width, height));
    }
}

impl PlatformGraphics for WebPlatform {
    fn init_hardware_context(&mut self) -> bool {
        // WebGL init logic if needed, but we prefer wgpu or software
        false
    }

    fn init_wgpu(
        &mut self,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        format: wgpu::TextureFormat,
    ) -> bool {
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface_format = Some(format);
        true
    }

    fn get_wgpu_view(&mut self) -> Option<Arc<wgpu::TextureView>> {
        self.current_view.clone()
    }

    fn get_wgpu_view_size(&mut self) -> Option<(u32, u32)> {
        self.current_view_size
    }

    fn get_hardware_framebuffer(&mut self) -> usize {
        0
    }

    fn prepare_frame(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            // Resize canvas if needed
            self.canvas.set_width(width);
            self.canvas.set_height(height);
        }
    }

    fn present_software_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        _stride_pixels: u32,
    ) {
        // Only present software if we have a 2D context
        if let Some(ctx) = &self.ctx_2d {
            let len = (width * height) as usize;
            let mut data = vec![0u8; len * 4];

            // Convert XRGB u32 buffer to RGBA u8 buffer for Canvas
            // Optimization: Unroll or use SIMD if performance is critical,
            // but for 320x240 this is usually fine.
            for (i, &pixel) in framebuffer.iter().take(len).enumerate() {
                let idx = i * 4;
                data[idx] = ((pixel >> 16) & 0xFF) as u8; // R
                data[idx + 1] = ((pixel >> 8) & 0xFF) as u8; // G
                data[idx + 2] = (pixel & 0xFF) as u8; // B
                data[idx + 3] = 255; // A
            }

            let clamped = Clamped(data.as_slice());
            if let Ok(image_data) =
                ImageData::new_with_u8_clamped_array_and_sh(clamped, width, height)
            {
                let _ = ctx.put_image_data(&image_data, 0.0, 0.0);
            }
        }
    }

    fn present_hardware_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        stride_pixels: u32,
    ) {
        // Fallback to software presentation
        self.present_software_frame(framebuffer, width, height, stride_pixels);
    }

    fn present_wgpu_frame(&mut self, _view: &wgpu::TextureView, _width: u32, _height: u32) {
        // View is already presented via the surface infrastructure in the main loop
        // We just clear our reference to it so we get a fresh one next time
        self.current_view = None;
    }

    fn notify_geometry_changed(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

impl PlatformAudio for WebPlatform {
    fn audio_batch(&mut self, samples: &[i16]) {
        self.audio.audio_batch(samples);
    }
}

impl PlatformInput for WebPlatform {
    fn input_poll(&mut self) {
        self.input.input_poll();
    }

    fn input_get_event(&mut self) -> Option<InputEvent> {
        self.input.input_get_event()
    }

    fn input_button_state(&mut self, port: u32, button: u32) -> bool {
        self.input.input_button_state(port, button)
    }

    fn input_key_state(&mut self, key: u32) -> bool {
        self.input.input_key_state(key)
    }

    fn input_get_char(&mut self) -> Option<u8> {
        self.input.input_get_char()
    }

    fn input_mouse_x(&mut self) -> i32 {
        self.input.input_mouse_x()
    }

    fn input_mouse_y(&mut self) -> i32 {
        self.input.input_mouse_y()
    }

    fn input_mouse_button(&mut self, button: u32) -> bool {
        self.input.input_mouse_button(button)
    }
}

impl PlatformCallbacks for WebPlatform {}
