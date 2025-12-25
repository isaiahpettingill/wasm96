use libretro_backend::{AudioVideoInfo, Core, CoreInfo, LoadGameResult, RuntimeHandle};

#[derive(Default)]
struct DummyCore;

impl Core for DummyCore {
    fn info() -> CoreInfo {
        CoreInfo::new("Dummy", "0.0.1")
    }

    fn on_load_game(&mut self, _game_data: libretro_backend::GameData) -> LoadGameResult {
        LoadGameResult::Success(AudioVideoInfo::new())
    }

    fn on_unload_game(&mut self) -> libretro_backend::GameData {
        // Just a dummy implementation
        unsafe { std::mem::zeroed() }
    }

    fn on_run(&mut self, _handle: &mut RuntimeHandle) {
        let _ = std::mem::size_of::<libretro_sys::retro_hw_render_callback>();
    }

    fn on_reset(&mut self) {}
}
