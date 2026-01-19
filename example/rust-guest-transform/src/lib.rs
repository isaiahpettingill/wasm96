// Transformation demo guest: translate, rotate, scale, shear, and matrix stack.

use wasm96_sdk::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn setup() {
    graphics::set_size(320, 240);
}

#[unsafe(no_mangle)]
pub extern "C" fn update() {}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    graphics::background(20, 20, 30);

    let time = system::millis() as f32 / 1000.0;

    // 1. Rotating rectangle in the center
    graphics::push_matrix();
    graphics::translate(160.0, 120.0, 0.0);
    graphics::rotate(time);

    graphics::set_color(255, 100, 100, 255);
    graphics::rect(-30, -30, 60, 60);

    // Nested orbiting square
    graphics::push_matrix();
    graphics::translate(80.0, 0.0, 0.0);
    graphics::rotate(time * 2.0);
    graphics::scale(0.5, 0.5, 1.0);
    graphics::set_color(100, 255, 100, 255);
    graphics::rect(-20, -20, 40, 40);
    graphics::pop_matrix();

    graphics::pop_matrix();

    // 2. Shearing effect
    graphics::push_matrix();
    graphics::translate(20.0, 20.0, 0.0);
    graphics::shear_x(0.3 * (time * 2.0).sin());
    graphics::set_color(100, 100, 255, 255);
    graphics::rect(0, 0, 100, 30);
    graphics::pop_matrix();

    // 3. Scaling effect
    graphics::push_matrix();
    graphics::translate(250.0, 50.0, 0.0);
    let s = 1.0 + 0.5 * (time * 3.0).sin();
    graphics::scale(s, s, 1.0);
    graphics::set_color(255, 255, 100, 255);
    graphics::circle(0, 0, 20);
    graphics::pop_matrix();
}
