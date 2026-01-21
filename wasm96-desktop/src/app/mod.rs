use crate::platform::DesktopPlatform;
use eframe::{egui, wgpu};
use std::sync::{Arc, Mutex};
use wasm96_engine::{Engine, PlatformGraphics};

#[derive(Clone)]
pub struct Cartridge {
    name: String,
    data: Vec<u8>,
}

pub struct Wasm96App {
    engine: Engine,
    platform: DesktopPlatform,
    texture: Option<egui::TextureHandle>,
    wgpu_texture_id: Option<(egui::TextureId, [u32; 2])>,
    wgpu_render_texture: Option<wgpu::Texture>,
    framebuffer: Arc<Mutex<Vec<u32>>>,
    last_frame_time: std::time::Instant,
    loaded_filename: Option<String>,
    cartridges: [Option<Cartridge>; 10],
    disk_paths: [Option<std::path::PathBuf>; 5],
    show_no_disk_warning: bool,
    show_mount_cart_dialog: Option<(String, Vec<u8>)>,
    show_run_from_disk_dialog: Option<usize>,
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

        let mut cartridges: [Option<Cartridge>; 10] = Default::default();
        let disk_paths: [Option<std::path::PathBuf>; 5] = Default::default();

        // Load persisted cartridges
        for i in 0..10 {
            let cart_path = format!("CART{}.w96", i);
            let name_path = format!("CART{}.name", i);
            if let Ok(data) = std::fs::read(&cart_path) {
                let name =
                    std::fs::read_to_string(&name_path).unwrap_or_else(|_| format!("CART{}", i));
                cartridges[i] = Some(Cartridge { name, data });
            }
        }

        // Check for DISK0 presence
        let mut show_no_disk_warning = false;
        let disk0_path = std::path::PathBuf::from("DISK0.img");
        if !disk0_path.exists() {
            show_no_disk_warning = true;
        } else if let Ok(bytes) = std::fs::read(&disk0_path) {
            let disk = wasm96_engine::vfs::VirtualDisk::from_bytes(bytes);
            let mut gs = wasm96_engine::state::global().lock().unwrap();
            gs.vfs.mount_slot(0, disk);
        }

        // Handle CART0 boot if it exists
        let mut loaded_filename = None;
        if let Some(cart) = &cartridges[0] {
            if let Err(e) = engine.load_game_from_bytes(&cart.data) {
                eprintln!("Failed to load CART0: {}", e);
            } else {
                loaded_filename = Some(cart.name.clone());
            }
        }

        // Handle command line arguments
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let path = std::path::Path::new(&args[1]);
            match std::fs::read(path) {
                Ok(bytes) => {
                    if let Err(e) = engine.load_game_from_bytes(&bytes) {
                        eprintln!("Failed to load game from args: {}", e);
                    } else {
                        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                        cartridges[1] = Some(Cartridge {
                            name: name.clone(),
                            data: bytes,
                        });
                        loaded_filename = Some(name);
                    }
                }
                Err(e) => eprintln!("Failed to read file from args: {}", e),
            }
        }

        let mut app = Self {
            engine,
            platform,
            texture: None,
            wgpu_texture_id: None,
            wgpu_render_texture: None,
            framebuffer,
            last_frame_time: std::time::Instant::now(),
            loaded_filename,
            cartridges,
            disk_paths,
            show_no_disk_warning,
            show_mount_cart_dialog: None,
            show_run_from_disk_dialog: None,
        };

        if let Some(name) = app.loaded_filename.clone() {
            app.load_storage(&name);
        }

        app
    }

    fn save_disk(&self, slot: usize) {
        let gs = wasm96_engine::state::global().lock().unwrap();
        if let Some(disk) = &gs.vfs.disks[slot] {
            let bytes = disk.export();
            let path = if slot == 0 {
                std::path::PathBuf::from("DISK0.img")
            } else if let Some(p) = &self.disk_paths[slot] {
                p.clone()
            } else {
                return;
            };
            if let Err(e) = std::fs::write(path, bytes) {
                eprintln!("Failed to auto-save DISK{}: {}", slot, e);
            }
        }
    }

    fn save_cartridges(&self) {
        for i in 0..10 {
            if let Some(cart) = &self.cartridges[i] {
                let _ = std::fs::write(format!("CART{}.w96", i), &cart.data);
                let _ = std::fs::write(format!("CART{}.name", i), &cart.name);
            }
        }
    }

    fn save_storage(&self) {
        if let Some(name) = &self.loaded_filename {
            let gs = wasm96_engine::state::global().lock().unwrap();
            let mut save_data = Vec::new();
            for (key, val) in &gs.storage.kv {
                save_data.extend_from_slice(&key.to_le_bytes());
                save_data.extend_from_slice(&(val.len() as u32).to_le_bytes());
                save_data.extend_from_slice(val);
            }
            let path = format!("{}.sav", name);
            if save_data.is_empty() {
                if std::path::Path::new(&path).exists() {
                    let _ = std::fs::remove_file(path);
                }
            } else {
                let _ = std::fs::write(path, save_data);
            }
        }
    }

    fn load_storage(&mut self, name: &str) {
        let path = format!("{}.sav", name);
        if let Ok(data) = std::fs::read(path) {
            let mut gs = wasm96_engine::state::global().lock().unwrap();
            let mut pos = 0;
            while pos + 12 <= data.len() {
                let key = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                let len = u32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()) as usize;
                pos += 12;
                if pos + len <= data.len() {
                    gs.storage.kv.insert(key, data[pos..pos + len].to_vec());
                    pos += len;
                } else {
                    break;
                }
            }
        }
    }
}

