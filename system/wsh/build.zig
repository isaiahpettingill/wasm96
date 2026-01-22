const std = @import("std");

pub fn build(b: *std.Build) void {
    // For wsh we use the WASI target to enable filesystem access via WASI imports,
    // while still exporting setup/update/draw for the wasm96 engine.
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .wasi,
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
        .name = "wsh",
        .root_module = exe_mod,
    });

    // Ensure we don't accidentally produce a stub module with only memory exported.
    // Guests should be pure libraries-from-host perspective with explicit exports.
    exe.entry = .disabled;
    exe.rdynamic = true;

    b.installArtifact(exe);
}
