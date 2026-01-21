use crate::platform::DesktopPlatform;
use eframe::{egui, wgpu};
use std::sync::{Arc, Mutex};
use wasm96_engine::{Engine, PlatformGraphics};

pub struct Wasm96App {
    engine: Engine,
    platform: DesktopPlatform,
    texture: Option<egui::TextureHandle>,
    wgpu_texture_id: Option<(egui::TextureId, [u32; 2])>,
    wgpu_render_texture: Option<wgpu::Texture>,
    framebuffer: Arc<Mutex<Vec<u32>>>,
    last_frame_time: std::time::Instant,
    loaded_filename: Option<String>,
}

impl Wasm96App {
    pub fn new(cc: &eframe::CreationContext<'_>, mut platform: DesktopPlatform) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(wgpu_render_state) = &cc.wgpu_render_state {
            platform.init_wgpu(
                wgpu_render_state.device.clone(),
                wgpu_render_state.queue.clone(),
                wgpu_render_state.target_format,
            );
        }

        let framebuffer = platform.framebuffer.clone();

        let mut engine = Engine::new();
        if let (Some(device), Some(queue), Some(format)) = (
            platform.device.clone(),
            platform.queue.clone(),
            platform.surface_format,
        ) {
            engine.init_wgpu(device, queue, format);
        }

        // Handle command line arguments
        let mut loaded_filename = None;
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let path = std::path::Path::new(&args[1]);
            match std::fs::read(path) {
                Ok(bytes) => {
                    if let Err(e) = engine.load_game_from_bytes(&bytes) {
                        eprintln!("Failed to load game from args: {}", e);
                    } else {
                        loaded_filename =
                            Some(path.file_name().unwrap().to_string_lossy().into_owned());
                    }
                }
                Err(e) => eprintln!("Failed to read file from args: {}", e),
            }
        }

        Self {
            engine,
            platform,
            texture: None,
            wgpu_texture_id: None,
            wgpu_render_texture: None,
            framebuffer,
            last_frame_time: std::time::Instant::now(),
            loaded_filename,
        }
    }
}

impl eframe::App for Wasm96App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // UI: Top Menu Bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open (.w96, .wasm, .wat)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("wasm96", &["w96", "wasm", "wat"])
                            .pick_file()
                        {
                            match std::fs::read(&path) {
                                Ok(bytes) => {
                                    if let Err(e) = self.engine.load_game_from_bytes(&bytes) {
                                        eprintln!("Failed to load game: {}", e);
                                    } else {
                                        self.loaded_filename = Some(
                                            path.file_name()
                                                .unwrap()
                                                .to_string_lossy()
                                                .into_owned(),
                                        );
                                    }
                                }
                                Err(e) => eprintln!("Failed to read file: {}", e),
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        // Run engine frame at ~60fps
        let now = std::time::Instant::now();
        let frame_budget = std::time::Duration::from_nanos(1_000_000_000 / 60);
        if now.duration_since(self.last_frame_time) >= frame_budget {
            self.last_frame_time = now;

            // Update input state before running the frame
            ctx.input(|i| {
                self.platform.input.update_egui_input(i.clone());
            });

            // Prepare WGPU texture if available
            #[cfg(not(target_arch = "wasm32"))]
            if let (Some(device), Some(render_state)) =
                (&self.platform.device, _frame.wgpu_render_state())
            {
                // Sync with engine resolution
                let video = self.engine.video_state();
                let width = video.width;
                let height = video.height;
                self.platform.width = width;
                self.platform.height = height;

                if width > 0 && height > 0 {
                    let needs_recreate = self
                        .wgpu_render_texture
                        .as_ref()
                        .map_or(true, |t| t.width() != width || t.height() != height);

                    if needs_recreate {
                        let texture = device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("wasm96_render_texture"),
                            size: wgpu::Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: self
                                .platform
                                .surface_format
                                .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb),
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });

                        let view =
                            Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
                        self.platform.current_view = Some(view.clone());
                        self.platform.current_view_size = Some((width, height));

                        let texture_id = render_state.renderer.write().register_native_texture(
                            device,
                            &view,
                            wgpu::FilterMode::Nearest,
                        );

                        self.wgpu_texture_id = Some((texture_id, [width, height]));
                        self.wgpu_render_texture = Some(texture);
                    }
                }
            }

            // Run one frame of the engine
            self.engine.run_frame(&mut self.platform);
        }

        // Display the game output
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                let width = self.platform.width;
                let height = self.platform.height;

                // Prefer WGPU display if we have a texture
                if let Some((texture_id, [tw, th])) = self.wgpu_texture_id {
                    if tw == width && th == height {
                        ui.centered_and_justified(|ui| {
                            ui.add(
                                egui::Image::new(egui::load::SizedTexture::new(
                                    texture_id,
                                    egui::vec2(width as f32, height as f32),
                                ))
                                .shrink_to_fit(),
                            );
                        });
                        return;
                    }
                }

                // Fallback to software display
                let fb = self.framebuffer.lock().unwrap();
                if !fb.is_empty() && width > 0 && height > 0 {
                    // Convert XRGB8888 to RGBA8888 for egui
                    let pixels: Vec<egui::Color32> = fb
                        .iter()
                        .map(|&p| {
                            let r = ((p >> 16) & 0xFF) as u8;
                            let g = ((p >> 8) & 0xFF) as u8;
                            let b = (p & 0xFF) as u8;
                            egui::Color32::from_rgb(r, g, b)
                        })
                        .collect();

                    let image = egui::ColorImage {
                        size: [width as usize, height as usize],
                        pixels,
                    };

                    let texture = self.texture.get_or_insert_with(|| {
                        ctx.load_texture("game_framebuffer", image.clone(), Default::default())
                    });
                    texture.set(image, Default::default());

                    ui.centered_and_justified(|ui| {
                        ui.add(egui::Image::new(&*texture).shrink_to_fit());
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        if let Some(name) = &self.loaded_filename {
                            ui.label(format!("Running {}...", name));
                        } else {
                            ui.label("Load a game from the File menu to start playing.");
                        }
                    });
                }
            });

        // Keep the UI loop running for smooth gameplay
        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.engine.unload();
    }
}
