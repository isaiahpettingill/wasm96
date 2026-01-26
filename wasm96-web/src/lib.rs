mod platform;

use platform::audio::WebAudio;
use platform::input::{map_code, RetroButton, WebInput};
use platform::WebPlatform;
use std::cell::RefCell;
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use std::sync::Arc;
use wasm96_engine::{Engine, PlatformGraphics};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, Window};
use yew::prelude::*;

#[derive(Properties, PartialEq, Default)]
pub struct AppProps {}

pub enum Msg {
    Tick,
    KeyDown(KeyboardEvent),
    KeyUp(KeyboardEvent),
    MouseMove(MouseEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    Loaded(Vec<u8>),
    Error(String),
    StartRemap(usize, RetroButton),
}

pub struct App {
    engine: Option<Rc<RefCell<Engine>>>,
    platform: Option<Rc<RefCell<WebPlatform>>>,
    canvas_ref: NodeRef,
    _loop_handle: Option<i32>,
    window: Window,
    remap_target: Option<(usize, RetroButton)>,
    last_frame_time: f64,
}

impl Component for App {
    type Message = Msg;
    type Properties = AppProps;

    fn create(_ctx: &Context<Self>) -> Self {
        wasm_logger::init(wasm_logger::Config::default());
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        let window = web_sys::window().expect("no global `window` exists");

        Self {
            engine: None,
            platform: None,
            canvas_ref: NodeRef::default(),
            _loop_handle: None,
            window,
            remap_target: None,
            last_frame_time: 0.0,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => {
                let now = self
                    .window
                    .performance()
                    .map(|p| p.now())
                    .unwrap_or_else(|| js_sys::Date::now());

                let target_interval = 1000.0 / 60.0;
                let elapsed = now - self.last_frame_time;

                if elapsed >= target_interval {
                    // Update timestamp, adjusting for slight overshoot to keep phase
                    // If elapsed is very large (e.g. first frame or tab inactive), just reset to now
                    if elapsed > 1000.0 {
                        self.last_frame_time = now;
                    } else {
                        self.last_frame_time = now - (elapsed % target_interval);
                    }

                    if let (Some(engine), Some(platform)) = (&self.engine, &self.platform) {
                        let mut eng = engine.borrow_mut();
                        let mut plat = platform.borrow_mut();

                        // WGPU surface handling
                        let mut surface_texture = None;
                        let surface_opt = plat.surface.take();

                        if let Some(surface) = &surface_opt {
                            if let Ok(frame) = surface.get_current_texture() {
                                let view = frame
                                    .texture
                                    .create_view(&wgpu::TextureViewDescriptor::default());
                                let width = frame.texture.width();
                                let height = frame.texture.height();

                                plat.set_wgpu_view(std::sync::Arc::new(view), width, height);
                                surface_texture = Some(frame);
                            }
                        }

                        eng.run_frame(&mut *plat);
                        plat.surface = surface_opt;

                        if let Some(frame) = surface_texture {
                            frame.present();
                        }
                    }
                }

                // Schedule next tick
                let link = ctx.link().clone();
                let closure = Closure::once(move || link.send_message(Msg::Tick));
                let _ = self
                    .window
                    .request_animation_frame(closure.as_ref().unchecked_ref());
                closure.forget();

                false
            }
            Msg::Loaded(bytes) => {
                if let Some(engine) = &self.engine {
                    log::info!("Cartridge loaded: {} bytes", bytes.len());
                    // Reset engine before loading new cartridge
                    // engine.borrow_mut().reset(); // Reset not fully exposed or needed if we just overwrite?

                    // The engine might need re-initialization logic if loading multiple times,
                    // but for now we just load.
                    match engine.borrow_mut().load_game_from_bytes(&bytes) {
                        Ok(_) => log::info!("Game loaded successfully"),
                        Err(e) => log::error!("Failed to load game: {:?}", e),
                    }
                }
                true
            }
            Msg::KeyDown(e) => {
                if let Some(target) = self.remap_target {
                    if let Some(platform) = &self.platform {
                        let mut plat = platform.borrow_mut();
                        let code = map_code(&e.code());
                        if code != 0 {
                            plat.input.update_mapping(code, target.0, target.1);
                            self.remap_target = None;
                            log::info!("Remapped {:?} to key code {}", target, code);
                        }
                    }
                    true
                } else {
                    if let Some(platform) = &self.platform {
                        let mut plat = platform.borrow_mut();
                        plat.input.on_key_down(&e.code(), &e.key());

                        // Resume audio context on user interaction
                        plat.audio.resume();
                    }
                    false
                }
            }
            Msg::KeyUp(e) => {
                if let Some(platform) = &self.platform {
                    let mut plat = platform.borrow_mut();
                    plat.input.on_key_up(&e.code());
                }
                false
            }
            Msg::MouseMove(e) => {
                if let Some(platform) = &self.platform {
                    let x = e.offset_x();
                    let y = e.offset_y();

                    let mut plat = platform.borrow_mut();
                    plat.input.on_mouse_move(x, y);
                }
                false
            }
            Msg::MouseDown(e) => {
                if let Some(platform) = &self.platform {
                    let x = e.offset_x();
                    let y = e.offset_y();

                    let mut plat = platform.borrow_mut();
                    plat.input.on_mouse_down(e.button(), x, y);

                    // Resume audio context on user interaction
                    plat.audio.resume();
                }
                false
            }
            Msg::MouseUp(e) => {
                if let Some(platform) = &self.platform {
                    let x = e.offset_x();
                    let y = e.offset_y();

                    let mut plat = platform.borrow_mut();
                    plat.input.on_mouse_up(e.button(), x, y);
                }
                false
            }
            Msg::Error(e) => {
                log::error!("App Error: {}", e);
                true
            }
            Msg::StartRemap(port, btn) => {
                self.remap_target = Some((port, btn));
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link().clone();
        html! {
            <div class="wasm96-player"
                 tabindex="0"
                 onkeydown={link.callback(|e: KeyboardEvent| Msg::KeyDown(e))}
                 onkeyup={link.callback(|e: KeyboardEvent| Msg::KeyUp(e))}
                 style="outline: none;">

                <div class="toolbar">
                    <input type="file" id="cartridge-input" accept=".w96,.wasm"
                        onchange={
                            let link_clone = link.clone();
                            link.callback(move |e: Event| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let link = link_clone.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let array_buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                                                .await
                                                .expect("array buffer");
                                            let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                                            let vec = uint8_array.to_vec();
                                            link.send_message(Msg::Loaded(vec));
                                        });
                                    }
                                }
                                Msg::Tick // Just to trigger something, though async handles the load
                            })
                        }
                    />
                </div>

