use eframe::egui;
use gilrs::Button as GilrsButton;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Retropad-style buttons as defined in the ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub const ALL: [RetroButton; 16] = [
        RetroButton::B,
        RetroButton::Y,
        RetroButton::Select,
        RetroButton::Start,
        RetroButton::Up,
        RetroButton::Down,
        RetroButton::Left,
        RetroButton::Right,
        RetroButton::A,
        RetroButton::X,
        RetroButton::L1,
        RetroButton::R1,
        RetroButton::L2,
        RetroButton::R2,
        RetroButton::L3,
        RetroButton::R3,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            RetroButton::B => "B (South)",
            RetroButton::Y => "Y (West)",
            RetroButton::Select => "Select",
            RetroButton::Start => "Start",
            RetroButton::Up => "D-Pad Up",
            RetroButton::Down => "D-Pad Down",
            RetroButton::Left => "D-Pad Left",
            RetroButton::Right => "D-Pad Right",
            RetroButton::A => "A (East)",
            RetroButton::X => "X (North)",
            RetroButton::L1 => "L1 (LB)",
            RetroButton::R1 => "R1 (RB)",
            RetroButton::L2 => "L2 (LT)",
            RetroButton::R2 => "R2 (RT)",
            RetroButton::L3 => "L3 (LSB)",
            RetroButton::R3 => "R3 (RSB)",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ControllerMapping {
    pub key_map: HashMap<egui::Key, RetroButton>,
    pub pad_map: HashMap<GilrsButton, RetroButton>,
}

impl ControllerMapping {
    pub fn default_for_port(port: usize) -> Self {
        let mut pad_map = HashMap::new();
        pad_map.insert(GilrsButton::South, RetroButton::B);
        pad_map.insert(GilrsButton::West, RetroButton::Y);
        pad_map.insert(GilrsButton::Select, RetroButton::Select);
        pad_map.insert(GilrsButton::Start, RetroButton::Start);
        pad_map.insert(GilrsButton::DPadUp, RetroButton::Up);
        pad_map.insert(GilrsButton::DPadDown, RetroButton::Down);
        pad_map.insert(GilrsButton::DPadLeft, RetroButton::Left);
        pad_map.insert(GilrsButton::DPadRight, RetroButton::Right);
        pad_map.insert(GilrsButton::East, RetroButton::A);
        pad_map.insert(GilrsButton::North, RetroButton::X);
        pad_map.insert(GilrsButton::LeftTrigger, RetroButton::L1);
        pad_map.insert(GilrsButton::RightTrigger, RetroButton::R1);
        pad_map.insert(GilrsButton::LeftTrigger2, RetroButton::L2);
        pad_map.insert(GilrsButton::RightTrigger2, RetroButton::R2);
        pad_map.insert(GilrsButton::LeftThumb, RetroButton::L3);
        pad_map.insert(GilrsButton::RightThumb, RetroButton::R3);

        if port == 0 {
            let mut key_map = HashMap::new();
            key_map.insert(egui::Key::Z, RetroButton::B);
            key_map.insert(egui::Key::A, RetroButton::Y);
            key_map.insert(egui::Key::Space, RetroButton::Select);
            key_map.insert(egui::Key::Enter, RetroButton::Start);
            key_map.insert(egui::Key::ArrowUp, RetroButton::Up);
            key_map.insert(egui::Key::ArrowDown, RetroButton::Down);
            key_map.insert(egui::Key::ArrowLeft, RetroButton::Left);
            key_map.insert(egui::Key::ArrowRight, RetroButton::Right);
            key_map.insert(egui::Key::X, RetroButton::A);
            key_map.insert(egui::Key::S, RetroButton::X);
            key_map.insert(egui::Key::Q, RetroButton::L1);
            key_map.insert(egui::Key::W, RetroButton::R1);

            Self { key_map, pad_map }
        } else {
            Self {
                key_map: HashMap::new(),
                pad_map,
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ControllerMappingWire {
    key_map: Vec<(String, RetroButton)>,
    pad_map: Vec<(String, RetroButton)>,
}

impl From<&ControllerMapping> for ControllerMappingWire {
    fn from(m: &ControllerMapping) -> Self {
        Self {
            key_map: m
                .key_map
                .iter()
                .map(|(k, v)| (format!("{:?}", k), *v))
                .collect(),
            pad_map: m
                .pad_map
                .iter()
                .map(|(k, v)| (format!("{:?}", k), *v))
                .collect(),
        }
    }
}

impl From<ControllerMappingWire> for ControllerMapping {
    fn from(w: ControllerMappingWire) -> Self {
        let mut key_map = HashMap::new();
        for (k_str, v) in w.key_map {
            if let Some(key) = parse_egui_key(&k_str) {
                key_map.insert(key, v);
            }
        }
        let mut pad_map = HashMap::new();
        for (p_str, v) in w.pad_map {
            if let Some(btn) = parse_gilrs_button(&p_str) {
                pad_map.insert(btn, v);
            }
        }
        Self { key_map, pad_map }
    }
}

#[derive(Debug, Clone)]
pub struct InputConfig {
    pub port_mappings: [ControllerMapping; 4],
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            port_mappings: [
                ControllerMapping::default_for_port(0),
                ControllerMapping::default_for_port(1),
                ControllerMapping::default_for_port(2),
                ControllerMapping::default_for_port(3),
            ],
        }
    }
}

#[derive(Serialize, Deserialize)]
struct InputConfigWire {
    port_mappings: Vec<ControllerMappingWire>,
}

impl InputConfig {
    pub fn load_from_bytes(bytes: &[u8]) -> Option<Self> {
        let wire: InputConfigWire = serde_json::from_slice(bytes).ok()?;
        let mut config = Self::default();
        for (i, w) in wire.port_mappings.into_iter().enumerate() {
            if i < 4 {
                config.port_mappings[i] = ControllerMapping::from(w);
            }
        }
        Some(config)
    }

    pub fn save_to_vec(&self) -> Vec<u8> {
        let wire = InputConfigWire {
            port_mappings: self
                .port_mappings
                .iter()
                .map(|m| ControllerMappingWire::from(m))
                .collect(),
        };
        serde_json::to_vec_pretty(&wire).unwrap_or_default()
    }
}

fn parse_egui_key(s: &str) -> Option<egui::Key> {
    use egui::Key::*;
    let keys = [
        ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Escape, Tab, Backspace, Enter, Space, Insert,
        Delete, Home, End, PageUp, PageDown, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R,
        S, T, U, V, W, X, Y, Z, Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9, F1, F2,
        F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
    ];
    for k in keys {
        if format!("{:?}", k) == s {
            return Some(k);
        }
    }
    None
}

fn parse_gilrs_button(s: &str) -> Option<GilrsButton> {
    use GilrsButton::*;
    let buttons = [
        South,
        East,
        North,
        West,
        C,
        Z,
        LeftTrigger,
        RightTrigger,
        LeftTrigger2,
        RightTrigger2,
        Select,
        Start,
        Mode,
        LeftThumb,
        RightThumb,
        DPadUp,
        DPadDown,
        DPadLeft,
        DPadRight,
    ];
    for b in buttons {
        if format!("{:?}", b) == s {
            return Some(b);
        }
    }
    None
}
