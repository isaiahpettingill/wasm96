const std = @import("std");
const wasmz = @import("wasmz");
const limits = @import("limits.zig");
const core_mod = @import("core.zig");

const Core = core_mod.Core;
const HostContext = wasmz.HostContext;
const RawVal = wasmz.RawVal;
const HostError = wasmz.HostError;
const ValType = wasmz.ValType;

const i32_params = [_]ValType{.I32};
const i32_i32_params = [_]ValType{ .I32, .I32 };
const i32_i32_i32_params = [_]ValType{ .I32, .I32, .I32 };
const i32_result = [_]ValType{.I32};
const no_params = [_]ValType{};
const no_results = [_]ValType{};

pub fn addToLinker(linker: *wasmz.Linker, allocator: std.mem.Allocator, core: *Core) !void {
    try define(linker, allocator, core, "get_framebuffer", getFramebuffer, &no_params, &i32_result);
    try define(linker, allocator, core, "get_audiobuffer", getAudiobuffer, &no_params, &i32_result);
    try define(linker, allocator, core, "controller_1", controller1, &no_params, &i32_result);
    try define(linker, allocator, core, "controller_2", controller2, &no_params, &i32_result);
    try define(linker, allocator, core, "controller_3", controller3, &no_params, &i32_result);
    try define(linker, allocator, core, "controller_4", controller4, &no_params, &i32_result);
    try define(linker, allocator, core, "present", present, &no_params, &no_results);
    try define(linker, allocator, core, "set_resolution", setResolution, &i32_i32_params, &i32_result);
    try define(linker, allocator, core, "sram_size", sramSize, &no_params, &i32_result);
    try define(linker, allocator, core, "sram_read", sramRead, &i32_i32_i32_params, &i32_result);
    try define(linker, allocator, core, "sram_write", sramWrite, &i32_i32_i32_params, &i32_result);
    try define(linker, allocator, core, "controller_count", controllerCount, &no_params, &i32_result);
    try define(linker, allocator, core, "controller_info", controllerInfo, &i32_params, &i32_result);
    try define(linker, allocator, core, "time_ms", timeMs, &no_params, &i32_result);
    try define(linker, allocator, core, "delta_ms", deltaMs, &no_params, &i32_result);
    try define(linker, allocator, core, "debug_log", debugLog, &i32_i32_params, &i32_result);
    try define(linker, allocator, core, "debug_trace", debugTrace, &i32_i32_i32_params, &no_results);
    try define(linker, allocator, core, "debug_mem_read", debugMemRead, &i32_i32_i32_params, &i32_result);
    try define(linker, allocator, core, "debug_mem_write", debugMemWrite, &i32_i32_i32_params, &i32_result);
    try define(linker, allocator, core, "exit", exitGuest, &i32_params, &no_results);
    try define(linker, allocator, core, "exit_group", exitGuest, &i32_params, &no_results);
}

fn define(
    linker: *wasmz.Linker,
    allocator: std.mem.Allocator,
    core: *Core,
    name: []const u8,
    func: *const fn (?*anyopaque, *HostContext, []const RawVal, []RawVal) HostError!void,
    params: []const ValType,
    results: []const ValType,
) !void {
    try linker.define(allocator, "env", name, wasmz.HostFunc.init(core, func, params, results));
}

fn state(raw: ?*anyopaque) *Core {
    return @ptrCast(@alignCast(raw.?));
}

fn readU32(params: []const RawVal, index: usize) u32 {
    return params[index].readAs(u32);
}

fn resultU32(results: []RawVal, value: u32) void {
    results[0] = RawVal.from(@as(i32, @bitCast(value)));
}

fn frameTimeMs(frame_count: u64) u64 {
    return (frame_count * 1000) / limits.video_fps;
}

