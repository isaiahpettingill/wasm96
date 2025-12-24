static mut CUBE_ROT: f32 = 0.0;

#[unsafe(no_mangle)]
pub extern "C" fn setup() {
    wasm96::graphics::set_size(320, 240);
    wasm96::graphics::set_3d(true);

    // Register font for HUD
    wasm96::graphics::font_register_spleen("spleen", 12); // Size 12 (6x12)

    // Define a cube with 24 vertices (4 per face) for flat shading
    // x, y, z, u, v, nx, ny, nz
    let vertices: &[f32] = &[
        // Front face (Z+)
        -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0,
        1.0, 1.0, 1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        // Back face (Z-)
        1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0, -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, -1.0, -1.0,
        1.0, -1.0, 1.0, 1.0, 0.0, 0.0, -1.0, 1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 0.0, -1.0,
        // Top face (Y+)
        -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0,
        -1.0, 1.0, 1.0, 0.0, 1.0, 0.0, -1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0,
        // Bottom face (Y-)
        -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, -1.0, -1.0, 1.0, 0.0, 0.0, -1.0, 0.0, 1.0,
        -1.0, 1.0, 1.0, 1.0, 0.0, -1.0, 0.0, -1.0, -1.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0,
        // Right face (X+)
        1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, -1.0, -1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0,
        -1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0,
        // Left face (X-)
        -1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, -1.0, 1.0, 1.0, 0.0, -1.0, 0.0, 0.0, -1.0,
        1.0, 1.0, 1.0, 1.0, -1.0, 0.0, 0.0, -1.0, 1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 0.0,
    ];

    let indices: &[u32] = &[
        0, 1, 2, 0, 2, 3, // Front
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, 8, 10, 11, // Top
        12, 13, 14, 12, 14, 15, // Bottom
        16, 17, 18, 16, 18, 19, // Right
        20, 21, 22, 20, 22, 23, // Left
    ];

    wasm96::graphics::mesh_create("cube", vertices, indices);
}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    // Clear screen (and depth buffer implicitly via our hack)
    wasm96::graphics::background(30, 30, 30);

    // Update rotation
    unsafe {
        CUBE_ROT += 0.02;
    }

    // Setup camera
    wasm96::graphics::camera_perspective(1.0, 320.0 / 240.0, 0.1, 100.0);
    wasm96::graphics::camera_look_at(
        (0.0, 2.0, 4.0), // Eye
        (0.0, 0.0, 0.0), // Target
        (0.0, 1.0, 0.0), // Up
    );

    // Draw cube
    let rot = unsafe { CUBE_ROT };
    wasm96::graphics::mesh_draw(
        "cube",
        (0.0, 0.0, 0.0),       // Pos
        (rot, rot * 0.5, 0.0), // Rot
        (1.0, 1.0, 1.0),       // Scale
    );

    // Draw another cube
    wasm96::graphics::mesh_draw("cube", (2.5, 0.0, -1.0), (0.0, rot, 0.0), (0.5, 0.5, 0.5));

    // Draw some 2D text on top
    wasm96::graphics::set_color(255, 255, 255, 255);
    wasm96::graphics::text_key(10, 10, "spleen", "3D Cube Demo");
}
