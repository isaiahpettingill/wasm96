const std = @import("std");

pub fn build(b: *std.Build) void {
    // For wasm96 guests we want a simple freestanding wasm module that exports
    // `setup`, `update`, and `draw` (provided by `src/main.zig` via `export fn`),
    // and does NOT require a WASI `_start` entrypoint.
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
        .name = "zig-guest-3d",
        .root_module = exe_mod,
    });

    // Ensure we don't accidentally produce a stub module with only memory exported.
    // Guests should be pure libraries-from-host perspective with explicit exports.
    exe.entry = .disabled;
    exe.rdynamic = true;

    b.installArtifact(exe);
}