fn allocGuest(core: *Core, ctx: *HostContext, size: usize) HostError!u32 {
    const mem = ctx.memory() orelse {
        try ctx.raiseTrap(wasmz.Trap.fromTrapCode(.MemoryOutOfBounds));
        unreachable;
    };
    if (core.guest_alloc_next == 0) core.guest_alloc_next = alignForward(mem.len, 16);
    const addr = alignForward(core.guest_alloc_next, 16);
    const end = std.math.add(u64, addr, size) catch {
        try ctx.raiseTrap(wasmz.Trap.fromTrapCode(.MemoryOutOfBounds));
        unreachable;
    };
    if (end > limits.max_guest_ram_size) {
        try ctx.raiseTrap(wasmz.Trap.fromTrapCode(.MemoryOutOfBounds));
        unreachable;
    }
    if (end > mem.len) {
        const page: u64 = 64 * 1024;
        const current_pages = (@as(u64, mem.len) + page - 1) / page;
        const required_pages = (end + page - 1) / page;
        const old = ctx.instance().memory.grow(required_pages - current_pages);
        if (old == std.math.maxInt(u64)) {
            try ctx.raiseTrap(wasmz.Trap.fromTrapCode(.MemoryOutOfBounds));
            unreachable;
        }
    }
    core.guest_alloc_next = end;
    return @intCast(addr);
}

fn alignForward(value: u64, alignment: u64) u64 {
    return (value + alignment - 1) & ~(alignment - 1);
}

fn getFramebuffer(raw: ?*anyopaque, ctx: *HostContext, _: []const RawVal, results: []RawVal) HostError!void {
    const core = state(raw);
    if (core.fb_guest_addr == 0) {
        core.fb_guest_addr = try allocGuest(core, ctx, limits.fb_max_size);
        try ctx.writeBytes(core.fb_guest_addr, &([_]u8{0} ** 1));
    }
    resultU32(results, @intCast(core.fb_guest_addr));
}

fn getAudiobuffer(raw: ?*anyopaque, ctx: *HostContext, _: []const RawVal, results: []RawVal) HostError!void {
    const core = state(raw);
    if (core.audio_guest_addr == 0) core.audio_guest_addr = try allocGuest(core, ctx, limits.audio_mapped_size);
    resultU32(results, @intCast(core.audio_guest_addr));
}

fn controllerAt(raw: ?*anyopaque, ctx: *HostContext, results: []RawVal, comptime index: usize) HostError!void {
    const core = state(raw);
    if (core.controller_guest_addrs[index] == 0) core.controller_guest_addrs[index] = try allocGuest(core, ctx, limits.controller_mapped_size);
    try ctx.writeBytes(core.controller_guest_addrs[index], core.controllers[index][0..limits.controller_size]);
    resultU32(results, @intCast(core.controller_guest_addrs[index]));
}

fn controller1(raw: ?*anyopaque, ctx: *HostContext, _: []const RawVal, results: []RawVal) HostError!void { try controllerAt(raw, ctx, results, 0); }
fn controller2(raw: ?*anyopaque, ctx: *HostContext, _: []const RawVal, results: []RawVal) HostError!void { try controllerAt(raw, ctx, results, 1); }
fn controller3(raw: ?*anyopaque, ctx: *HostContext, _: []const RawVal, results: []RawVal) HostError!void { try controllerAt(raw, ctx, results, 2); }
fn controller4(raw: ?*anyopaque, ctx: *HostContext, _: []const RawVal, results: []RawVal) HostError!void { try controllerAt(raw, ctx, results, 3); }

fn present(raw: ?*anyopaque, ctx: *HostContext, _: []const RawVal, _: []RawVal) HostError!void {
    const core = state(raw);
    const before = frameTimeMs(core.presented_frames);
    core.presented_frames += 1;
    const after = frameTimeMs(core.presented_frames);
    core.last_frame_delta_ms = after - before;
    if (core.fb_guest_addr != 0) {
        const bytes = try ctx.readBytes(core.fb_guest_addr, @as(usize, core.fb_width) * core.fb_height * @sizeOf(u32));
        @memcpy(std.mem.sliceAsBytes(core.framebuffer[0 .. core.fb_width * core.fb_height]), bytes);
    }
    if (core.audio_guest_addr != 0) {
        const bytes = try ctx.readBytes(core.audio_guest_addr, limits.audio_size);
        @memcpy(std.mem.sliceAsBytes(core.audiobuffer), bytes);
    }
    core.frame_requested = true;
}

fn setResolution(raw: ?*anyopaque, _: *HostContext, params: []const RawVal, results: []RawVal) HostError!void {
    const core = state(raw);
    core.fb_width = std.math.clamp(readU32(params, 0), 1, limits.fb_max_width);
    core.fb_height = std.math.clamp(readU32(params, 1), 1, limits.fb_max_height);
    resultU32(results, (core.fb_height << 16) | core.fb_width);
}

fn sramSize(raw: ?*anyopaque, _: *HostContext, _: []const RawVal, results: []RawVal) HostError!void {
    resultU32(results, @intCast(state(raw).sram.len));
}

