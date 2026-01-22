pub mod mapping;

use eframe::egui;
use gilrs::{GamepadId, Gilrs};
use mapping::{InputConfig, RetroButton};
use wasm96_engine::PlatformInput;

pub struct InputState {
    pub gilrs: Gilrs,
    pub active_gamepads: [Option<GamepadId>; 4],
    pub egui_input: Option<egui::InputState>,
    pub config: InputConfig,
    pub config_dirty: bool,
    pub char_queue: Vec<u8>,
    pub last_rect: Option<egui::Rect>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            gilrs: Gilrs::new().expect("Failed to initialize Gilrs"),
            active_gamepads: [None; 4],
            egui_input: None,
            config: InputConfig::default(),
            config_dirty: false,
            char_queue: Vec::new(),
            last_rect: None,
        }
    }

    pub fn poll(&mut self) {
        while let Some(gilrs::Event { id, event, .. }) = self.gilrs.next_event() {
            match event {
                gilrs::EventType::Connected => {
                    // Automatically assign to first empty port
                    for port in 0..4 {
                        if self.active_gamepads[port].is_none() {
                            self.active_gamepads[port] = Some(id);
                            break;
                        }
                    }
                }
                gilrs::EventType::Disconnected => {
                    for port in 0..4 {
                        if self.active_gamepads[port] == Some(id) {
                            self.active_gamepads[port] = None;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update_egui_input(&mut self, input: egui::InputState, rect: Option<egui::Rect>) {
        for event in &input.events {
            if let egui::Event::Text(text) = event {
                for c in text.chars() {
                    if c.is_ascii() {
                        self.char_queue.push(c as u8);
                    }
                }
            }
        }
        self.egui_input = Some(input);
        self.last_rect = rect;
    }
}

impl PlatformInput for crate::platform::DesktopPlatform {
    fn input_poll(&mut self) {
        self.input.poll();
    }

    fn input_button_state(&mut self, port: u32, button_idx: u32) -> bool {
        if port >= 4 {
            return false;
        }

        let mode = {
            let gs = wasm96_engine::state::global().lock().unwrap();
            gs.input.mode
        };

        let target_button = match button_idx {
            0 => RetroButton::B,
            1 => RetroButton::Y,
            2 => RetroButton::Select,
            3 => RetroButton::Start,
            4 => RetroButton::Up,
            5 => RetroButton::Down,
            6 => RetroButton::Left,
            7 => RetroButton::Right,
            8 => RetroButton::A,
            9 => RetroButton::X,
            10 => RetroButton::L1,
            11 => RetroButton::R1,
            12 => RetroButton::L2,
            13 => RetroButton::R2,
            14 => RetroButton::L3,
            15 => RetroButton::R3,
            _ => return false,
        };

        // 1. Check Gamepad
        let gamepad_pressed = if let Some(id) = self.input.active_gamepads[port as usize] {
            let gamepad = self.input.gilrs.gamepad(id);
            let mapping = &self.input.config.port_mappings[port as usize];

            mapping.pad_map.iter().any(|(gil_btn, retro_btn)| {
                *retro_btn == target_button && gamepad.is_pressed(*gil_btn)
            })
        } else {
            false
        };

        if gamepad_pressed {
            return true;
        }

        // 2. Check Keyboard (only in Game mode or for Port 0)
        if mode == wasm96_engine::state::InputMode::Game && port == 0 {
            if let Some(input) = &self.input.egui_input {
                let mapping = &self.input.config.port_mappings[0];
                return mapping
                    .key_map
                    .iter()
                    .any(|(key, retro_btn)| *retro_btn == target_button && input.key_down(*key));
            }
        }

        false
    }

    fn input_key_state(&mut self, key_code: u32) -> bool {
        let input = match &self.input.egui_input {
            Some(i) => i,
            None => return false,
        };

        let shift = input.modifiers.shift;

        match key_code {
            // Control keys
            8 => input.key_down(egui::Key::Backspace),
            9 => input.key_down(egui::Key::Tab),
            13 => input.key_down(egui::Key::Enter),
            27 => input.key_down(egui::Key::Escape),
            32 => input.key_down(egui::Key::Space),

            // Alphabet (Case sensitive mapping)
            65..=90 => shift && input.key_down(match_letter(key_code)), // A-Z
            97..=122 => !shift && input.key_down(match_letter(key_code - 32)), // a-z

            // Numbers & Symbols (US Layout assumed for is_key_down mapping)
            48 => !shift && input.key_down(egui::Key::Num0),
            41 => shift && input.key_down(egui::Key::Num0), // )
            49 => !shift && input.key_down(egui::Key::Num1),
            33 => shift && input.key_down(egui::Key::Num1), // !
            50 => !shift && input.key_down(egui::Key::Num2),
            64 => shift && input.key_down(egui::Key::Num2), // @
            51 => !shift && input.key_down(egui::Key::Num3),
            35 => shift && input.key_down(egui::Key::Num3), // #
            52 => !shift && input.key_down(egui::Key::Num4),
            36 => shift && input.key_down(egui::Key::Num4), // $
            53 => !shift && input.key_down(egui::Key::Num5),
            37 => {
                (shift && input.key_down(egui::Key::Num5))
                    || (!shift && input.key_down(egui::Key::ArrowLeft))
            } // % or ArrowLeft
            54 => !shift && input.key_down(egui::Key::Num6),
            94 => shift && input.key_down(egui::Key::Num6), // ^
            55 => !shift && input.key_down(egui::Key::Num7),
            38 => {
                (shift && input.key_down(egui::Key::Num7))
                    || (!shift && input.key_down(egui::Key::ArrowUp))
            } // & or ArrowUp
            56 => !shift && input.key_down(egui::Key::Num8),
            42 => shift && input.key_down(egui::Key::Num8), // *
            57 => !shift && input.key_down(egui::Key::Num9),
            40 => {
                (shift && input.key_down(egui::Key::Num9))
                    || (!shift && input.key_down(egui::Key::ArrowDown))
            } // ( or ArrowDown

            // Punctuation
            45 => !shift && input.key_down(egui::Key::Minus),
            95 => shift && input.key_down(egui::Key::Minus), // _
            61 => !shift && input.key_down(egui::Key::Equals),
            43 => shift && input.key_down(egui::Key::Plus), // +
            91 => !shift && input.key_down(egui::Key::OpenBracket),
            123 => shift && input.key_down(egui::Key::OpenBracket), // {
            93 => !shift && input.key_down(egui::Key::CloseBracket),
            125 => shift && input.key_down(egui::Key::CloseBracket), // }
            92 => !shift && input.key_down(egui::Key::Backslash),
            124 => shift && input.key_down(egui::Key::Backslash), // |
            59 => !shift && input.key_down(egui::Key::Semicolon),
            58 => shift && input.key_down(egui::Key::Semicolon), // :
            39 => {
                (!shift && input.key_down(egui::Key::Quote))
                    || (!shift && input.key_down(egui::Key::ArrowRight))
            } // ' or ArrowRight
            34 => shift && input.key_down(egui::Key::Quote),     // "
            44 => !shift && input.key_down(egui::Key::Comma),
            60 => shift && input.key_down(egui::Key::Comma), // <
            46 => !shift && input.key_down(egui::Key::Period),
            62 => shift && input.key_down(egui::Key::Period), // >
            47 => !shift && input.key_down(egui::Key::Slash),
            63 => shift && input.key_down(egui::Key::Slash), // ?
            96 => !shift && input.key_down(egui::Key::Backtick),
            126 => shift && input.key_down(egui::Key::Backtick), // ~

            _ => false,
        }
    }

    fn input_get_char(&mut self) -> Option<u8> {
        if self.input.char_queue.is_empty() {
            None
        } else {
            Some(self.input.char_queue.remove(0))
        }
    }

    fn input_mouse_x(&mut self) -> i32 {
        if let (Some(input), Some(rect)) = (&self.input.egui_input, self.input.last_rect) {
            if let Some(pos) = input.pointer.hover_pos() {
                let local_x = pos.x - rect.min.x;
                let scale_x = {
                    let gs = wasm96_engine::state::global().lock().unwrap();
                    gs.video.width as f32 / rect.width()
                };
                return (local_x * scale_x) as i32;
            }
        }
        0
    }

    fn input_mouse_y(&mut self) -> i32 {
        if let (Some(input), Some(rect)) = (&self.input.egui_input, self.input.last_rect) {
            if let Some(pos) = input.pointer.hover_pos() {
                let local_y = pos.y - rect.min.y;
                let scale_y = {
                    let gs = wasm96_engine::state::global().lock().unwrap();
                    gs.video.height as f32 / rect.height()
                };
                return (local_y * scale_y) as i32;
            }
        }
        0
    }

    fn input_mouse_button(&mut self, button: u32) -> bool {
        if let Some(input) = &self.input.egui_input {
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

fn match_letter(code: u32) -> egui::Key {
    match code {
        65 => egui::Key::A,
        66 => egui::Key::B,
        67 => egui::Key::C,
        68 => egui::Key::D,
        69 => egui::Key::E,
        70 => egui::Key::F,
        71 => egui::Key::G,
        72 => egui::Key::H,
        73 => egui::Key::I,
        74 => egui::Key::J,
        75 => egui::Key::K,
        76 => egui::Key::L,
        77 => egui::Key::M,
        78 => egui::Key::N,
        79 => egui::Key::O,
        80 => egui::Key::P,
        81 => egui::Key::Q,
        82 => egui::Key::R,
        83 => egui::Key::S,
        84 => egui::Key::T,
        85 => egui::Key::U,
        86 => egui::Key::V,
        87 => egui::Key::W,
        88 => egui::Key::X,
        89 => egui::Key::Y,
        90 => egui::Key::Z,
        _ => egui::Key::A,
    }
}
