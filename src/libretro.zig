const std = @import("std");
const Core = @import("core.zig").Core;
const limits = @import("limits.zig");
const retro = @import("libretro_defs.zig");

const allocator = std.heap.c_allocator;
const controller_type_libretro_joypad: u8 = 4;

var g_core: ?*Core = null;
var g_loaded = false;
var environ_cb: ?retro.EnvironmentFn = null;
var video_cb: ?retro.VideoRefreshFn = null;
var audio_cb: ?retro.AudioSampleFn = null;
var audio_batch_cb: ?retro.AudioBatchFn = null;
var input_poll_cb: ?retro.InputPollFn = null;
var input_state_cb: ?retro.InputStateFn = null;
var reported_width: c_uint = 0;
var reported_height: c_uint = 0;
var port_devices = [_]c_uint{ retro.device_joypad, retro.device_joypad, retro.device_joypad, retro.device_joypad };

export fn retro_api_version() c_uint {
    return retro.api_version;
}

export fn retro_set_environment(cb: ?retro.EnvironmentFn) void {
    environ_cb = cb;
    if (environ_cb) |env| {
        var fmt: c_uint = retro.pixel_format_xrgb8888;
        _ = env(retro.environment_set_pixel_format, &fmt);
    }
}

export fn retro_set_video_refresh(cb: ?retro.VideoRefreshFn) void { video_cb = cb; }
export fn retro_set_audio_sample(cb: ?retro.AudioSampleFn) void { audio_cb = cb; }
export fn retro_set_audio_sample_batch(cb: ?retro.AudioBatchFn) void { audio_batch_cb = cb; }
export fn retro_set_input_poll(cb: ?retro.InputPollFn) void { input_poll_cb = cb; }
export fn retro_set_input_state(cb: ?retro.InputStateFn) void { input_state_cb = cb; }

export fn retro_init() void {
    const core = allocator.create(Core) catch return;
    core.* = Core.init(allocator) catch {
        allocator.destroy(core);
        return;
    };
    g_core = core;
}

export fn retro_deinit() void {
    if (g_core) |core| {
        core.deinit();
        allocator.destroy(core);
    }
    g_core = null;
}

export fn retro_get_system_info(info: ?*retro.SystemInfo) void {
    const out = info orelse return;
    out.* = .{
        .library_name = "wasm96",
        .library_version = "0.1.0",
        .valid_extensions = "wasm|wasm96",
        .need_fullpath = false,
        .block_extract = false,
    };
}

export fn retro_get_system_av_info(info: ?*retro.SystemAvInfo) void {
    const core = g_core orelse return;
    const out = info orelse return;
    out.* = .{
        .geometry = .{
            .base_width = core.fb_width,
            .base_height = core.fb_height,
            .max_width = limits.fb_max_width,
            .max_height = limits.fb_max_height,
            .aspect_ratio = @as(f32, @floatFromInt(core.fb_width)) / @as(f32, @floatFromInt(core.fb_height)),
        },
        .timing = .{ .fps = 60.0, .sample_rate = @floatFromInt(limits.audio_sample_rate) },
    };
    reported_width = core.fb_width;
    reported_height = core.fb_height;
}

export fn retro_set_controller_port_device(port: c_uint, device: c_uint) void {
    if (port >= port_devices.len) return;
    port_devices[port] = device;
    updateControllerPresence();
}

export fn retro_reset() void {
    if (!g_loaded) return;
    const core = g_core orelse return;
    g_loaded = core.reset();
    if (g_loaded) {
        core.clearControllers();
        updateControllerPresence();
    }
}

export fn retro_run() void {
    const core = g_core orelse return;
    if (!g_loaded) {
        submitVideo(core);
        return;
    }
    if (input_poll_cb) |poll| poll();
    for (0..port_devices.len) |port| samplePad(core, port);

    const result = core.runFrame();
    if (result == .guest_exited or result == .runtime_error) {
        std.debug.print("wasm96 runtime stopped: {s}\n", .{core.lastRuntimeError()});
        g_loaded = false;
        core.unload();
        return;
    }
    updateGeometryIfNeeded(core);
    submitVideo(core);
    if (result == .frame_ready) submitAudio(core);
}

