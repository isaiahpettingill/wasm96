// Shapes demo guest: ellipse, arc, quad.

use wasm96_sdk::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn setup() {
    graphics::set_size(320, 240);
}

#[unsafe(no_mangle)]
pub extern "C" fn update() {}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    graphics::background(12, 16, 24);

    // Ellipse
    graphics::set_color(255, 120, 80, 255);
    graphics::ellipse(80, 80, 60, 40);

    // Arc (quarter circle)
    graphics::set_color(120, 200, 255, 255);
    graphics::arc(200, 80, 80, 80, 0.0, core::f32::consts::FRAC_PI_2);

    // Quad (filled)
    graphics::set_color(180, 255, 120, 255);
    graphics::quad(60, 150, 120, 130, 160, 190, 90, 210);
}
