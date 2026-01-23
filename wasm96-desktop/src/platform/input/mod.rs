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
    pub event_queue: Vec<wasm96_engine::InputEvent>,
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
            event_queue: Vec::new(),
            last_rect: None,
        }
    }

    pub fn poll(&mut self) {
        while let Some(gilrs::Event { id, event, .. }) = self.gilrs.next_event() {
            let port_opt = self.active_gamepads.iter().position(|&p| p == Some(id));

            match event {
                gilrs::EventType::ButtonPressed(btn, _) => {
                    if let Some(port) = port_opt {
                        let mapping = &self.config.port_mappings[port];
                        if let Some(retro_btn) = mapping.pad_map.get(&btn) {
                            self.event_queue
                                .push(wasm96_engine::InputEvent::JoypadPressed {
                                    port: port as u32,
                                    button: *retro_btn as u32,
                                });
                        }
                    }
                }
                gilrs::EventType::ButtonReleased(btn, _) => {
                    if let Some(port) = port_opt {
                        let mapping = &self.config.port_mappings[port];
                        if let Some(retro_btn) = mapping.pad_map.get(&btn) {
                            self.event_queue
                                .push(wasm96_engine::InputEvent::JoypadReleased {
                                    port: port as u32,
                                    button: *retro_btn as u32,
                                });
                        }
                    }
                }
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
            match event {
                egui::Event::Text(text) => {
                    for c in text.chars() {
                        if c.is_ascii() {
                            self.char_queue.push(c as u8);
                        }
                    }
                }
                egui::Event::Key {
                    key, pressed: true, ..
                } => {
                    if let Some(code) = self.egui_key_to_wasm96(*key, input.modifiers.shift) {
                        self.event_queue
                            .push(wasm96_engine::InputEvent::KeyPressed { key: code });
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: false,
                    ..
                } => {
                    if let Some(code) = self.egui_key_to_wasm96(*key, input.modifiers.shift) {
                        self.event_queue
                            .push(wasm96_engine::InputEvent::KeyReleased { key: code });
                    }
                }
                egui::Event::PointerButton {
                    button,
                    pressed: true,
                    pos,
                    ..
                } => {
                    let (x, y) = if let Some(r) = rect {
                        let local_x = pos.x - r.min.x;
                        let local_y = pos.y - r.min.y;
                        let gs = wasm96_engine::state::global().lock().unwrap();
                        let scale_x = gs.video.width as f32 / r.width();
                        let scale_y = gs.video.height as f32 / r.height();
                        ((local_x * scale_x) as i32, (local_y * scale_y) as i32)
                    } else {
                        (0, 0)
                    };
                    let btn_idx = match button {
                        egui::PointerButton::Primary => 0,
                        egui::PointerButton::Secondary => 1,
                        egui::PointerButton::Middle => 2,
                        _ => 0,
                    };
                    self.event_queue
                        .push(wasm96_engine::InputEvent::MousePressed {
                            button: btn_idx,
                            x,
                            y,
                        });
                }
                _ => {}
            }
        }
        self.egui_input = Some(input);
        self.last_rect = rect;
    }

    fn egui_key_to_wasm96(&self, key: egui::Key, shift: bool) -> Option<u32> {
        match key {
            egui::Key::Backspace => Some(8),
            egui::Key::Tab => Some(9),
            egui::Key::Enter => Some(13),
            egui::Key::Escape => Some(27),
            egui::Key::Space => Some(32),

            egui::Key::A => Some(if shift { 65 } else { 97 }),
            egui::Key::B => Some(if shift { 66 } else { 98 }),
            egui::Key::C => Some(if shift { 67 } else { 99 }),
            egui::Key::D => Some(if shift { 68 } else { 100 }),
            egui::Key::E => Some(if shift { 69 } else { 101 }),
            egui::Key::F => Some(if shift { 70 } else { 102 }),
            egui::Key::G => Some(if shift { 71 } else { 103 }),
            egui::Key::H => Some(if shift { 72 } else { 104 }),
            egui::Key::I => Some(if shift { 73 } else { 105 }),
            egui::Key::J => Some(if shift { 74 } else { 106 }),
            egui::Key::K => Some(if shift { 75 } else { 107 }),
            egui::Key::L => Some(if shift { 76 } else { 108 }),
            egui::Key::M => Some(if shift { 77 } else { 109 }),
            egui::Key::N => Some(if shift { 78 } else { 110 }),
            egui::Key::O => Some(if shift { 79 } else { 111 }),
            egui::Key::P => Some(if shift { 80 } else { 112 }),
            egui::Key::Q => Some(if shift { 81 } else { 113 }),
            egui::Key::R => Some(if shift { 82 } else { 114 }),
            egui::Key::S => Some(if shift { 83 } else { 115 }),
            egui::Key::T => Some(if shift { 84 } else { 116 }),
            egui::Key::U => Some(if shift { 85 } else { 117 }),
            egui::Key::V => Some(if shift { 86 } else { 118 }),
            egui::Key::W => Some(if shift { 87 } else { 119 }),
            egui::Key::X => Some(if shift { 88 } else { 120 }),
            egui::Key::Y => Some(if shift { 89 } else { 121 }),
            egui::Key::Z => Some(if shift { 90 } else { 122 }),

            egui::Key::Num0 => Some(if shift { 41 } else { 48 }),
            egui::Key::Num1 => Some(if shift { 33 } else { 49 }),
            egui::Key::Num2 => Some(if shift { 64 } else { 50 }),
            egui::Key::Num3 => Some(if shift { 35 } else { 51 }),
            egui::Key::Num4 => Some(if shift { 36 } else { 52 }),
            egui::Key::Num5 => Some(if shift { 37 } else { 53 }),
            egui::Key::Num6 => Some(if shift { 94 } else { 54 }),
            egui::Key::Num7 => Some(if shift { 38 } else { 55 }),
            egui::Key::Num8 => Some(if shift { 42 } else { 56 }),
            egui::Key::Num9 => Some(if shift { 40 } else { 57 }),

            egui::Key::Minus => Some(if shift { 95 } else { 45 }),
            egui::Key::Equals => Some(if shift { 43 } else { 61 }),
            egui::Key::OpenBracket => Some(if shift { 123 } else { 91 }),
            egui::Key::CloseBracket => Some(if shift { 125 } else { 93 }),
            egui::Key::Backslash => Some(if shift { 124 } else { 92 }),
            egui::Key::Semicolon => Some(if shift { 58 } else { 59 }),
            egui::Key::Quote => Some(if shift { 34 } else { 39 }),
            egui::Key::Comma => Some(if shift { 60 } else { 44 }),
            egui::Key::Period => Some(if shift { 62 } else { 46 }),
            egui::Key::Slash => Some(if shift { 63 } else { 47 }),
            egui::Key::Backtick => Some(if shift { 126 } else { 96 }),

            egui::Key::ArrowLeft => Some(37),
            egui::Key::ArrowUp => Some(38),
            egui::Key::ArrowRight => Some(39),
            egui::Key::ArrowDown => Some(40),

            _ => None,
        }
    }
}

impl PlatformInput for crate::platform::DesktopPlatform {
    fn input_poll(&mut self) {
        self.input.poll();
    }

    fn input_get_event(&mut self) -> Option<wasm96_engine::InputEvent> {
        if self.input.event_queue.is_empty() {
            None
        } else {
            Some(self.input.event_queue.remove(0))
        }
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
