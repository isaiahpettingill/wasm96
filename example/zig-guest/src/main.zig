const wasm96 = @import("wasm96");

export fn setup() void {
    wasm96.graphics.setSize(640, 480);
    _ = wasm96.graphics.fontUseSpleen(16); // Load spleen font size 16, returns font id 0
}

export fn update() void {
    // Update logic here
}

export fn draw() void {
    wasm96.graphics.background(0, 0, 0); // Black background
    wasm96.graphics.setColor(255, 255, 255, 255); // White
    wasm96.graphics.rect(100, 100, 100, 100); // Draw a white rectangle
    wasm96.graphics.text(10, 10, 0, "Hello from Zig!"); // Draw text (assuming font 0 is default)
}
