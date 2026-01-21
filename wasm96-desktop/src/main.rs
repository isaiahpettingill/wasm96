mod app;
mod platform;

use anyhow::Result;
use app::Wasm96App;
use platform::DesktopPlatform;
use platform::audio::init_audio;

fn main() -> Result<()> {
    // Note: Command line arguments (initial game path) are parsed and handled
    // inside Wasm96App::new to keep the entry point clean.

    // Initialize Audio system
    let (prod, _audio_stream) = init_audio()?;

    // Initialize Platform state (Input, Graphics, etc.)
    let platform = DesktopPlatform::new(prod);

    // Application window options
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([320.0, 240.0])
            .with_title("wasm96"),
        // Use WGPU for Vulkan/modern 3D support
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "wasm96-desktop",
        options,
        Box::new(|cc| {
            // Create the app state
            Ok(Box::new(Wasm96App::new(cc, platform)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