                <canvas ref={self.canvas_ref.clone()}
                        width="320" height="240"
                        onmousemove={link.callback(|e: MouseEvent| Msg::MouseMove(e))}
                        onmousedown={link.callback(|e: MouseEvent| Msg::MouseDown(e))}
                        onmouseup={link.callback(|e: MouseEvent| Msg::MouseUp(e))}
                        style="image-rendering: pixelated; width: 640px; height: 480px; background: #000;">
                </canvas>
                { self.view_controls(ctx) }
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            if let Some(canvas) = self.canvas_ref.cast::<HtmlCanvasElement>() {
                // Initialize Audio
                let audio = match WebAudio::new() {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("Failed to init audio: {:?}", e);
                        return;
                    }
                };

                // Initialize Input
                let nav = self.window.navigator();
                let input = WebInput::new(Some(nav));

                // Initialize Platform
                let platform =
                    Rc::new(RefCell::new(WebPlatform::new(canvas.clone(), audio, input)));
                self.platform = Some(platform.clone());

                // Initialize Engine
                let engine = Rc::new(RefCell::new(Engine::new()));
                self.engine = Some(engine.clone());

                // Initialize WGPU (Async)
                #[cfg(all(target_arch = "wasm32", any(feature = "webgpu", feature = "webgl")))]
                {
                    let platform_clone = platform.clone();
                    let engine_clone = engine.clone();
                    let canvas_clone = canvas.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        // Try to init WGPU
                        let instance = wgpu::Instance::default();
                        if let Ok(surface) =
                            instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas_clone))
                        {
                            let adapter_opt = instance
                                .request_adapter(&wgpu::RequestAdapterOptions {
                                    power_preference: wgpu::PowerPreference::default(),
                                    compatible_surface: Some(&surface),
                                    force_fallback_adapter: false,
                                })
                                .await;

                            if let Some(adapter) = adapter_opt {
                                let (device, queue) = adapter
                                    .request_device(
                                        &wgpu::DeviceDescriptor {
                                            label: None,
                                            required_features: wgpu::Features::empty(),
                                            required_limits:
                                                wgpu::Limits::downlevel_webgl2_defaults()
                                                    .using_resolution(adapter.limits()),
                                        },
                                        None,
                                    )
                                    .await
                                    .unwrap();

                                let device = Arc::new(device);
                                let queue = Arc::new(queue);

                                // Let the platform know about WGPU
                                let mut plat = platform_clone.borrow_mut();
                                let format = surface.get_capabilities(&adapter).formats[0];
                                plat.init_wgpu(device.clone(), queue.clone(), format);
                                plat.surface = Some(surface);

                                // Let the engine know about WGPU
                                let mut eng = engine_clone.borrow_mut();
                                eng.init_wgpu(device, queue, format);
                            }
                        }
                    });
                }

                // Start loop
                ctx.link().send_message(Msg::Tick);
            }
        }
    }
}

impl App {
    fn view_controls(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let buttons = [
            ("Up", RetroButton::Up),
            ("Down", RetroButton::Down),
            ("Left", RetroButton::Left),
            ("Right", RetroButton::Right),
            ("A", RetroButton::A),
            ("B", RetroButton::B),
            ("X", RetroButton::X),
            ("Y", RetroButton::Y),
            ("Start", RetroButton::Start),
            ("Select", RetroButton::Select),
            ("L1", RetroButton::L1),
            ("R1", RetroButton::R1),
        ];

        html! {
            <div class="controls-section" style="margin-top: 20px; color: white; font-family: monospace;">
                <h3>{ "Controls (Port 0)" }</h3>
                <div class="controls-grid" style="display: grid; grid-template-columns: auto auto; gap: 10px; max-width: 300px;">
                    {
                        for buttons.iter().map(|(name, btn)| {
                            let btn = *btn;
                            let is_remapping = self.remap_target == Some((0, btn));
                            let label = if is_remapping { "Press Key..." } else { "Remap" };
                            html! {
                                <>
                                    <span>{ name }</span>
                                    <button onclick={link.callback(move |_| Msg::StartRemap(0, btn))}
                                            disabled={self.remap_target.is_some()}
                                            tabindex="-1">
                                        { label }
                                    </button>
                                </>
                            }
                        })
                    }
                </div>
            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    yew::Renderer::<App>::new().render();
}
