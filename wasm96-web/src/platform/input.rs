use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use wasm96_engine::{InputEvent, PlatformInput};
use wasm_bindgen::JsCast;
use web_sys::{Gamepad, GamepadButton, Navigator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetroButton {
    B = 0,
    Y = 1,
    Select = 2,
    Start = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
    A = 8,
    X = 9,
    L1 = 10,
    R1 = 11,
    L2 = 12,
    R2 = 13,
    L3 = 14,
    R3 = 15,
}

impl RetroButton {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::B),
            1 => Some(Self::Y),
            2 => Some(Self::Select),
            3 => Some(Self::Start),
            4 => Some(Self::Up),
            5 => Some(Self::Down),
            6 => Some(Self::Left),
            7 => Some(Self::Right),
            8 => Some(Self::A),
            9 => Some(Self::X),
            10 => Some(Self::L1),
            11 => Some(Self::R1),
            12 => Some(Self::L2),
            13 => Some(Self::R2),
            14 => Some(Self::L3),
            15 => Some(Self::R3),
            _ => None,
        }
    }
}

pub struct WebInput {
    pub state: Rc<RefCell<InputState>>,
    navigator: Option<Navigator>,
    prev_gamepad_buttons: [Vec<bool>; 4],
    // Mapping: KeyCode -> (Port, Button)
    key_map: HashMap<u32, (usize, RetroButton)>,
    // Reverse Mapping for remapping UI if needed, but for now just hardcoded
}

pub struct InputState {
    pub keys_down: HashSet<u32>,
    pub mouse_buttons: HashSet<u32>,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub events: VecDeque<InputEvent>,
    pub chars: VecDeque<char>,
    // Gamepad state storage if we needed continuous poll,
    // but PlatformInput relies on events + query at moment.
    // For joypads, we'll cache state in `WebInput` or `InputState`.
    pub joypad_buttons: [[bool; 16]; 4],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            mouse_buttons: HashSet::new(),
            mouse_x: 0,
            mouse_y: 0,
            events: VecDeque::new(),
            chars: VecDeque::new(),
            joypad_buttons: [[false; 16]; 4],
        }
    }
}

impl WebInput {
    pub fn new(navigator: Option<Navigator>) -> Self {
        let mut key_map = HashMap::new();
        Self::add_default_mappings(&mut key_map);

        Self {
            state: Rc::new(RefCell::new(InputState::new())),
            navigator,
            prev_gamepad_buttons: Default::default(),
            key_map,
        }
    }

    fn add_default_mappings(map: &mut HashMap<u32, (usize, RetroButton)>) {
        // Z=90, X=88 -> B, A (East)
        map.insert(90, (0, RetroButton::B));
        map.insert(88, (0, RetroButton::A));

        // A=65, S=83 -> Y, X (North)
        map.insert(65, (0, RetroButton::Y));
        map.insert(83, (0, RetroButton::X));

        // Enter=13, Space=32 -> Start, Select
        map.insert(13, (0, RetroButton::Start));
        map.insert(32, (0, RetroButton::Select));

        // Arrows
        map.insert(38, (0, RetroButton::Up));
        map.insert(40, (0, RetroButton::Down));
        map.insert(37, (0, RetroButton::Left));
        map.insert(39, (0, RetroButton::Right));

        // Q=81, W=87 -> L1, R1
        map.insert(81, (0, RetroButton::L1));
        map.insert(87, (0, RetroButton::R1));
    }

    pub fn update_mapping(&mut self, key_code: u32, port: usize, button: RetroButton) {
        self.key_map.insert(key_code, (port, button));
    }

    pub fn on_key_down(&mut self, code: &str, key: &str) {
        let key_code = map_code(code);
        let mut state = self.state.borrow_mut();

        if !state.keys_down.contains(&key_code) {
            state.keys_down.insert(key_code);
            state
                .events
                .push_back(InputEvent::KeyPressed { key: key_code });

            if key.len() == 1 {
                if let Some(c) = key.chars().next() {
                    state.chars.push_back(c);
                }
            }

            if let Some((port, btn)) = self.key_map.get(&key_code) {
                let p = *port;
                let b = *btn as usize;
                if p < 4 && b < 16 {
                    state.joypad_buttons[p][b] = true;
                    state.events.push_back(InputEvent::JoypadPressed {
                        port: p as u32,
                        button: b as u32,
                    });
                }
            }
        }
    }

