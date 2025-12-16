const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    });
    const optimize = b.standardOptimizeOption(.{});

    const sdk_mod = b.createModule(.{
        .root_source_file = b.path("../../wasm96-zig-sdk/src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    const exe_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{
            .{ .name = "wasm96", .module = sdk_mod },
        },
    });

    const exe = b.addExecutable(.{
        .name = "zig-guest",
        .root_module = exe_mod,
    });
    exe.entry = .disabled;

    b.installArtifact(exe);
}
