const std = @import("std");

/// Joypad button ids.
pub const Button = enum(u32) {
    b = 0,
    y = 1,
    select = 2,
    start = 3,
    up = 4,
    down = 5,
    left = 6,
    right = 7,
    a = 8,
    x = 9,
    l1 = 10,
    r1 = 11,
    l2 = 12,
    r2 = 13,
    l3 = 14,
    r3 = 15,
};

/// Text size dimensions.
pub const TextSize = struct {
    width: u32,
    height: u32,
};

/// Low-level raw ABI imports.
pub const sys = struct {
    // Graphics
    extern fn wasm96_graphics_set_size(width: u32, height: u32) void;
    extern fn wasm96_graphics_set_color(r: u32, g: u32, b: u32, a: u32) void;
    extern fn wasm96_graphics_background(r: u32, g: u32, b: u32) void;
    extern fn wasm96_graphics_point(x: i32, y: i32) void;
    extern fn wasm96_graphics_line(x1: i32, y1: i32, x2: i32, y2: i32) void;
    extern fn wasm96_graphics_rect(x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_rect_outline(x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_circle(x: i32, y: i32, r: u32) void;
    extern fn wasm96_graphics_circle_outline(x: i32, y: i32, r: u32) void;
    extern fn wasm96_graphics_image(x: i32, y: i32, w: u32, h: u32, ptr: [*]const u8, len: usize) void;
    extern fn wasm96_graphics_triangle(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) void;
    extern fn wasm96_graphics_triangle_outline(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) void;
    extern fn wasm96_graphics_bezier_quadratic(x1: i32, y1: i32, cx: i32, cy: i32, x2: i32, y2: i32, segments: u32) void;
    extern fn wasm96_graphics_bezier_cubic(x1: i32, y1: i32, cx1: i32, cy1: i32, cx2: i32, cy2: i32, x2: i32, y2: i32, segments: u32) void;
    extern fn wasm96_graphics_pill(x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_pill_outline(x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_svg_create(ptr: [*]const u8, len: usize) u32;
    extern fn wasm96_graphics_svg_draw(id: u32, x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_svg_destroy(id: u32) void;
    extern fn wasm96_graphics_gif_create(ptr: [*]const u8, len: usize) u32;
    extern fn wasm96_graphics_gif_draw(id: u32, x: i32, y: i32) void;
    extern fn wasm96_graphics_gif_draw_scaled(id: u32, x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_gif_destroy(id: u32) void;
    extern fn wasm96_graphics_font_upload_ttf(ptr: [*]const u8, len: usize) u32;
    extern fn wasm96_graphics_font_use_spleen(size: u32) u32;
    extern fn wasm96_graphics_text(x: i32, y: i32, font: u32, ptr: [*]const u8, len: usize) void;
    extern fn wasm96_graphics_text_measure(font: u32, ptr: [*]const u8, len: usize) u64;

    // Input
    extern fn wasm96_input_is_button_down(port: u32, btn: u32) u32;
    extern fn wasm96_input_is_key_down(key: u32) u32;
    extern fn wasm96_input_get_mouse_x() i32;
    extern fn wasm96_input_get_mouse_y() i32;
    extern fn wasm96_input_is_mouse_down(btn: u32) u32;

    // Audio
    extern fn wasm96_audio_init(sample_rate: u32) u32;
    extern fn wasm96_audio_push_samples(ptr: [*]const i16, len: usize) void;
    extern fn wasm96_audio_play_wav(ptr: [*]const u8, len: usize) void;
    extern fn wasm96_audio_play_qoa(ptr: [*]const u8, len: usize) void;
    extern fn wasm96_audio_play_xm(ptr: [*]const u8, len: usize) void;

    // System
    extern fn wasm96_system_log(ptr: [*]const u8, len: usize) void;
    extern fn wasm96_system_millis() u64;
};

/// Graphics API.
pub const graphics = struct {
    /// Set the screen dimensions.
    pub fn setSize(width: u32, height: u32) void {
        sys.wasm96_graphics_set_size(width, height);
    }

    /// Set the current drawing color (RGBA).
    pub fn setColor(r: u8, g: u8, b: u8, a: u8) void {
        sys.wasm96_graphics_set_color(@as(u32, r), @as(u32, g), @as(u32, b), @as(u32, a));
    }

    /// Clear the screen with a specific color (RGB).
    pub fn background(r: u8, g: u8, b: u8) void {
        sys.wasm96_graphics_background(@as(u32, r), @as(u32, g), @as(u32, b));
    }

    /// Draw a single pixel at (x, y).
    pub fn point(x: i32, y: i32) void {
        sys.wasm96_graphics_point(x, y);
    }

    /// Draw a line from (x1, y1) to (x2, y2).
    pub fn line(x1: i32, y1: i32, x2: i32, y2: i32) void {
        sys.wasm96_graphics_line(x1, y1, x2, y2);
    }

    /// Draw a filled rectangle.
    pub fn rect(x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_rect(x, y, w, h);
    }

    /// Draw a rectangle outline.
    pub fn rectOutline(x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_rect_outline(x, y, w, h);
    }

    /// Draw a filled circle.
    pub fn circle(x: i32, y: i32, r: u32) void {
        sys.wasm96_graphics_circle(x, y, r);
    }

    /// Draw a circle outline.
    pub fn circleOutline(x: i32, y: i32, r: u32) void {
        sys.wasm96_graphics_circle_outline(x, y, r);
    }

    /// Draw an image/sprite.
    /// `data` is a slice of RGBA bytes (4 bytes per pixel).
    pub fn image(x: i32, y: i32, w: u32, h: u32, data: []const u8) void {
        sys.wasm96_graphics_image(x, y, w, h, data.ptr, data.len);
    }

    /// Draw a filled triangle.
    pub fn triangle(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) void {
        sys.wasm96_graphics_triangle(x1, y1, x2, y2, x3, y3);
    }

    /// Draw a triangle outline.
    pub fn triangleOutline(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) void {
        sys.wasm96_graphics_triangle_outline(x1, y1, x2, y2, x3, y3);
    }

    /// Draw a quadratic Bezier curve.
    pub fn bezierQuadratic(x1: i32, y1: i32, cx: i32, cy: i32, x2: i32, y2: i32, segments: u32) void {
        sys.wasm96_graphics_bezier_quadratic(x1, y1, cx, cy, x2, y2, segments);
    }

    /// Draw a cubic Bezier curve.
    pub fn bezierCubic(x1: i32, y1: i32, cx1: i32, cy1: i32, cx2: i32, cy2: i32, x2: i32, y2: i32, segments: u32) void {
        sys.wasm96_graphics_bezier_cubic(x1, y1, cx1, cy1, cx2, cy2, x2, y2, segments);
    }

    /// Draw a filled pill.
    pub fn pill(x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_pill(x, y, w, h);
    }

    /// Draw a pill outline.
    pub fn pillOutline(x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_pill_outline(x, y, w, h);
    }

    /// Create an SVG resource.
    pub fn svgCreate(data: []const u8) u32 {
        return sys.wasm96_graphics_svg_create(data.ptr, data.len);
    }

    /// Draw an SVG resource.
    pub fn svgDraw(id: u32, x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_svg_draw(id, x, y, w, h);
    }

    /// Destroy an SVG resource.
    pub fn svgDestroy(id: u32) void {
        sys.wasm96_graphics_svg_destroy(id);
    }

    /// Create a GIF resource.
    pub fn gifCreate(data: []const u8) u32 {
        return sys.wasm96_graphics_gif_create(data.ptr, data.len);
    }

    /// Draw a GIF resource at natural size.
    pub fn gifDraw(id: u32, x: i32, y: i32) void {
        sys.wasm96_graphics_gif_draw(id, x, y);
    }

    /// Draw a GIF resource scaled.
    pub fn gifDrawScaled(id: u32, x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_gif_draw_scaled(id, x, y, w, h);
    }

    /// Destroy a GIF resource.
    pub fn gifDestroy(id: u32) void {
        sys.wasm96_graphics_gif_destroy(id);
    }

    /// Upload a TTF font.
    pub fn fontUploadTtf(data: []const u8) u32 {
        return sys.wasm96_graphics_font_upload_ttf(data.ptr, data.len);
    }

    /// Use a built-in Spleen font.
    pub fn fontUseSpleen(size: u32) u32 {
        return sys.wasm96_graphics_font_use_spleen(size);
    }

    /// Draw text.
    pub fn text(x: i32, y: i32, font: u32, string: []const u8) void {
        sys.wasm96_graphics_text(x, y, font, string.ptr, string.len);
    }

    /// Measure text.
    pub fn textMeasure(font: u32, str: []const u8) TextSize {
        const result = sys.wasm96_graphics_text_measure(font, str.ptr, str.len);
        return TextSize{
            .width = @as(u32, @intCast(result >> 32)),
            .height = @as(u32, @intCast(result & 0xFFFFFFFF)),
        };
    }
};

/// Input API.
pub const input = struct {
    /// Returns true if the specified button is currently held down.
    pub fn isButtonDown(port: u32, btn: Button) bool {
        return sys.wasm96_input_is_button_down(port, @intFromEnum(btn)) != 0;
    }

    /// Returns true if the specified key is currently held down.
    pub fn isKeyDown(key: u32) bool {
        return sys.wasm96_input_is_key_down(key) != 0;
    }

    /// Get current mouse X position.
    pub fn getMouseX() i32 {
        return sys.wasm96_input_get_mouse_x();
    }

    /// Get current mouse Y position.
    pub fn getMouseY() i32 {
        return sys.wasm96_input_get_mouse_y();
    }

    /// Returns true if the specified mouse button is held down.
    /// 0 = Left, 1 = Right, 2 = Middle.
    pub fn isMouseDown(btn: u32) bool {
        return sys.wasm96_input_is_mouse_down(btn) != 0;
    }
};

/// Audio API.
pub const audio = struct {
    /// Initialize audio system.
    pub fn init(sample_rate: u32) u32 {
        return sys.wasm96_audio_init(sample_rate);
    }

    /// Push a chunk of audio samples.
    /// Samples are interleaved stereo (L, R, L, R...) signed 16-bit integers.
    pub fn pushSamples(samples: []const i16) void {
        sys.wasm96_audio_push_samples(samples.ptr, samples.len);
    }

    /// Play a WAV file.
    /// The WAV data is decoded and played as a one-shot audio channel.
    pub fn playWav(data: []const u8) void {
        sys.wasm96_audio_play_wav(data.ptr, data.len);
    }

    /// Play a QOA file.
    /// The QOA data is decoded and played as a looping audio channel.
    pub fn playQoa(data: []const u8) void {
        sys.wasm96_audio_play_qoa(data.ptr, data.len);
    }

    /// Play an XM file.
    /// The XM data is decoded using xmrsplayer and played as a looping audio channel.
    pub fn playXm(data: []const u8) void {
        sys.wasm96_audio_play_xm(data.ptr, data.len);
    }
};

/// System API.
pub const system = struct {
    /// Log a message to the host console.
    pub fn log(message: []const u8) void {
        sys.wasm96_system_log(message.ptr, message.len);
    }

    /// Get the number of milliseconds since the app started.
    pub fn millis() u64 {
        return sys.wasm96_system_millis();
    }
};