export fn retro_load_game(game: ?*const retro.GameInfo) bool {
    const core = g_core orelse return false;
    const info = game orelse return false;
    const ok = if (info.data) |ptr| blk: {
        const bytes = @as([*]const u8, @ptrCast(ptr))[0..info.size];
        break :blk core.loadCartridge(bytes);
    } else if (info.path) |path| core.loadCartridgeFromPath(std.mem.span(path)) else false;
    if (ok) {
        g_loaded = true;
        core.clearControllers();
        updateControllerPresence();
    }
    return ok;
}

export fn retro_unload_game() void {
    g_loaded = false;
    if (g_core) |core| core.unload();
}

export fn retro_get_region() c_uint { return 0; }

export fn retro_get_memory_data(id: c_uint) ?*anyopaque {
    const core = g_core orelse return null;
    if (id == retro.memory_save_ram) return retro.mutAnyopaquePtr(core.sramBytes());
    return null;
}

export fn retro_get_memory_size(id: c_uint) usize {
    const core = g_core orelse return 0;
    if (id == retro.memory_save_ram) return core.sramBytes().len;
    return 0;
}

export fn retro_serialize_size() usize {
    if (!g_loaded) return 0;
    const core = g_core orelse return 0;
    return core.serializeSize();
}

export fn retro_serialize(data: ?*anyopaque, size: usize) bool {
    if (!g_loaded) return false;
    const core = g_core orelse return false;
    const ptr = data orelse return false;
    return core.serialize(@as([*]u8, @ptrCast(ptr))[0..size]);
}

export fn retro_unserialize(data: ?*const anyopaque, size: usize) bool {
    if (!g_loaded) return false;
    const core = g_core orelse return false;
    const ptr = data orelse return false;
    const ok = core.unserialize(@as([*]const u8, @ptrCast(ptr))[0..size]);
    if (ok) updateControllerPresence();
    return ok;
}

export fn retro_cheat_reset() void {}
export fn retro_cheat_set(_: c_uint, _: bool, _: ?[*:0]const u8) void {}
export fn retro_load_game_special(_: c_uint, _: ?*const retro.GameInfo, _: usize) bool { return false; }

fn updateControllerPresence() void {
    const core = g_core orelse return;
    for (port_devices, 0..) |device, port| {
        core.setControllerDevice(port, device == retro.device_joypad, controller_type_libretro_joypad);
    }
}

fn samplePad(core: *Core, port: usize) void {
    var levels = [_]u8{0} ** 12;
    const input = input_state_cb orelse {
        core.setControllerButtons(port, levels);
        return;
    };
    if (port >= port_devices.len or port_devices[port] != retro.device_joypad) {
        core.setControllerButtons(port, levels);
        return;
    }
    const p: c_uint = @intCast(port);
    levels[0] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_up));
    levels[1] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_down));
    levels[2] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_left));
    levels[3] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_right));
    levels[4] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_a));
    levels[5] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_b));
    levels[6] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_x));
    levels[7] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_y));
    levels[8] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_l));
    levels[9] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_r));
    levels[10] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_start));
    levels[11] = retro.levelFromPressed(input(p, retro.device_joypad, 0, retro.joypad_select));
    core.setControllerButtons(port, levels);
}

fn updateGeometryIfNeeded(core: *const Core) void {
    const env = environ_cb orelse return;
    if (reported_width == core.fb_width and reported_height == core.fb_height) return;
    var geometry = retro.GameGeometry{
        .base_width = core.fb_width,
        .base_height = core.fb_height,
        .max_width = limits.fb_max_width,
        .max_height = limits.fb_max_height,
        .aspect_ratio = @as(f32, @floatFromInt(core.fb_width)) / @as(f32, @floatFromInt(core.fb_height)),
    };
    _ = env(retro.environment_set_geometry, &geometry);
    reported_width = core.fb_width;
    reported_height = core.fb_height;
}

fn submitVideo(core: *const Core) void {
    const video = video_cb orelse return;
    video(retro.anyopaquePtr(core.fbBytes()), core.fb_width, core.fb_height, core.fbPitch());
}

fn submitAudio(core: *const Core) void {
    const bytes = core.audioBytes();
    const samples = @as([*]const i16, @ptrCast(@alignCast(bytes.ptr)));
    if (audio_batch_cb) |batch| {
        _ = batch(samples, limits.audio_frames_per_video);
        return;
    }
    const sample = audio_cb orelse return;
    var i: usize = 0;
    while (i + 1 < limits.audio_samples) : (i += 2) {
        _ = sample(samples[i], samples[i + 1]);
    }
}
