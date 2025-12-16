// wasm96/example/go-guest/main.go
package main

import (
	"wasm96-go-sdk"
)

//go:export setup
func setup() {
	wasm96.Graphics.SetSize(640, 480)
	_ = wasm96.Graphics.FontUseSpleen(16) // Load spleen font size 16
}

//go:export update
func update() {
	// Update logic here
}

//go:export draw
func draw() {
	wasm96.Graphics.Background(0, 0, 0)               // Black background
	wasm96.Graphics.SetColor(255, 255, 255, 255)      // White
	wasm96.Graphics.Rect(100, 100, 100, 100)          // Draw a white rectangle
	wasm96.Graphics.Text(10, 10, 0, "Hello from Go!") // Draw text
}

func main() {
	// Go requires a main function, but it won't be called in WASM
}
