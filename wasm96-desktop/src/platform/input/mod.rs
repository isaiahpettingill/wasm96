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
}

impl InputState {
    pub fn new() -> Self {
        Self {
            gilrs: Gilrs::new().expect("Failed to initialize Gilrs"),
            active_gamepads: [None; 4],
            egui_input: None,
            config: InputConfig::default(),
            config_dirty: false,
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

    pub fn update_egui_input(&mut self, input: egui::InputState) {
        self.egui_input = Some(input);
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

        // In Computer mode, we treat keyboard/mouse as raw inputs.
        // If the guest is asking for Joypad buttons in Computer mode,
        // we only allow them if they come from a physical gamepad,
        // otherwise we might conflict with raw keyboard usage.

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
        // Libretro style: Port 0 usually maps to keyboard if no gamepad is present.
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
        // In Game mode, we might want to disable raw keyboard to prevent "cheating"
        // or unexpected behavior, but for now we'll allow it if implemented.
        // We use egui Key mapping for simplicity in the desktop version.

        if let Some(input) = &self.input.egui_input {
            // key_code here is assumed to be an egui::Key index or similar.
            // For a real implementation, we'd need a stable mapping.
            // Since egui::Key is an enum, we'll try to treat the u32 as an index.
            if let Some(key) = egui::Key::from_index(key_code as usize) {
                return input.key_down(key);
            }
        }
        false
    }

    fn input_mouse_x(&mut self) -> i32 {
        self.input
            .egui_input
            .as_ref()
            .and_then(|i| i.pointer.hover_pos())
            .map(|p| p.x as i32)
            .unwrap_or(0)
    }

    fn input_mouse_y(&mut self) -> i32 {
        self.input
            .egui_input
            .as_ref()
            .and_then(|i| i.pointer.hover_pos())
            .map(|p| p.y as i32)
            .unwrap_or(0)
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

/// Helper for egui::Key to index conversion if missing in older egui
trait KeyExt {
    fn from_index(index: usize) -> Option<egui::Key>;
}

impl KeyExt for egui::Key {
    fn from_index(index: usize) -> Option<egui::Key> {
        use egui::Key::*;
        const KEYS: [egui::Key; 71] = [
            ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Escape, Tab, Backspace, Enter, Space,
            Insert, Delete, Home, End, PageUp, PageDown, A, B, C, D, E, F, G, H, I, J, K, L, M, N,
            O, P, Q, R, S, T, U, V, W, X, Y, Z, Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7,
            Num8, Num9, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17,
            F18, F19, F20,
        ];
        KEYS.get(index).copied()
    }
}