    pub fn on_key_up(&mut self, code: &str) {
        let key_code = map_code(code);
        let mut state = self.state.borrow_mut();

        if state.keys_down.remove(&key_code) {
            state
                .events
                .push_back(InputEvent::KeyReleased { key: key_code });

            if let Some((port, btn)) = self.key_map.get(&key_code) {
                let p = *port;
                let b = *btn as usize;
                if p < 4 && b < 16 {
                    state.joypad_buttons[p][b] = false;
                    state.events.push_back(InputEvent::JoypadReleased {
                        port: p as u32,
                        button: b as u32,
                    });
                }
            }
        }
    }

    pub fn on_mouse_move(&mut self, x: i32, y: i32) {
        let mut state = self.state.borrow_mut();
        state.mouse_x = x;
        state.mouse_y = y;
    }

    pub fn on_mouse_down(&mut self, button: i16, x: i32, y: i32) {
        let mut state = self.state.borrow_mut();
        state.mouse_x = x;
        state.mouse_y = y;
        let btn_u32 = button as u32;
        if !state.mouse_buttons.contains(&btn_u32) {
            state.mouse_buttons.insert(btn_u32);
            state.events.push_back(InputEvent::MousePressed {
                button: btn_u32,
                x,
                y,
            });
        }
    }

    pub fn on_mouse_up(&mut self, button: i16, x: i32, y: i32) {
        let mut state = self.state.borrow_mut();
        state.mouse_x = x;
        state.mouse_y = y;
        let btn_u32 = button as u32;
        if state.mouse_buttons.remove(&btn_u32) {
            state.events.push_back(InputEvent::MouseReleased {
                button: btn_u32,
                x,
                y,
            });
        }
    }
}

