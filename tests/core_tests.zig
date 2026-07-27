const std = @import("std");
const Core = @import("wasm96_core").Core;

fn sampleBytes(allocator: std.mem.Allocator) ![]u8 {
    return std.Io.Dir.cwd().readFileAlloc(
        std.Io.Threaded.global_single_threaded.io(),
        "templates/cartridge-c/cartridge.wasm",
        allocator,
        .limited(128 * 1024 * 1024),
    );
}

fn loadSample(core: *Core, allocator: std.mem.Allocator) !void {
    const bytes = try sampleBytes(allocator);
    defer allocator.free(bytes);
    try std.testing.expect(core.loadCartridge(bytes));
}

fn hash(bytes: []const u8) u64 {
    return std.hash.Wyhash.hash(0, bytes);
}

fn expectFrame(core: *Core) !void {
    const result = core.runFrame();
    if (result != .frame_ready) std.debug.print("runFrame failed: {s}\n", .{core.lastRuntimeError()});
    try std.testing.expectEqual(.frame_ready, result);
}

test "invalid cartridges are rejected" {
    var core = try Core.init(std.testing.allocator);
    defer core.deinit();
    try std.testing.expect(!core.loadCartridge(&.{}));
    try std.testing.expect(!core.loadCartridge("not wasm"));
}

test "sample cartridge reaches frame and serializes" {
    const allocator = std.testing.allocator;
    var core = try Core.init(allocator);
    defer core.deinit();
    try loadSample(&core, allocator);

    try expectFrame(&core);
    try std.testing.expectEqual(@as(u32, 640), core.fb_width);
    try std.testing.expectEqual(@as(u32, 360), core.fb_height);
    try std.testing.expect(core.fbBytes().len == 640 * 360 * 4);
    try std.testing.expect(core.audioBytes().len == 800 * 2 * 2);
    try std.testing.expect(hash(core.fbBytes()) != 0);
    try std.testing.expect(hash(core.audioBytes()) != 0);

    const state_size = core.serializeSize();
    try std.testing.expect(state_size > 0);
    const state = try allocator.alloc(u8, state_size);
    defer allocator.free(state);
    try std.testing.expect(core.serialize(state));

    try expectFrame(&core);
    try std.testing.expect(core.unserialize(state));
    try expectFrame(&core);
}

test "deterministic frames match across cores" {
    const allocator = std.testing.allocator;
    var first = try Core.init(allocator);
    defer first.deinit();
    try loadSample(&first, allocator);
    var second = try Core.init(allocator);
    defer second.deinit();
    try loadSample(&second, allocator);

    var levels = [_]u8{0} ** 12;
    levels[3] = 3;
    levels[4] = 3;
    first.setControllerButtons(0, levels);
    second.setControllerButtons(0, levels);

    for (0..6) |_| {
        try expectFrame(&first);
        try expectFrame(&second);
        try std.testing.expectEqual(hash(first.fbBytes()), hash(second.fbBytes()));
        try std.testing.expectEqual(hash(first.audioBytes()), hash(second.audioBytes()));
    }
}

test "save restore preserves SRAM and frame progression" {
    const allocator = std.testing.allocator;
    var continuous = try Core.init(allocator);
    defer continuous.deinit();
    try loadSample(&continuous, allocator);
    var restored = try Core.init(allocator);
    defer restored.deinit();
    try loadSample(&restored, allocator);

    for (0..3) |_| {
        try expectFrame(&continuous);
        try expectFrame(&restored);
    }
    restored.sram[0] = 0xaa;
    restored.sram[restored.sram.len - 1] = 0x55;

    const state = try allocator.alloc(u8, restored.serializeSize());
    defer allocator.free(state);
    try std.testing.expect(restored.serialize(state));

    for (0..4) |_| try expectFrame(&continuous);
    try std.testing.expect(restored.reset());
    try std.testing.expect(restored.unserialize(state));
    try std.testing.expectEqual(@as(u8, 0xaa), restored.sram[0]);
    try std.testing.expectEqual(@as(u8, 0x55), restored.sram[restored.sram.len - 1]);
    for (0..4) |_| try expectFrame(&restored);

    try std.testing.expectEqual(hash(continuous.fbBytes()), hash(restored.fbBytes()));
    try std.testing.expectEqual(hash(continuous.audioBytes()), hash(restored.audioBytes()));
}
