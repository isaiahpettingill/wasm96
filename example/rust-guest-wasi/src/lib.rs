use std::fs;
use std::io::{Read, Write};
use wasm96_sdk::*;

#[no_mangle]
pub extern "C" fn setup() {
    graphics_set_size(320, 240);

    // Demonstrate WASI file I/O
    let test_file = "hello.txt";
    let message = "Hello from WASI on WASM96!";

    // Write to a file
    if let Ok(mut file) = fs::File::create(test_file) {
        let _ = file.write_all(message.as_bytes());
    }

    // Read it back
    let mut content = String::new();
    if let Ok(mut file) = fs::File::open(test_file) {
        let _ = file.read_to_string(&mut content);
    }

    // Print to stdout (inherited by host)
    println!("WASI test: read '{}' from file", content);
}

#[no_mangle]
pub extern "C" fn update() {}

#[no_mangle]
pub extern "C" fn draw() {
    graphics_clear();

    graphics_set_color(255, 255, 255, 255);
    graphics_text(10, 20, 0, "WASI Filesystem Demo");

    graphics_set_color(200, 200, 200, 255);
    graphics_text(10, 50, 0, "Checking hello.txt...");

    // Display file content on screen
    if let Ok(content) = fs::read_to_string("hello.txt") {
        graphics_set_color(0, 255, 0, 255);
        graphics_text(10, 70, 0, &format!("Content: {}", content));
    } else {
        graphics_set_color(255, 0, 0, 255);
        graphics_text(10, 70, 0, "Error reading file!");
    }

    graphics_set_color(255, 255, 255, 255);
    graphics_text(10, 110, 0, "This file persists across reloads");
    graphics_text(10, 130, 0, "in Libretro SRAM or Desktop .img");
}
