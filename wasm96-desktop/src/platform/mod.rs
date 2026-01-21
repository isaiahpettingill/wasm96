pub mod audio;
pub mod input;

use crate::platform::audio::AudioProducer;
use crate::platform::input::InputState;
use eframe::wgpu;
use std::sync::{Arc, Mutex};
use wasm96_engine::{PlatformAudio, PlatformCallbacks, PlatformGraphics};

/// Desktop implementation of PlatformCallbacks for use with eframe.
/// This implementation prioritizes wgpu for hardware acceleration.
pub struct DesktopPlatform {
    // Graphics state
    pub framebuffer: Arc<Mutex<Vec<u32>>>,
    pub width: u32,
    pub height: u32,
    pub hw_fbo: usize,

    // wgpu context (if available)
    pub device: Option<Arc<wgpu::Device>>,
    pub queue: Option<Arc<wgpu::Queue>>,
    pub surface_format: Option<wgpu::TextureFormat>,
    pub current_view: Option<Arc<wgpu::TextureView>>,
    pub current_view_size: Option<(u32, u32)>,

    // Audio state
    pub audio_producer: AudioProducer,

    // Input state
    pub input: InputState,
}

impl DesktopPlatform {
    pub fn new(audio_producer: AudioProducer) -> Self {
        Self {
            framebuffer: Arc::new(Mutex::new(vec![0u32; 320 * 240])),
            width: 320,
            height: 240,
            hw_fbo: 0,
            device: None,
            queue: None,
            surface_format: None,
            current_view: None,
            current_view_size: None,
            audio_producer,
            input: InputState::new(),
        }
    }
}

impl PlatformGraphics for DesktopPlatform {
    fn init_hardware_context(&mut self) -> bool {
        // We prefer wgpu via init_wgpu, but we keep this for GL compatibility if the engine requires it.
        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we have a GL context (e.g. from eframe's glow renderer), init engine GL.
            // Note: Desktop app now defaults to wgpu/vulkan via eframe.
            false
        }
        #[cfg(target_arch = "wasm32")]
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
        self.hw_fbo
    }

    fn prepare_frame(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            let mut fb = self.framebuffer.lock().unwrap();
            fb.resize((width * height) as usize, 0);
        }
    }

    fn present_software_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        stride_pixels: u32,
    ) {
        let mut fb = self.framebuffer.lock().unwrap();
        if fb.len() != (width * height) as usize {
            fb.resize((width * height) as usize, 0);
        }

        // Copy row by row to respect stride
        for y in 0..height {
            let src_start = (y * stride_pixels) as usize;
            let src_end = src_start + width as usize;
            let dst_start = (y * width) as usize;
            let dst_end = dst_start + width as usize;

            if src_end <= framebuffer.len() && dst_end <= fb.len() {
                fb[dst_start..dst_end].copy_from_slice(&framebuffer[src_start..src_end]);
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
        // Fallback to software presentation when hardware path is not explicitly handled by wgpu
        self.present_software_frame(framebuffer, width, height, stride_pixels);
    }

    fn present_wgpu_frame(&mut self, _view: &wgpu::TextureView, _width: u32, _height: u32) {
        // The engine has finished rendering to the wgpu view.
        // On desktop, we'll display this view in the egui UI.
    }

    fn notify_geometry_changed(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

impl PlatformAudio for DesktopPlatform {
    fn audio_batch(&mut self, samples: &[i16]) {
        use ringbuf::traits::Producer;
        let _ = self.audio_producer.push_slice(samples);
    }
}

impl PlatformCallbacks for DesktopPlatform {}
