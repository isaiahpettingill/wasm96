const wasm96 = @import("wasm96");

var cube_rot: f32 = 0.0;

export fn setup() void {
    wasm96.graphics.setSize(320, 240);
    wasm96.graphics.set3d(true);

    // Register font for HUD
    _ = wasm96.graphics.fontRegisterSpleen("spleen", 12);

    // Define a cube with 24 vertices (4 per face) for flat shading
    // x, y, z, u, v, nx, ny, nz
    const vertices = [_]f32{
        // Front face (Z+)
        -1.0, -1.0, 1.0,  0.0, 0.0, 0.0,  0.0,  1.0,
        1.0,  -1.0, 1.0,  1.0, 0.0, 0.0,  0.0,  1.0,
        1.0,  1.0,  1.0,  1.0, 1.0, 0.0,  0.0,  1.0,
        -1.0, 1.0,  1.0,  0.0, 1.0, 0.0,  0.0,  1.0,
        // Back face (Z-)
        1.0,  -1.0, -1.0, 0.0, 0.0, 0.0,  0.0,  -1.0,
        -1.0, -1.0, -1.0, 1.0, 0.0, 0.0,  0.0,  -1.0,
        -1.0, 1.0,  -1.0, 1.0, 1.0, 0.0,  0.0,  -1.0,
        1.0,  1.0,  -1.0, 0.0, 1.0, 0.0,  0.0,  -1.0,
        // Top face (Y+)
        -1.0, 1.0,  1.0,  0.0, 0.0, 0.0,  1.0,  0.0,
        1.0,  1.0,  1.0,  1.0, 0.0, 0.0,  1.0,  0.0,
        1.0,  1.0,  -1.0, 1.0, 1.0, 0.0,  1.0,  0.0,
        -1.0, 1.0,  -1.0, 0.0, 1.0, 0.0,  1.0,  0.0,
        // Bottom face (Y-)
        -1.0, -1.0, -1.0, 0.0, 0.0, 0.0,  -1.0, 0.0,
        1.0,  -1.0, -1.0, 1.0, 0.0, 0.0,  -1.0, 0.0,
        1.0,  -1.0, 1.0,  1.0, 1.0, 0.0,  -1.0, 0.0,
        -1.0, -1.0, 1.0,  0.0, 1.0, 0.0,  -1.0, 0.0,
        // Right face (X+)
        1.0,  -1.0, 1.0,  0.0, 0.0, 1.0,  0.0,  0.0,
        1.0,  1.0,  1.0,  1.0, 0.0, 1.0,  0.0,  0.0,
        1.0,  1.0,  -1.0, 1.0, 1.0, 1.0,  0.0,  0.0,
        1.0,  -1.0, -1.0, 0.0, 1.0, 1.0,  0.0,  0.0,
        // Left face (X-)
        -1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0,  0.0,
        -1.0, 1.0,  -1.0, 1.0, 0.0, -1.0, 0.0,  0.0,
        -1.0, 1.0,  1.0,  1.0, 1.0, -1.0, 0.0,  0.0,
        -1.0, -1.0, 1.0,  0.0, 1.0, -1.0, 0.0,  0.0,
    };

    const indices = [_]u32{
        0, 1, 2, 0, 2, 3, // Front
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, 8, 10, 11, // Top
        12, 13, 14, 12, 14, 15, // Bottom
        16, 17, 18, 16, 18, 19, // Right
        20, 21, 22, 20, 22, 23, // Left
    };

    _ = wasm96.graphics.meshCreate("cube", &vertices, &indices);
}

export fn draw() void {
    // Clear screen (and depth buffer implicitly via our hack)
    wasm96.graphics.background(30, 30, 30);

    // Update rotation
    cube_rot += 0.02;

    // Setup camera
    wasm96.graphics.cameraPerspective(1.0, 320.0 / 240.0, 0.1, 100.0);
    wasm96.graphics.cameraLookAt(0.0, 2.0, 4.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0);

    // Draw cube
    wasm96.graphics.meshDraw("cube", 0.0, 0.0, 0.0, cube_rot, cube_rot * 0.5, 0.0, 1.0, 1.0, 1.0);

    // Draw another cube
    wasm96.graphics.meshDraw("cube", 2.5, 0.0, -1.0, 0.0, cube_rot, 0.0, 0.5, 0.5, 0.5);

    // Draw some 2D text on top
    wasm96.graphics.setColor(255, 255, 255, 255);
    wasm96.graphics.textKey(10, 10, "spleen", "3D Cube Demo");
}
