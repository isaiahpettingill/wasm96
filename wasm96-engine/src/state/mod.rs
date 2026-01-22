//! Core-side shared state.
//!
//! This module owns the host-side state for the WASM runtime host functions.
//!
//! ABI model (Immediate Mode):
//! - Host owns the framebuffer and handles all rendering commands.
//! - Guest issues commands (draw rect, line, etc.) which modify the host framebuffer.
//! - Host presents the framebuffer to the platform frontend at the end of the frame.
//!
//! NOTE: Video pitch may be padded (row stride larger than visible width) on some
//! targets/drivers for compatibility/performance. Keep `width/height` as the visible
//! area and use `pitch_bytes` (or `stride_pixels`) for row stepping.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use glam::Mat4;

#[cfg(not(target_arch = "wasm32"))]
use wasmtime::Memory as WasmtimeMemory;

#[cfg(target_arch = "wasm32")]
use js_sys::WebAssembly::Memory as WebMemory;

// MIDI synthesizer
use midly::{MidiMessage, Smf};

thread_local! {
    /// Thread-local storage for PlatformCallbacks during frame execution.
    ///
    /// This allows WASM imports to access platform callbacks without requiring
    /// them to be passed through Wasmtime's Caller<T> store data.
    static CALLBACKS: RefCell<Option<*mut dyn crate::PlatformCallbacks>> = RefCell::new(None);
}

