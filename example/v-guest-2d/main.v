module v_guest

import wasm96

__global (
	rect_x int
	rect_y int
	vel_x int
	vel_y int
)

@[export: 'setup']
fn setup() {
	wasm96.graphics_set_size(320, 240)
	rect_x = 10
	rect_y = 10
	vel_x = 2
	vel_y = 2
}

@[export: 'update']
fn update() {
	rect_x = rect_x + vel_x
	rect_y = rect_y + vel_y

	if rect_x <= 0 {
		vel_x = -vel_x
	}
	if rect_x >= 290 {
		vel_x = -vel_x
	}
	if rect_y <= 0 {
		vel_y = -vel_y
	}
	if rect_y >= 210 {
		vel_y = -vel_y
	}
}

@[export: 'draw']
fn draw() {
	// Clear screen
	wasm96.graphics_background(20, 20, 40)

	// Draw rectangle
	wasm96.graphics_set_color(255, 100, 100, 255)
	wasm96.graphics_rect(rect_x, rect_y, 30, 30)
}
