use eframe::egui;
use gilrs::{Button as GilrsButton, Gilrs};
use wasm96_engine::PlatformInput;

pub struct InputState {
    pub gilrs: Gilrs,
    pub active_gamepad: Option<gilrs::GamepadId>,
    pub egui_input: Option<egui::InputState>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            gilrs: Gilrs::new().expect("Failed to initialize Gilrs"),
            active_gamepad: None,
            egui_input: None,
        }
    }

    pub fn poll(&mut self) {
        while let Some(gilrs::Event { id, .. }) = self.gilrs.next_event() {
            self.active_gamepad = Some(id);
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

    fn input_button_state(&mut self, _port: u32, button: u32) -> bool {
        // SNES-style button mapping
        let gamepad_pressed = if let Some(id) = self.input.active_gamepad {
            let gamepad = self.input.gilrs.gamepad(id);
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

        let key_pressed = if let Some(input) = &self.input.egui_input {
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
