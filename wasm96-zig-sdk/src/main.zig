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
    extern fn wasm96_graphics_image_png(x: i32, y: i32, ptr: [*]const u8, len: usize) void;
    extern fn wasm96_graphics_image_jpeg(x: i32, y: i32, ptr: [*]const u8, len: usize) void;
    extern fn wasm96_graphics_triangle(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) void;
    extern fn wasm96_graphics_triangle_outline(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) void;
    extern fn wasm96_graphics_bezier_quadratic(x1: i32, y1: i32, cx: i32, cy: i32, x2: i32, y2: i32, segments: u32) void;
    extern fn wasm96_graphics_bezier_cubic(x1: i32, y1: i32, cx1: i32, cy1: i32, cx2: i32, cy2: i32, x2: i32, y2: i32, segments: u32) void;
    extern fn wasm96_graphics_pill(x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_pill_outline(x: i32, y: i32, w: u32, h: u32) void;

    // 3D Graphics
    extern fn wasm96_graphics_set_3d(enable: u32) void;
    extern fn wasm96_graphics_camera_look_at(eye_x: f32, eye_y: f32, eye_z: f32, target_x: f32, target_y: f32, target_z: f32, up_x: f32, up_y: f32, up_z: f32) void;
    extern fn wasm96_graphics_camera_perspective(fovy: f32, aspect: f32, near: f32, far: f32) void;
    extern fn wasm96_graphics_mesh_create(key: u64, v_ptr: [*]const f32, v_len: usize, i_ptr: [*]const u32, i_len: usize) u32;
    extern fn wasm96_graphics_mesh_create_obj(key: u64, ptr: [*]const u8, len: usize) u32;
    extern fn wasm96_graphics_mesh_create_stl(key: u64, ptr: [*]const u8, len: usize) u32;
    extern fn wasm96_graphics_mesh_draw(key: u64, x: f32, y: f32, z: f32, rx: f32, ry: f32, rz: f32, sx: f32, sy: f32, sz: f32) void;
    extern fn wasm96_graphics_mesh_set_texture(mesh_key: u64, image_key: u64) u32;

    // Materials / textures (OBJ+MTL workflows)
    extern fn wasm96_graphics_mtl_register_texture(
        texture_key: u64,
        mtl_ptr: u32,
        mtl_len: u32,
        tex_filename_ptr: u32,
        tex_filename_len: u32,
        tex_ptr: u32,
        tex_len: u32,
    ) u32;

    extern fn wasm96_graphics_svg_register(key: u64, data_ptr: [*]const u8, data_len: usize) u32;
    extern fn wasm96_graphics_svg_draw_key(key: u64, x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_svg_unregister(key: u64) void;

    extern fn wasm96_graphics_gif_register(key: u64, data_ptr: [*]const u8, data_len: usize) u32;
    extern fn wasm96_graphics_gif_draw_key(key: u64, x: i32, y: i32) void;
    extern fn wasm96_graphics_gif_draw_key_scaled(key: u64, x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_gif_unregister(key: u64) void;

    extern fn wasm96_graphics_png_register(key: u64, data_ptr: [*]const u8, data_len: usize) u32;
    extern fn wasm96_graphics_png_draw_key(key: u64, x: i32, y: i32) void;
    extern fn wasm96_graphics_png_draw_key_scaled(key: u64, x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_png_unregister(key: u64) void;

    extern fn wasm96_graphics_jpeg_register(key: u64, data_ptr: [*]const u8, data_len: usize) u32;
    extern fn wasm96_graphics_jpeg_draw_key(key: u64, x: i32, y: i32) void;
    extern fn wasm96_graphics_jpeg_draw_key_scaled(key: u64, x: i32, y: i32, w: u32, h: u32) void;
    extern fn wasm96_graphics_jpeg_unregister(key: u64) void;

    extern fn wasm96_graphics_font_register_ttf(key: u64, data_ptr: [*]const u8, data_len: usize) u32;
    extern fn wasm96_graphics_font_register_bdf(key: u64, data_ptr: [*]const u8, data_len: usize) u32;
    extern fn wasm96_graphics_font_register_spleen(key: u64, size: u32) u32;
    extern fn wasm96_graphics_font_unregister(key: u64) void;
    extern fn wasm96_graphics_text_key(x: i32, y: i32, font_key: u64, text_ptr: [*]const u8, text_len: usize) void;
    extern fn wasm96_graphics_text_measure_key(font_key: u64, text_ptr: [*]const u8, text_len: usize) u64;

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
    extern fn wasm96_storage_save(key: u64, data_ptr: [*]const u8, data_len: usize) void;
    extern fn wasm96_storage_load(key: u64) u64;
    extern fn wasm96_storage_free(ptr: [*]const u8, len: usize) void;

    extern fn wasm96_system_log(ptr: [*]const u8, len: usize) void;
    extern fn wasm96_system_millis() u64;
};

/// Graphics API.
pub const graphics = struct {
    fn hashKey(key: []const u8) u64 {
        var hash: u64 = 0xcbf29ce484222325;
        for (key) |byte| {
            hash ^= byte;
            hash *%= 0x100000001b3;
        }
        return hash;
    }

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

    /// Draw an image from raw PNG bytes.
    pub fn imagePng(x: i32, y: i32, data: []const u8) void {
        sys.wasm96_graphics_image_png(x, y, data.ptr, data.len);
    }

    pub fn imageJpeg(x: i32, y: i32, data: []const u8) void {
        sys.wasm96_graphics_image_jpeg(x, y, data.ptr, data.len);
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

    // =========================
    // 3D Graphics
    // =========================

    /// Enable or disable 3D rendering mode.
    pub fn set3d(enable: bool) void {
        sys.wasm96_graphics_set_3d(@intFromBool(enable));
    }

    /// Set the camera view matrix using look-at parameters.
    pub fn cameraLookAt(eye_x: f32, eye_y: f32, eye_z: f32, target_x: f32, target_y: f32, target_z: f32, up_x: f32, up_y: f32, up_z: f32) void {
        sys.wasm96_graphics_camera_look_at(eye_x, eye_y, eye_z, target_x, target_y, target_z, up_x, up_y, up_z);
    }

    /// Set the camera projection matrix.
    pub fn cameraPerspective(fovy: f32, aspect: f32, near: f32, far: f32) void {
        sys.wasm96_graphics_camera_perspective(fovy, aspect, near, far);
    }

    /// Create a mesh from raw vertex data.
    /// Vertices are [x, y, z, u, v, nx, ny, nz] (8 floats).
    /// Returns true on success.
    pub fn meshCreate(key: []const u8, vertices: []const f32, indices: []const u32) bool {
        return sys.wasm96_graphics_mesh_create(hashKey(key), vertices.ptr, vertices.len, indices.ptr, indices.len) != 0;
    }

    /// Create a mesh from OBJ source text.
    /// Returns true on success.
    pub fn meshCreateObj(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_mesh_create_obj(hashKey(key), data.ptr, data.len) != 0;
    }

    /// Create a mesh from STL binary data.
    /// Returns true on success.
    pub fn meshCreateStl(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_mesh_create_stl(hashKey(key), data.ptr, data.len) != 0;
    }

    /// Draw a mesh instance.
    /// Rotation is Euler angles in radians (x, y, z).
    pub fn meshDraw(key: []const u8, x: f32, y: f32, z: f32, rx: f32, ry: f32, rz: f32, sx: f32, sy: f32, sz: f32) void {
        sys.wasm96_graphics_mesh_draw(hashKey(key), x, y, z, rx, ry, rz, sx, sy, sz);
    }

    /// Bind a keyed decoded image (PNG/JPEG) as the texture for a mesh.
    /// Returns true on success.
    ///
    /// Notes:
    /// - PNG alpha is respected (RGBA).
    /// - JPEG is treated as opaque (RGB), but may still be uploaded as RGBA with A=255 on host.
    pub fn meshSetTexture(meshKey: []const u8, imageKey: []const u8) bool {
        return sys.wasm96_graphics_mesh_set_texture(hashKey(meshKey), hashKey(imageKey)) != 0;
    }

    /// Register an SVG resource under a string key.
    /// Register an encoded texture referenced by an `.mtl` file (`map_Kd`) under `texture_key`.
    ///
    /// Returns `true` if it registered (filename matched + decode succeeded), else `false`.
    pub fn mtlRegisterTexture(textureKey: []const u8, mtlBytes: []const u8, texFilename: []const u8, texBytes: []const u8) bool {
        return sys.wasm96_graphics_mtl_register_texture(
            hashKey(textureKey),
            @as(u32, @intCast(@intFromPtr(mtlBytes.ptr))),
            @as(u32, @intCast(mtlBytes.len)),
            @as(u32, @intCast(@intFromPtr(texFilename.ptr))),
            @as(u32, @intCast(texFilename.len)),
            @as(u32, @intCast(@intFromPtr(texBytes.ptr))),
            @as(u32, @intCast(texBytes.len)),
        ) != 0;
    }

    pub fn svgRegister(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_svg_register(hashKey(key), data.ptr, data.len) != 0;
    }

    /// Draw a registered SVG by key.
    pub fn svgDrawKey(key: []const u8, x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_svg_draw_key(hashKey(key), x, y, w, h);
    }

    /// Unregister an SVG by key.
    pub fn svgUnregister(key: []const u8) void {
        sys.wasm96_graphics_svg_unregister(hashKey(key));
    }

    /// Register a GIF resource under a string key.
    pub fn gifRegister(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_gif_register(hashKey(key), data.ptr, data.len) != 0;
    }

    /// Draw a registered GIF by key at natural size.
    pub fn gifDrawKey(key: []const u8, x: i32, y: i32) void {
        sys.wasm96_graphics_gif_draw_key(hashKey(key), x, y);
    }

    /// Draw a registered GIF by key scaled.
    pub fn gifDrawKeyScaled(key: []const u8, x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_gif_draw_key_scaled(hashKey(key), x, y, w, h);
    }

    /// Unregister a GIF by key.
    pub fn gifUnregister(key: []const u8) void {
        sys.wasm96_graphics_gif_unregister(hashKey(key));
    }

    /// Register a PNG resource under a string key.
    pub fn pngRegister(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_png_register(hashKey(key), data.ptr, data.len) != 0;
    }

    pub fn jpegRegister(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_jpeg_register(hashKey(key), data.ptr, data.len) != 0;
    }

    /// Draw a registered PNG by key at natural size.
    pub fn pngDrawKey(key: []const u8, x: i32, y: i32) void {
        sys.wasm96_graphics_png_draw_key(hashKey(key), x, y);
    }

    pub fn jpegDrawKey(key: []const u8, x: i32, y: i32) void {
        sys.wasm96_graphics_jpeg_draw_key(hashKey(key), x, y);
    }

    /// Draw a registered PNG by key scaled.
    pub fn pngDrawKeyScaled(key: []const u8, x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_png_draw_key_scaled(hashKey(key), x, y, w, h);
    }

    pub fn jpegDrawKeyScaled(key: []const u8, x: i32, y: i32, w: u32, h: u32) void {
        sys.wasm96_graphics_jpeg_draw_key_scaled(hashKey(key), x, y, w, h);
    }

    /// Unregister a PNG by key.
    pub fn pngUnregister(key: []const u8) void {
        sys.wasm96_graphics_png_unregister(hashKey(key));
    }

    pub fn jpegUnregister(key: []const u8) void {
        sys.wasm96_graphics_jpeg_unregister(hashKey(key));
    }

    /// Register a TTF font under a string key.
    pub fn fontRegisterTtf(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_font_register_ttf(hashKey(key), data.ptr, data.len) != 0;
    }

    /// Register a BDF font under a string key.
    pub fn fontRegisterBdf(key: []const u8, data: []const u8) bool {
        return sys.wasm96_graphics_font_register_bdf(hashKey(key), data.ptr, data.len) != 0;
    }

    /// Register a built-in Spleen font under a string key.
    pub fn fontRegisterSpleen(key: []const u8, size: u32) bool {
        return sys.wasm96_graphics_font_register_spleen(hashKey(key), size) != 0;
    }

    /// Unregister a font by key.
    pub fn fontUnregister(key: []const u8) void {
        sys.wasm96_graphics_font_unregister(hashKey(key));
    }

    /// Draw text using a font referenced by key.
    pub fn textKey(x: i32, y: i32, font_key: []const u8, string: []const u8) void {
        sys.wasm96_graphics_text_key(x, y, hashKey(font_key), string.ptr, string.len);
    }

    /// Measure text using a font referenced by key.
    pub fn textMeasureKey(font_key: []const u8, str: []const u8) TextSize {
        const result = sys.wasm96_graphics_text_measure_key(hashKey(font_key), str.ptr, str.len);
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

/// Storage API.
pub const storage = struct {
    /// Save data to persistent storage.
    pub fn save(key: []const u8, data: []const u8) void {
        sys.wasm96_storage_save(graphics.hashKey(key), data.ptr, data.len);
    }

    /// Load data from persistent storage.
    /// Returns the data if found, null otherwise.
    pub fn load(allocator: std.mem.Allocator, key: []const u8) !?[]u8 {
        const packed_result = sys.wasm96_storage_load(graphics.hashKey(key));
        if (packed_result == 0) return null;

        const ptr_int = packed_result >> 32;
        const len = packed_result & 0xFFFFFFFF;

        const ptr = @as([*]const u8, @ptrFromInt(ptr_int));

        // Copy data from guest memory
        const data = try allocator.alloc(u8, @as(usize, @intCast(len)));
        @memcpy(data, ptr[0..@as(usize, @intCast(len))]);

        // Free the memory in guest space
        sys.wasm96_storage_free(ptr, @as(usize, @intCast(len)));

        return data;
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
