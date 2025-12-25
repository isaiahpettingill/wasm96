module main

import wasm96


__global (
	cube_rot f32
)

@[export: 'setup']
fn setup() {
	wasm96.graphics_set_size(320, 240)
	wasm96.graphics_set_3d(true)
	wasm96.graphics_font_register_spleen('spleen'.bytes(), 12)

	// Define a cube with 24 vertices (4 per face) for flat shading
	// x, y, z, u, v, nx, ny, nz
	vertices := [
		// Front face (Z+)
		f32(-1.0), -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
		1.0, -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0,
		1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0,
		-1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0,
		// Back face (Z-)
		1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
		-1.0, -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, -1.0,
		-1.0, 1.0, -1.0, 1.0, 1.0, 0.0, 0.0, -1.0,
		1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 0.0, -1.0,
		// Top face (Y+)
		-1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
		1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
		1.0, 1.0, -1.0, 1.0, 1.0, 0.0, 1.0, 0.0,
		-1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0,
		// Bottom face (Y-)
		-1.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
		1.0, -1.0, -1.0, 1.0, 0.0, 0.0, -1.0, 0.0,
		1.0, -1.0, 1.0, 1.0, 1.0, 0.0, -1.0, 0.0,
		-1.0, -1.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0,
		// Right face (X+)
		1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
		1.0, -1.0, -1.0, 1.0, 0.0, 1.0, 0.0, 0.0,
		1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 0.0, 0.0,
		1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0,
		// Left face (X-)
		-1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
		-1.0, -1.0, 1.0, 1.0, 0.0, -1.0, 0.0, 0.0,
		-1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 0.0, 0.0,
		-1.0, 1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 0.0
	]

	indices := [
		u32(0), 1, 2, 0, 2, 3, // Front
		4, 5, 6, 4, 6, 7, // Back
		8, 9, 10, 8, 10, 11, // Top
		12, 13, 14, 12, 14, 15, // Bottom
		16, 17, 18, 16, 18, 19, // Right
		20, 21, 22, 20, 22, 23  // Left
	]

	wasm96.graphics_mesh_create('cube'.bytes(), vertices, indices)
}

@[export: 'draw']
fn draw() {
	// Clear screen
	wasm96.graphics_background(30, 30, 30)

	// Update rotation
	cube_rot += 0.02

	// Setup camera
	wasm96.graphics_camera_perspective(1.0, 320.0 / 240.0, 0.1, 100.0)
	wasm96.graphics_camera_look_at(
		0.0, 2.0, 4.0, // Eye
		0.0, 0.0, 0.0, // Target
		0.0, 1.0, 0.0  // Up
	)

	// Draw cube
	rot := cube_rot
	wasm96.graphics_mesh_draw(
		'cube'.bytes(),
		0.0, 0.0, 0.0,       // Pos
		rot, rot * 0.5, 0.0, // Rot
		1.0, 1.0, 1.0        // Scale
	)

	// Draw another cube
	wasm96.graphics_mesh_draw(
		'cube'.bytes(),
		2.5, 0.0, -1.0,
		0.0, rot, 0.0,
		0.5, 0.5, 0.5
	)

	// Draw text
	wasm96.graphics_set_color(255, 255, 255, 255)
	wasm96.graphics_text_key(10, 10, 'spleen'.bytes(), '3D Cube Demo (V)'.bytes())
}