impl PlatformInput for WebInput {
    fn input_poll(&mut self) {
        // Poll Gamepads
        if let Some(nav) = &self.navigator {
            if let Ok(gamepads) = nav.get_gamepads() {
                for i in 0..4 {
                    // Previous state
                    if self.prev_gamepad_buttons[i].is_empty() {
                        self.prev_gamepad_buttons[i] = vec![false; 16];
                    }

                    if let Some(gp) = gamepads.get(i as u32).dyn_into::<Gamepad>().ok() {
                        if !gp.connected() {
                            continue;
                        }

                        let buttons = gp.buttons();
                        let mut state = self.state.borrow_mut();

                        // Map standard gamepad layout to RetroButtons
                        // 0: A (South) -> B
                        // 1: B (East) -> A
                        // 2: X (West) -> Y
                        // 3: Y (North) -> X
                        // 4: L1 -> L1
                        // 5: R1 -> R1
                        // 6: L2 -> L2
                        // 7: R2 -> R2
                        // 8: Select -> Select
                        // 9: Start -> Start
                        // 10: L3 -> L3
                        // 11: R3 -> R3
                        // 12: Up -> Up
                        // 13: Down -> Down
                        // 14: Left -> Left
                        // 15: Right -> Right

                        let mapping = [
                            (0, RetroButton::B),
                            (1, RetroButton::A),
                            (2, RetroButton::Y),
                            (3, RetroButton::X),
                            (4, RetroButton::L1),
                            (5, RetroButton::R1),
                            (6, RetroButton::L2),
                            (7, RetroButton::R2),
                            (8, RetroButton::Select),
                            (9, RetroButton::Start),
                            (10, RetroButton::L3),
                            (11, RetroButton::R3),
                            (12, RetroButton::Up),
                            (13, RetroButton::Down),
                            (14, RetroButton::Left),
                            (15, RetroButton::Right),
                        ];

                        for (gp_idx, retro_btn) in mapping {
                            let idx = gp_idx as u32;
                            if idx < buttons.length() {
                                let btn: GamepadButton = buttons.get(idx).unchecked_into();
                                let pressed = btn.pressed();
                                let retro_idx = retro_btn as usize;

                                if pressed != self.prev_gamepad_buttons[i][retro_idx] {
                                    // Update internal state
                                    state.joypad_buttons[i][retro_idx] = pressed;
                                    self.prev_gamepad_buttons[i][retro_idx] = pressed;

                                    if pressed {
                                        state.events.push_back(InputEvent::JoypadPressed {
                                            port: i as u32,
                                            button: retro_idx as u32,
                                        });
                                    } else {
                                        state.events.push_back(InputEvent::JoypadReleased {
                                            port: i as u32,
                                            button: retro_idx as u32,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn input_get_event(&mut self) -> Option<InputEvent> {
        self.state.borrow_mut().events.pop_front()
    }

    fn input_button_state(&mut self, port: u32, button: u32) -> bool {
        let state = self.state.borrow();
        if (port as usize) < 4 && (button as usize) < 16 {
            state.joypad_buttons[port as usize][button as usize]
        } else {
            false
        }
    }

    fn input_key_state(&mut self, key: u32) -> bool {
        self.state.borrow().keys_down.contains(&key)
    }

    fn input_get_char(&mut self) -> Option<u8> {
        self.state.borrow_mut().chars.pop_front().map(|c| c as u8)
    }

    fn input_mouse_x(&mut self) -> i32 {
        self.state.borrow().mouse_x
    }

    fn input_mouse_y(&mut self) -> i32 {
        self.state.borrow().mouse_y
    }

    fn input_mouse_button(&mut self, button: u32) -> bool {
        self.state.borrow().mouse_buttons.contains(&button)
    }
}

pub fn map_code(code: &str) -> u32 {
    match code {
        "Backspace" => 8,
        "Tab" => 9,
        "Enter" => 13,
        "ShiftLeft" | "ShiftRight" => 16,
        "ControlLeft" | "ControlRight" => 17,
        "AltLeft" | "AltRight" => 18,
        "Escape" => 27,
        "Space" => 32,
        "ArrowLeft" => 37,
        "ArrowUp" => 38,
        "ArrowRight" => 39,
        "ArrowDown" => 40,
        "Digit0" => 48,
        "Digit1" => 49,
        "Digit2" => 50,
        "Digit3" => 51,
        "Digit4" => 52,
        "Digit5" => 53,
        "Digit6" => 54,
        "Digit7" => 55,
        "Digit8" => 56,
        "Digit9" => 57,
        "KeyA" => 65,
        "KeyB" => 66,
        "KeyC" => 67,
        "KeyD" => 68,
        "KeyE" => 69,
        "KeyF" => 70,
        "KeyG" => 71,
        "KeyH" => 72,
        "KeyI" => 73,
        "KeyJ" => 74,
        "KeyK" => 75,
        "KeyL" => 76,
        "KeyM" => 77,
        "KeyN" => 78,
        "KeyO" => 79,
        "KeyP" => 80,
        "KeyQ" => 81,
        "KeyR" => 82,
        "KeyS" => 83,
        "KeyT" => 84,
        "KeyU" => 85,
        "KeyV" => 86,
        "KeyW" => 87,
        "KeyX" => 88,
        "KeyY" => 89,
        "KeyZ" => 90,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_code() {
        assert_eq!(map_code("Backspace"), 8);
        assert_eq!(map_code("Tab"), 9);
        assert_eq!(map_code("Enter"), 13);
        assert_eq!(map_code("ShiftLeft"), 16);
        assert_eq!(map_code("ControlLeft"), 17);
        assert_eq!(map_code("AltLeft"), 18);
        assert_eq!(map_code("Escape"), 27);
        assert_eq!(map_code("Space"), 32);
        assert_eq!(map_code("ArrowLeft"), 37);
        assert_eq!(map_code("ArrowUp"), 38);
        assert_eq!(map_code("ArrowRight"), 39);
        assert_eq!(map_code("ArrowDown"), 40);
        assert_eq!(map_code("Digit0"), 48);
        assert_eq!(map_code("KeyA"), 65);
        assert_eq!(map_code("KeyZ"), 90);
        assert_eq!(map_code("UnknownKey"), 0);
    }
}