impl eframe::App for Wasm96App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // UI: Top Menu Bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Cartridge (.w96, .wasm, .wat)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("wasm96", &["w96", "wasm", "wat"])
                            .pick_file()
                        {
                            match std::fs::read(&path) {
                                Ok(bytes) => {
                                    let name =
                                        path.file_stem().unwrap().to_string_lossy().into_owned();
                                    self.show_mount_cart_dialog = Some((name, bytes));
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

                ui.menu_button("Cartridges", |ui| {
                    for i in 0..10 {
                        let label = if i == 0 {
                            "CART0 (Boot)".to_string()
                        } else {
                            format!("CART{}", i)
                        };
                        ui.menu_button(&label, |ui| {
                            if let Some(cart) = self.cartridges[i].clone() {
                                ui.label(format!("Loaded: {}", cart.name));
                                if ui.button("Run").clicked() {
                                    self.save_storage();
                                    if let Err(e) = self.engine.load_game_from_bytes(&cart.data) {
                                        eprintln!("Failed to load game: {}", e);
                                    } else {
                                        self.loaded_filename = Some(cart.name.clone());
                                        self.load_storage(&cart.name);
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Unload").clicked() {
                                    self.cartridges[i] = None;
                                    ui.close_menu();
                                }
                            } else {
                                ui.label("Empty");
                                if ui.button("Mount Here...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("wasm96", &["w96", "wasm", "wat"])
                                        .pick_file()
                                    {
                                        match std::fs::read(&path) {
                                            Ok(bytes) => {
                                                let name = path
                                                    .file_stem()
                                                    .unwrap()
                                                    .to_string_lossy()
                                                    .into_owned();
                                                self.cartridges[i] =
                                                    Some(Cartridge { name, data: bytes });
                                            }
                                            Err(e) => eprintln!("Failed to read file: {}", e),
                                        }
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                });

                ui.menu_button("Disks", |ui| {
                    for i in 0..5 {
                        let label = if i == 0 {
                            "DISK0 (SRAM)".to_string()
                        } else {
                            format!("DISK{}", i)
                        };
                        ui.menu_button(&label, |ui| {
                            let has_disk = {
                                let gs = wasm96_engine::state::global().lock().unwrap();
                                gs.vfs.disks[i].is_some()
                            };

                            if has_disk {
                                ui.label("Mounted");
                                if ui.button("Export to Image...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .set_file_name(format!("disk{}.img", i))
                                        .save_file()
                                    {
                                        let gs = wasm96_engine::state::global().lock().unwrap();
                                        if let Some(disk) = &gs.vfs.disks[i] {
                                            let bytes = disk.export();
                                            let _ = std::fs::write(path, bytes);
                                        }
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Unmount").clicked() {
                                    let mut gs = wasm96_engine::state::global().lock().unwrap();
                                    gs.vfs.disks[i] = None;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Run Program from Disk...").clicked() {
                                    self.show_run_from_disk_dialog = Some(i);
                                    ui.close_menu();
                                }
                            } else {
                                ui.label("Empty");
                                if ui.button("Mount Image...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Disk Image", &["img"])
                                        .pick_file()
                                    {
                                        match std::fs::read(&path) {
                                            Ok(bytes) => {
                                                let disk =
                                                    wasm96_engine::vfs::VirtualDisk::from_bytes(
                                                        bytes,
                                                    );
                                                let mut gs =
                                                    wasm96_engine::state::global().lock().unwrap();
                                                gs.vfs.mount_slot(i, disk);
                                                self.disk_paths[i] = Some(path);
                                            }
                                            Err(e) => eprintln!("Failed to read disk: {}", e),
                                        }
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Create New (25MB)...").clicked() {
                                    let disk = wasm96_engine::vfs::VirtualDisk::new_in_memory(
                                        25 * 1024 * 1024,
                                    );
                                    let _ = disk.format("WASM96");
                                    let mut gs = wasm96_engine::state::global().lock().unwrap();
                                    gs.vfs.mount_slot(i, disk);
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                });
            });
        });

        // Dialogs
        if let Some((name, bytes)) = self.show_mount_cart_dialog.clone() {
            egui::Window::new("Mount Cartridge").show(ctx, |ui| {
                ui.label(format!("Mounting: {}", name));
                ui.label("Choose a slot (Default is SLOT1):");

                ui.horizontal_wrapped(|ui| {
                    for i in 0..10 {
                        let label = if i == 0 {
                            "SLOT0 (Boot)".to_owned()
                        } else {
                            format!("SLOT{}", i)
                        };
                        let is_default = i == 1;
                        let btn = if is_default {
                            ui.add(egui::Button::new(egui::RichText::new(label).strong()))
                        } else {
                            ui.button(label)
                        };

                        if btn.clicked() {
                            self.cartridges[i] = Some(Cartridge {
                                name: name.clone(),
                                data: bytes.clone(),
                            });
                            self.save_storage();
                            if let Err(e) = self.engine.load_game_from_bytes(&bytes) {
                                eprintln!("Failed to load game: {}", e);
                            } else {
                                self.loaded_filename = Some(name.clone());
                                self.load_storage(&name);
                            }
                            self.show_mount_cart_dialog = None;
                        }
                    }
                });

                if ui.button("Cancel").clicked() {
                    self.show_mount_cart_dialog = None;
                }
            });
        }

        if let Some(slot) = self.show_run_from_disk_dialog {
            let mut close = false;
            egui::Window::new(format!("Run from DISK{}", slot)).show(ctx, |ui| {
                ui.label("Scanning disk for .w96, .wasm, .wat files...");

                let mut programs = Vec::new();
                let temp_dir = std::env::temp_dir().join(format!("wasm96_vfs_scan_{}", slot));
                let _ = std::fs::remove_dir_all(&temp_dir);
                let _ = std::fs::create_dir_all(&temp_dir);

                let gs = wasm96_engine::state::global().lock().unwrap();
                if let Some(disk) = &gs.vfs.disks[slot] {
                    if disk.extract_to_host(&temp_dir).is_ok() {
                        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                                if !path.is_dir()
                                    && (name.ends_with(".w96")
                                        || name.ends_with(".wasm")
                                        || name.ends_with(".wat"))
                                {
                                    programs.push(name);
                                }
                            }
                        }
                    }
                }
                drop(gs);

                if programs.is_empty() {
                    ui.label("No executable programs found.");
                } else {
                    for name in programs {
                        if ui.button(&name).clicked() {
                            let path = temp_dir.join(&name);
                            if let Ok(data) = std::fs::read(path) {
                                self.save_storage();
                                if let Err(e) = self.engine.load_game_from_bytes(&data) {
                                    eprintln!("Failed to load from disk: {}", e);
                                } else {
                                    let stem = std::path::Path::new(&name)
                                        .file_stem()
                                        .unwrap()
                                        .to_string_lossy()
                                        .into_owned();
                                    self.loaded_filename = Some(stem.clone());
                                    self.load_storage(&stem);
                                    close = true;
                                }
                            }
                        }
                    }
                }

                if ui.button("Close").clicked() {
                    close = true;
                }
            });
            if close {
                self.show_run_from_disk_dialog = None;
            }
        }

        if self.show_no_disk_warning {
            egui::Window::new("No Disk Warning").show(ctx, |ui| {
                ui.label(
                    "No DISK0 (SRAM) found. It is highly recommended to create one for game saves.",
                );
                if ui.button("Create 25MB Disk in DISK0").clicked() {
                    let disk = wasm96_engine::vfs::VirtualDisk::new_in_memory(25 * 1024 * 1024);
                    let _ = disk.format("WASM96");
                    let mut gs = wasm96_engine::state::global().lock().unwrap();
                    gs.vfs.mount_slot(0, disk);
                    self.show_no_disk_warning = false;
                    drop(gs);
                    self.save_disk(0);
                }
                if ui.button("Dismiss").clicked() {
                    self.show_no_disk_warning = false;
                }
            });
        }

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
        self.save_storage();
        self.save_cartridges();
        self.engine.unload();
        // Auto-save all mounted disks that have paths or are DISK0
        for i in 0..5 {
            self.save_disk(i);
        }
    }
}
