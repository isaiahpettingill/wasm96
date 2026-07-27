const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const wasmz_path = "third_party/wasmz";
    const build_options = b.addOptions();
    build_options.addOption(bool, "profiling", false);

    const zigrc_mod = b.createModule(.{
        .root_source_file = b.path(wasmz_path ++ "/src/libs/zigrc/root.zig"),
        .target = target,
    });
    const payload_mod = b.createModule(.{
        .root_source_file = b.path(wasmz_path ++ "/src/parser/payload.zig"),
        .target = target,
        .optimize = optimize,
    });
    const parser_mod = b.createModule(.{
        .root_source_file = b.path(wasmz_path ++ "/src/parser/root.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{.{ .name = "payload", .module = payload_mod }},
    });
    const wasm_core_mod = b.createModule(.{
        .root_source_file = b.path(wasmz_path ++ "/src/core/root.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{.{ .name = "payload", .module = payload_mod }},
    });
    const wasmz_mod = b.createModule(.{
        .root_source_file = b.path(wasmz_path ++ "/src/root.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{
            .{ .name = "zigrc", .module = zigrc_mod },
            .{ .name = "parser", .module = parser_mod },
            .{ .name = "payload", .module = payload_mod },
            .{ .name = "core", .module = wasm_core_mod },
            .{ .name = "build_options", .module = build_options.createModule() },
        },
    });

    const lib = b.addLibrary(.{
        .name = "wasm96_libretro",
        .linkage = .dynamic,
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/libretro.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{.{ .name = "wasmz", .module = wasmz_mod }},
            .link_libc = true,
        }),
    });
    b.installArtifact(lib);
    b.getInstallStep().dependOn(&b.addInstallFile(b.path("wasm96_libretro.info"), "lib/wasm96_libretro.info").step);

    const check = b.step("check", "Build the libretro core");
    check.dependOn(&b.addInstallArtifact(lib, .{}).step);

    const wasm96_core_mod = b.createModule(.{
        .root_source_file = b.path("src/core.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{.{ .name = "wasmz", .module = wasmz_mod }},
        .link_libc = true,
    });

    const tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("tests/core_tests.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{
                .{ .name = "wasmz", .module = wasmz_mod },
                .{ .name = "wasm96_core", .module = wasm96_core_mod },
            },
            .link_libc = true,
        }),
    });
    const test_cartridge = b.addSystemCommand(&.{
        "zig",
        "cc",
        "-target",
        "wasm32-freestanding",
        "-O2",
        "-ffreestanding",
        "-nostdlib",
        "-Wall",
        "-Wextra",
        "templates/cartridge-c/main.c",
        "-Wl,--no-entry",
        "-Wl,--export=wasm96_update",
        "-Wl,--initial-memory=2097152",
        "-Wl,--max-memory=134217728",
        "-o",
        "templates/cartridge-c/cartridge.wasm",
    });
    const test_step = b.step("test", "Run wasm96 core tests");
    test_step.dependOn(&test_cartridge.step);
    test_step.dependOn(&b.addRunArtifact(tests).step);
}
