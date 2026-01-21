use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use gilrs::{Button as GilrsButton, Gilrs};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split},
};
use std::sync::{Arc, Mutex};
use wasm96_engine::{Engine, PlatformAudio, PlatformCallbacks, PlatformGraphics, PlatformInput};

const DEFAULT_WIDTH: u32 = 320;
const DEFAULT_HEIGHT: u32 = 240;
const AUDIO_BUFFER_SIZE: usize = 4096;

/// Concrete type for the audio producer to avoid dyn compatibility issues
type AudioProducer = ringbuf::wrap::caching::Caching<Arc<HeapRb<i16>>, true, false>;

/// Desktop implementation of PlatformCallbacks for use with eframe.
struct DesktopPlatform {
    // Graphics state
    framebuffer: Arc<Mutex<Vec<u32>>>,
    width: u32,
    height: u32,

    // Audio state
    audio_producer: AudioProducer,

    // Input state
    gilrs: Gilrs,
    active_gamepad: Option<gilrs::GamepadId>,
    egui_input: Option<egui::InputState>,
}

impl PlatformGraphics for DesktopPlatform {
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
        // Fallback to software presentation in eframe context
        self.present_software_frame(framebuffer, width, height, stride_pixels);
    }

    fn notify_geometry_changed(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

impl PlatformAudio for DesktopPlatform {
    fn audio_batch(&mut self, samples: &[i16]) {
        let _ = self.audio_producer.push_slice(samples);
    }
}

impl PlatformInput for DesktopPlatform {
    fn input_poll(&mut self) {
        while let Some(gilrs::Event { id, .. }) = self.gilrs.next_event() {
            self.active_gamepad = Some(id);
        }
    }

    fn input_button_state(&mut self, _port: u32, button: u32) -> bool {
        // SNES-style button mapping
        let gamepad_pressed = if let Some(id) = self.active_gamepad {
            let gamepad = self.gilrs.gamepad(id);
            match button {
                0 => gamepad.is_pressed(GilrsButton::South),         // B
                1 => gamepad.is_pressed(GilrsButton::West),          // Y
                2 => gamepad.is_pressed(GilrsButton::Select),        // Select
                3 => gamepad.is_pressed(GilrsButton::Start),         // Start
                4 => gamepad.is_pressed(GilrsButton::DPadUp),        // Up
                5 => gamepad.is_pressed(GilrsButton::DPadDown),      // Down
                6 => gamepad.is_pressed(GilrsButton::DPadLeft),      // Left
                7 => gamepad.is_pressed(GilrsButton::DPadRight),     // Right
                8 => gamepad.is_pressed(GilrsButton::East),          // A
                9 => gamepad.is_pressed(GilrsButton::North),         // X
                10 => gamepad.is_pressed(GilrsButton::LeftTrigger),  // L1
                11 => gamepad.is_pressed(GilrsButton::RightTrigger), // R1
                12 => gamepad.is_pressed(GilrsButton::LeftTrigger2), // L2
                13 => gamepad.is_pressed(GilrsButton::RightTrigger2), // R2
                14 => gamepad.is_pressed(GilrsButton::LeftThumb),    // L3
                15 => gamepad.is_pressed(GilrsButton::RightThumb),   // R3
                _ => false,
            }
        } else {
            false
        };

        let key_pressed = if let Some(input) = &self.egui_input {
            match button {
                0 => input.key_down(egui::Key::Z),
                1 => input.key_down(egui::Key::A),
                2 => input.key_down(egui::Key::Space),
                3 => input.key_down(egui::Key::Enter),
                4 => input.key_down(egui::Key::ArrowUp),
                5 => input.key_down(egui::Key::ArrowDown),
                6 => input.key_down(egui::Key::ArrowLeft),
                7 => input.key_down(egui::Key::ArrowRight),
                8 => input.key_down(egui::Key::X),
                9 => input.key_down(egui::Key::S),
                10 => input.key_down(egui::Key::Q),
                11 => input.key_down(egui::Key::W),
                _ => false,
            }
        } else {
            false
        };

        gamepad_pressed || key_pressed
    }

    fn input_key_state(&mut self, _key: u32) -> bool {
        false // Raw key codes not yet implemented in desktop frontend
    }

    fn input_mouse_x(&mut self) -> i32 {
        self.egui_input
            .as_ref()
            .and_then(|i| i.pointer.hover_pos())
            .map(|p| p.x as i32)
            .unwrap_or(0)
    }

    fn input_mouse_y(&mut self) -> i32 {
        self.egui_input
            .as_ref()
            .and_then(|i| i.pointer.hover_pos())
            .map(|p| p.y as i32)
            .unwrap_or(0)
    }

    fn input_mouse_button(&mut self, button: u32) -> bool {
        if let Some(input) = &self.egui_input {
            match button {
                0 => input.pointer.primary_down(),
                1 => input.pointer.secondary_down(),
                2 => input.pointer.middle_down(),
                _ => false,
            }
        } else {
            false
        }
    }
}

impl PlatformCallbacks for DesktopPlatform {}

struct Wasm96App {
    engine: Engine,
    platform: DesktopPlatform,
    texture: Option<egui::TextureHandle>,
    framebuffer: Arc<Mutex<Vec<u32>>>,
    last_frame_time: std::time::Instant,
    loaded_filename: Option<String>,
}

impl Wasm96App {
    fn new(_cc: &eframe::CreationContext<'_>, audio_producer: AudioProducer) -> Self {
        let framebuffer = Arc::new(Mutex::new(vec![
            0u32;
            (DEFAULT_WIDTH * DEFAULT_HEIGHT) as usize
        ]));
        let platform = DesktopPlatform {
            framebuffer: framebuffer.clone(),
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            audio_producer,
            gilrs: Gilrs::new().expect("Failed to initialize Gilrs"),
            active_gamepad: None,
            egui_input: None,
        };

        Self {
            engine: Engine::new(),
            platform,
            texture: None,
            framebuffer,
            last_frame_time: std::time::Instant::now(),
            loaded_filename: None,
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
                self.platform.egui_input = Some(i.clone());
            });

            // Run one frame of the engine
            self.engine.run_frame(&mut self.platform);
        }

        // Display the game output
        egui::CentralPanel::default().show(ctx, |ui| {
            let fb = self.framebuffer.lock().unwrap();
            let width = self.platform.width;
            let height = self.platform.height;

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
                    // Dereference &mut to & and then pass to Image::new
                    ui.add(egui::Image::new(&*texture).shrink_to_fit());
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Load a game from the File menu to start playing.");
                });
            }
        });

        // Keep the UI loop running for smooth gameplay
        ctx.request_repaint();
    }
}

fn main() -> Result<()> {
    // Audio output setup
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("No audio output device found")?;
    let config = device.default_output_config()?;

    let rb = Arc::new(HeapRb::<i16>::new(AUDIO_BUFFER_SIZE * 2));
    let (prod, mut cons) = rb.split();

    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _| {
            for sample in data.iter_mut() {
                if let Some(s) = cons.try_pop() {
                    *sample = s as f32 / 32768.0;
                } else {
                    *sample = 0.0;
                }
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;
    stream.play()?;

    // Application window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([320.0, 240.0])
            .with_title("wasm96"),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "wasm96-desktop",
        options,
        Box::new(|cc| Ok(Box::new(Wasm96App::new(cc, prod)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