fn sramRead(raw: ?*anyopaque, ctx: *HostContext, params: []const RawVal, results: []RawVal) HostError!void {
    const core = state(raw);
    const offset = readU32(params, 0);
    const dst = readU32(params, 1);
    const len = readU32(params, 2);
    if (offset >= core.sram.len or dst == 0 or len == 0) return resultU32(results, 0);
    const count = @min(@as(usize, len), core.sram.len - offset);
    try ctx.writeBytes(dst, core.sram[offset..][0..count]);
    resultU32(results, @intCast(count));
}

fn sramWrite(raw: ?*anyopaque, ctx: *HostContext, params: []const RawVal, results: []RawVal) HostError!void {
    const core = state(raw);
    const offset = readU32(params, 0);
    const src = readU32(params, 1);
    const len = readU32(params, 2);
    if (offset >= core.sram.len or src == 0 or len == 0) return resultU32(results, 0);
    const count = @min(@as(usize, len), core.sram.len - offset);
    const bytes = try ctx.readBytes(src, count);
    @memcpy(core.sram[offset..][0..count], bytes);
    resultU32(results, @intCast(count));
}

fn controllerCount(raw: ?*anyopaque, _: *HostContext, _: []const RawVal, results: []RawVal) HostError!void {
    var count: u32 = 0;
    for (state(raw).controller_connected) |connected| {
        if (connected) count += 1;
    }
    resultU32(results, count);
}

fn controllerInfo(raw: ?*anyopaque, _: *HostContext, params: []const RawVal, results: []RawVal) HostError!void {
    const core = state(raw);
    const port = readU32(params, 0);
    if (port >= limits.controller_count) return resultU32(results, 0);
    const connected: u32 = if (core.controller_connected[port]) 1 else 0;
    resultU32(results, connected | (@as(u32, core.controller_device_types[port]) << 8));
}

fn timeMs(raw: ?*anyopaque, _: *HostContext, _: []const RawVal, results: []RawVal) HostError!void {
    resultU32(results, @truncate(frameTimeMs(state(raw).presented_frames)));
}

fn deltaMs(raw: ?*anyopaque, _: *HostContext, _: []const RawVal, results: []RawVal) HostError!void {
    resultU32(results, @truncate(state(raw).last_frame_delta_ms));
}

fn debugLog(raw: ?*anyopaque, ctx: *HostContext, params: []const RawVal, results: []RawVal) HostError!void {
    const src = readU32(params, 0);
    const len = readU32(params, 1);
    if (src == 0 or len == 0) return resultU32(results, 0);
    const bytes = try ctx.readBytes(src, len);
    state(raw).debug_log.appendSlice(state(raw).allocator, bytes) catch {};
    std.debug.print("{s}", .{bytes});
    resultU32(results, len);
}

fn debugTrace(raw: ?*anyopaque, _: *HostContext, params: []const RawVal, _: []RawVal) HostError!void {
    const core = state(raw);
    const msg = std.fmt.allocPrint(core.allocator, "wasm96 trace: {d} {d} {d}\n", .{ readU32(params, 0), readU32(params, 1), readU32(params, 2) }) catch return;
    defer core.allocator.free(msg);
    core.debug_log.appendSlice(core.allocator, msg) catch {};
}

fn debugMemRead(_: ?*anyopaque, ctx: *HostContext, params: []const RawVal, results: []RawVal) HostError!void {
    const src = readU32(params, 0);
    const dst = readU32(params, 1);
    const len = readU32(params, 2);
    if (src == 0 or dst == 0 or len == 0) return resultU32(results, 0);
    const bytes = try ctx.readBytes(src, len);
    try ctx.writeBytes(dst, bytes);
    resultU32(results, len);
}

fn debugMemWrite(_: ?*anyopaque, ctx: *HostContext, params: []const RawVal, results: []RawVal) HostError!void {
    const dst = readU32(params, 0);
    const src = readU32(params, 1);
    const len = readU32(params, 2);
    if (src == 0 or dst == 0 or len == 0) return resultU32(results, 0);
    const bytes = try ctx.readBytes(src, len);
    try ctx.writeBytes(dst, bytes);
    resultU32(results, len);
}

fn exitGuest(raw: ?*anyopaque, _: *HostContext, _: []const RawVal, _: []RawVal) HostError!void {
    state(raw).guest_exited = true;
}