// Synthesizer structures
#[derive(Clone, Debug)]
pub struct Voice {
    pub note: u8,
    pub velocity: u8,
    pub phase: f32,
    pub envelope_phase: f32,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct Instrument {
    pub waveform: Waveform,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

#[derive(Clone, Debug)]
pub enum Waveform {
    Sine,
    Square,
    Sawtooth,
}

impl Default for Instrument {
    fn default() -> Self {
        Instrument {
            waveform: Waveform::Sine,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
        }
    }
}

#[derive(Debug)]
pub struct Synthesizer {
    pub sample_rate: f32,
    pub voices: Vec<Voice>,
    pub instruments: HashMap<u8, Instrument>,
    pub midi_events: Vec<(u32, MidiMessage)>, // (time in samples, event)
    pub current_time: u32,
}

impl Synthesizer {
    pub fn new(sample_rate: f32) -> Self {
        Synthesizer {
            sample_rate,
            voices: Vec::new(),
            instruments: HashMap::new(),
            midi_events: Vec::new(),
            current_time: 0,
        }
    }

    pub fn load_midi(&mut self, smf: Smf) {
        self.midi_events.clear();
        let mut time = 0u32;
        for track in smf.tracks {
            for event in track {
                time += event.delta.as_int() as u32;
                if let midly::TrackEventKind::Midi { message, .. } = event.kind {
                    self.midi_events.push((time, message));
                }
            }
        }
        self.midi_events.sort_by_key(|(t, _)| *t);
    }

    pub fn generate_sample(&mut self) -> f32 {
        self.current_time += 1;
        let mut sample = 0.0f32;

        // Process MIDI events
        while let Some((time, message)) = self.midi_events.first() {
            if *time <= self.current_time {
                self.handle_midi_message(*message);
                self.midi_events.remove(0);
            } else {
                break;
            }
        }

        // Generate audio from voices
        self.voices.retain(|voice| voice.active);
        for voice in &mut self.voices {
            let freq = 440.0 * 2.0f32.powf((voice.note as f32 - 69.0) / 12.0);
            let phase_inc = freq / self.sample_rate;

            voice.phase += phase_inc;
            if voice.phase >= 1.0 {
                voice.phase -= 1.0;
            }

            let wave_sample = match self
                .instruments
                .get(&0)
                .unwrap_or(&Instrument::default())
                .waveform
            {
                Waveform::Sine => (voice.phase * std::f32::consts::TAU).sin(),
                Waveform::Square => {
                    if voice.phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Waveform::Sawtooth => voice.phase * 2.0 - 1.0,
            };

            // Simple envelope
            let envelope = if voice.envelope_phase < 0.1 {
                voice.envelope_phase / 0.1
            } else {
                0.7
            };
            voice.envelope_phase += 1.0 / self.sample_rate;

            sample += wave_sample * envelope * (voice.velocity as f32 / 127.0);
        }

        sample * 0.1 // Reduce volume
    }

    pub fn handle_midi_message(&mut self, message: MidiMessage) {
        match message {
            MidiMessage::NoteOn { key, vel } => {
                if vel > 0 {
                    self.voices.push(Voice {
                        note: key.as_int(),
                        velocity: vel.as_int(),
                        phase: 0.0,
                        envelope_phase: 0.0,
                        active: true,
                    });
                } else {
                    self.note_off(key.as_int());
                }
            }
            MidiMessage::NoteOff { key, .. } => {
                self.note_off(key.as_int());
            }
            _ => {}
        }
    }

    pub fn note_off(&mut self, note: u8) {
        for voice in &mut self.voices {
            if voice.note == note {
                voice.active = false;
            }
        }
    }
}

/// A single host-side “audio channel” (a.k.a. a mixing voice).
///
/// This is used for higher-level playback APIs (e.g. `play_wav`, `play_ogg`, etc.)
/// where the host decodes and mixes audio. Guests get back an `id` that can be
/// adjusted (volume/pan/loop/stop) without pushing raw samples every frame.
///
/// NOTE: Actual decoding/mixing logic lives elsewhere (e.g. `av`); this is only state.
#[derive(Debug, Clone)]
pub struct AudioChannel {
    /// Whether this channel is currently active/playing.
    pub active: bool,

    /// Channel volume in Q8.8 fixed-point (256 = 1.0x).
    pub volume_q8_8: u32,

    /// Pan in i16 domain: -32768 = full left, 0 = center, 32767 = full right.
    pub pan_i16: i32,

    /// Whether playback should loop when reaching end.
    pub loop_enabled: bool,

    /// Interleaved stereo PCM samples (i16) for this channel.
    ///
    /// This is a simple representation that enables mixing without requiring the
    /// guest to continuously feed audio. Decoders can fill this buffer and reset
    /// `position_frames` as needed.
    pub pcm_stereo: Vec<i16>,

    /// Current playback position in *frames* (not i16 samples).
    /// One frame = 2 i16 samples (L, R).
    pub position_frames: usize,
}

impl Default for AudioChannel {
    fn default() -> Self {
        Self {
            active: false,
            volume_q8_8: 256,
            pan_i16: 0,
            loop_enabled: false,
            pcm_stereo: Vec::new(),
            position_frames: 0,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Context for Wasmtime, including WASI state and the host-backed root.
pub struct PendingCartridge {
    pub data: Vec<u8>,
    pub args: Vec<String>,
    pub stdin: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Wasm96Ctx {
    pub wasi: wasmtime_wasi::p1::WasiP1Ctx,
    pub temp_dir: tempfile::TempDir,
}

#[cfg(target_arch = "wasm32")]
/// Dummy context for the web runtime.
pub struct Wasm96Ctx;

/// Global core state accessed from:
/// - `Engine::run_frame` (to set the current `RuntimeHandle`)
/// - host import functions
pub struct GlobalState {
    /// A cartridge that is scheduled to be loaded on the next frame.
    #[cfg(not(target_arch = "wasm32"))]
    pub pending_cartridge: Option<PendingCartridge>,

    /// Guest linear memory export (`memory`) for the Wasmtime runtime.
    ///
    /// Stored as a raw pointer because the rest of the codebase accesses global state
    /// through a mutex-protected singleton and expects a stable address once set.
    #[cfg(not(target_arch = "wasm32"))]
    pub memory_wasmtime: *const WasmtimeMemory,

    /// Owned copy of the memory handle to ensure the pointer above remains valid.
    #[cfg(not(target_arch = "wasm32"))]
    pub memory_owned: Option<Box<WasmtimeMemory>>,

    /// Guest linear memory export for the web runtime.
    #[cfg(target_arch = "wasm32")]
    pub memory_web: Option<WebMemory>,

    /// Host-owned video state (system memory).
    pub video: VideoState,

    /// Host-owned audio state (system memory).
    pub audio: AudioState,

    /// Cached input state.
    pub input: InputState,

    /// Host-owned storage state (persistent-ish key/value store).
    pub storage: StorageState,

    /// PRNG for math_random functions.
    pub rng: rand::rngs::StdRng,

    /// Seed for noise functions.
    pub noise_seed: u32,

    /// Virtual File System state.
    pub vfs: crate::vfs::VfsState,
}

impl Default for GlobalState {
    fn default() -> Self {
        use rand::SeedableRng;
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pending_cartridge: None,
            #[cfg(not(target_arch = "wasm32"))]
            memory_wasmtime: std::ptr::null(),
            #[cfg(not(target_arch = "wasm32"))]
            memory_owned: None,
            #[cfg(target_arch = "wasm32")]
            memory_web: None,
            video: VideoState::default(),
            audio: AudioState::default(),
            input: InputState::default(),
            storage: StorageState::default(),
            rng: rand::rngs::StdRng::from_entropy(),
            noise_seed: 0,
            vfs: crate::vfs::VfsState::default(),
        }
    }
}

// Raw pointers are used for `handle` and `memory`. We guard access with a mutex.
unsafe impl Send for GlobalState {}
unsafe impl Sync for GlobalState {}

static GLOBAL_STATE: OnceLock<Mutex<GlobalState>> = OnceLock::new();

pub fn global() -> &'static Mutex<GlobalState> {
    GLOBAL_STATE.get_or_init(|| Mutex::new(GlobalState::default()))
}

/// Color mode for color interpretation.
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ColorMode {
    RGB = 0,
    HSL = 1,
}

impl Default for ColorMode {
    fn default() -> Self {
        ColorMode::RGB
    }
}

/// Host-owned framebuffer state for immediate mode drawing.
#[derive(Debug)]
pub struct VideoState {
    /// Visible width (in pixels).
    pub width: u32,
    /// Visible height (in pixels).
    pub height: u32,

    /// Row stride in pixels (may be >= `width`).
    ///
    /// This allows us to align/pad rows for frontends/drivers (e.g. some ARM GPUs)
    /// while keeping visible geometry unchanged.
    pub stride_pixels: u32,

    /// Cached pitch in bytes for XRGB8888 output.
    ///
    /// NOTE: XRGB8888 is 4 bytes/pixel, so `pitch_bytes = stride_pixels * 4`.
    pub pitch_bytes: usize,

    /// Framebuffer pixels (XRGB8888).
    ///
    /// Size is `stride_pixels * height` (NOT `width * height` when padded).
    /// Stored as `u32` for easy pixel manipulation.
    /// Format: 0x00RRGGBB (little endian in memory: BB GG RR 00).
    pub framebuffer: Vec<u32>,

    /// Current drawing color (packed 0xAARRGGBB for ARGB8888).
    pub draw_color: u32,

    /// Fill color (packed 0xAARRGGBB for ARGB8888).
    pub fill_color: u32,

    /// Stroke color (packed 0xAARRGGBB for ARGB8888).
    pub stroke_color: u32,

    /// Whether fill is enabled for filled shapes.
    pub fill_enabled: bool,

    /// Whether stroke is enabled for outlines.
    pub stroke_enabled: bool,

    /// Whether erase mode is enabled (draw with destination alpha blending).
    pub erase_mode_enabled: bool,

    /// Current color mode (RGB or HSL).
    pub color_mode: ColorMode,

    /// Clipping region (x, y, w, h). None means no clipping.
    pub clip_rect: Option<(i32, i32, u32, u32)>,

    /// Current transformation matrix.
    pub transform: Mat4,

    /// Transformation matrix stack.
    pub transform_stack: Vec<Mat4>,

    /// Tracks whether geometry was last communicated to libretro for the current size.
    ///
    /// This is used so higher layers can request `RETRO_ENVIRONMENT_SET_GEOMETRY`
    /// only on changes.
    pub geometry_dirty: bool,
}

impl Default for VideoState {
    fn default() -> Self {
        let width = 320u32;
        let height = 240u32;
        let stride_pixels = width;
        let pitch_bytes = (stride_pixels as usize) * 4;

        Self {
            width, // Default size until set_size is called
            height,
            stride_pixels,
            pitch_bytes,
            framebuffer: vec![0; (stride_pixels * height) as usize],
            draw_color: 0xFFFFFFFF, // Default white with full alpha
            fill_color: 0xFFFFFFFF,
            stroke_color: 0xFFFFFFFF,
            fill_enabled: false,
            stroke_enabled: false,
            erase_mode_enabled: false,
            color_mode: ColorMode::default(),
            clip_rect: None,
            transform: Mat4::IDENTITY,
            transform_stack: Vec::new(),
            geometry_dirty: true,
        }
    }
}

/// Host-owned audio buffer state.
#[derive(Debug)]
pub struct AudioState {
    /// Output sample rate (what libretro expects).
    pub sample_rate: u32,

    /// Guest-provided staging buffer (interleaved i16 stereo).
    ///
    /// This is still supported for “raw push” style audio.
    pub host_queue: Vec<i16>,

    /// Host-mixed playback channels (decoded assets like WAV/QOA/M4A/OGG).
    ///
    /// Guests can trigger playback via higher-level audio APIs and the core will mix
    /// these channels into the output stream.
    pub channels: Vec<AudioChannel>,

    /// MIDI synthesizer.
    pub synthesizer: Option<Synthesizer>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            host_queue: Vec::new(),

            channels: Vec::new(),
            synthesizer: None,
        }
    }
}

/// Host-owned storage state.
///
/// This is a simple in-memory key/value store used by the `storage` ABI.
/// Persistence (e.g. to disk via libretro save APIs) can be added later.
///
/// Keys and values are owned by the host.
#[derive(Debug, Default)]
pub struct StorageState {
    pub kv: HashMap<u64, Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Game,
    Computer,
}

/// Minimal cached input state.
#[derive(Default, Debug)]
pub struct InputState {
    pub mode: InputMode,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_buttons: u32,
}

/// Set the guest memory for the Wasmtime runtime.
#[cfg(not(target_arch = "wasm32"))]
pub fn set_guest_memory_wasmtime(memory: &WasmtimeMemory) {
    let mut s = global().lock().unwrap();
    let boxed = Box::new(*memory);
    s.memory_wasmtime = &*boxed as *const _;
    s.memory_owned = Some(boxed);
}

/// Set the guest memory for the web runtime.
#[cfg(target_arch = "wasm32")]
pub fn set_guest_memory_web(memory: WebMemory) {
    let mut s = global().lock().unwrap();
    s.memory_web = Some(memory);
}

pub fn clear_on_unload() {
    // If a previous panic occurred while holding the global lock, the mutex will be poisoned.
    // We still want to reset state in that case so other tests/frames can proceed.
    let mut s = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        s.memory_wasmtime = std::ptr::null();
        s.memory_owned = None;
    }

    #[cfg(target_arch = "wasm32")]
    {
        s.memory_web = None;
    }

    s.video = VideoState::default();
    s.audio = AudioState::default();
    s.input = InputState::default();
    s.storage = StorageState::default();
}

/// Snapshot of video state for querying without holding the lock.
#[derive(Debug, Clone, Copy)]
pub struct VideoStateSnapshot {
    pub width: u32,
    pub height: u32,
    pub stride_pixels: u32,
    pub geometry_dirty: bool,
}

/// Set the current platform callbacks for this thread.
///
/// SAFETY: The caller must ensure that the pointer remains valid for the duration
/// of the frame execution and that no other thread accesses these callbacks.
pub fn set_callbacks(callbacks: &mut dyn crate::PlatformCallbacks) {
    CALLBACKS.with(|c| {
        // SAFETY: The caller (Engine::run_frame) ensures that:
        // 1. The pointer remains valid for the duration of frame execution
        // 2. No other thread accesses these callbacks during this time
        // 3. clear_callbacks is called before the reference becomes invalid
        *c.borrow_mut() = Some(unsafe {
            std::mem::transmute::<
                &mut dyn crate::PlatformCallbacks,
                *mut dyn crate::PlatformCallbacks,
            >(callbacks)
        });
    });
}

/// Clear the current platform callbacks for this thread.
pub fn clear_callbacks() {
    CALLBACKS.with(|c| {
        *c.borrow_mut() = None;
    });
}

/// Execute a closure with access to the current platform callbacks.
///
/// Returns None if no callbacks are currently set.
pub fn with_callbacks<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut dyn crate::PlatformCallbacks) -> R,
{
    CALLBACKS.with(|c| {
        let callbacks_ptr = *c.borrow();
        callbacks_ptr.map(|ptr| {
            // SAFETY: We trust that the pointer is valid during frame execution.
            // The caller of set_callbacks must ensure this.
            let callbacks = unsafe { &mut *ptr };
            f(callbacks)
        })
    })
}
