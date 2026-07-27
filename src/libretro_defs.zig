const std = @import("std");

pub const api_version = 1;
pub const environment_set_pixel_format = 10;
pub const environment_set_geometry = 37;
pub const memory_save_ram = 0;
pub const device_none = 0;
pub const device_joypad = 1;
pub const pixel_format_xrgb8888 = 1;

pub const joypad_b = 0;
pub const joypad_y = 1;
pub const joypad_select = 2;
pub const joypad_start = 3;
pub const joypad_up = 4;
pub const joypad_down = 5;
pub const joypad_left = 6;
pub const joypad_right = 7;
pub const joypad_a = 8;
pub const joypad_x = 9;
pub const joypad_l = 10;
pub const joypad_r = 11;

pub const EnvironmentFn = *const fn (cmd: c_uint, data: ?*anyopaque) callconv(.c) bool;
pub const VideoRefreshFn = *const fn (data: ?*const anyopaque, width: c_uint, height: c_uint, pitch: usize) callconv(.c) void;
pub const AudioSampleFn = *const fn (left: i16, right: i16) callconv(.c) usize;
pub const AudioBatchFn = *const fn (data: [*]const i16, frames: usize) callconv(.c) usize;
pub const InputPollFn = *const fn () callconv(.c) void;
pub const InputStateFn = *const fn (port: c_uint, device: c_uint, index: c_uint, id: c_uint) callconv(.c) i16;

pub const GameInfo = extern struct {
    path: ?[*:0]const u8,
    data: ?*const anyopaque,
    size: usize,
    meta: ?[*:0]const u8,
};

pub const SystemInfo = extern struct {
    library_name: [*:0]const u8,
    library_version: [*:0]const u8,
    valid_extensions: [*:0]const u8,
    need_fullpath: bool,
    block_extract: bool,
};

pub const GameGeometry = extern struct {
    base_width: c_uint,
    base_height: c_uint,
    max_width: c_uint,
    max_height: c_uint,
    aspect_ratio: f32,
};

pub const SystemTiming = extern struct {
    fps: f64,
    sample_rate: f64,
};

pub const SystemAvInfo = extern struct {
    geometry: GameGeometry,
    timing: SystemTiming,
};

pub fn levelFromPressed(pressed: i16) u8 {
    return if (pressed != 0) 3 else 0;
}

pub fn anyopaquePtr(slice: []const u8) ?*const anyopaque {
    if (slice.len == 0) return null;
    return @ptrCast(slice.ptr);
}

pub fn mutAnyopaquePtr(slice: []u8) ?*anyopaque {
    if (slice.len == 0) return null;
    return @ptrCast(slice.ptr);
}

pub fn writePackedController(dst: *[3]u8, levels: [12]u8) void {
    dst.* = .{ 0, 0, 0 };
    for (levels, 0..) |level, button| {
        const bit: u5 = @intCast(button * 2);
        const byte_index = bit / 8;
        const shift: u3 = @intCast(bit % 8);
        dst[byte_index] |= (level & 0x3) << shift;
    }
}

comptime {
    _ = std;
}
