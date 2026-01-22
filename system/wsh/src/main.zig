const std = @import("std");
const wasm96 = @import("wasm96");

const SCREEN_WIDTH = 640;
const SCREEN_HEIGHT = 480;
const FONT_WIDTH = 8;
const FONT_HEIGHT = 16;
const COLS = SCREEN_WIDTH / FONT_WIDTH;
const ROWS = SCREEN_HEIGHT / FONT_HEIGHT;

const MAX_INPUT = 256;
const MAX_HISTORY = 32;

var terminal_buffer: [ROWS][COLS]u8 = undefined;
var color_buffer: [ROWS][COLS]u32 = undefined;
var cursor_x: usize = 0;
var cursor_y: usize = 0;

var input_buffer: [MAX_INPUT]u8 = undefined;
var input_len: usize = 0;

var current_dir: [256]u8 = undefined;
var current_dir_len: usize = 0;

var allocator_buffer: [1024 * 1024]u8 = undefined;
var fba = std.heap.FixedBufferAllocator.init(&allocator_buffer);
const allocator = fba.allocator();

export fn setup() void {
    wasm96.graphics.setSize(SCREEN_WIDTH, SCREEN_HEIGHT);
    _ = wasm96.graphics.fontRegisterSpleen("font/spleen/16", 16);
    wasm96.input.setMode(.computer);

    for (0..ROWS) |y| {
        for (0..COLS) |x| {
            terminal_buffer[y][x] = ' ';
            color_buffer[y][x] = 0xFFFFFF;
        }
    }

    const initial_path = "disk0";
    @memcpy(current_dir[0..initial_path.len], initial_path);
    current_dir_len = initial_path.len;

    print("wsh v0.1.0\n");
    printPrompt();
}

fn print(msg: []const u8) void {
    for (msg) |c| {
        if (c == '\n') {
            newline();
        } else {
            terminal_buffer[cursor_y][cursor_x] = c;
            cursor_x += 1;
            if (cursor_x >= COLS) {
                newline();
            }
        }
    }
}

fn newline() void {
    cursor_x = 0;
    if (cursor_y < ROWS - 1) {
        cursor_y += 1;
    } else {
        // Scroll
        for (0..ROWS - 1) |y| {
            terminal_buffer[y] = terminal_buffer[y + 1];
            color_buffer[y] = color_buffer[y + 1];
        }
        for (0..COLS) |x| {
            terminal_buffer[ROWS - 1][x] = ' ';
            color_buffer[ROWS - 1][x] = 0xFFFFFF;
        }
    }
}

fn printPrompt() void {
    print(current_dir[0..current_dir_len]);
    print("> ");
}

var last_control_time: u64 = 0;
const CONTROL_REPEAT_MS = 150;

export fn update() void {
    const now = wasm96.system.millis();

    // Character input (buffered by host)
    while (wasm96.input.getChar()) |c| {
        if (c >= 32 and c < 127) {
            if (input_len < MAX_INPUT - 1) {
                input_buffer[input_len] = c;
                input_len += 1;
                var buf: [1]u8 = .{c};
                print(&buf);
            }
        }
    }

    // Control keys (handled with repeat delay)
    if (now - last_control_time >= CONTROL_REPEAT_MS) {
        var control_pressed = false;

        // Backspace
        if (wasm96.input.isKeyDown(8)) {
            if (input_len > 0) {
                input_len -= 1;
                if (cursor_x > 0) {
                    cursor_x -= 1;
                    terminal_buffer[cursor_y][cursor_x] = ' ';
                }
                control_pressed = true;
            }
        }

        // Enter
        if (wasm96.input.isKeyDown(13)) {
            newline();
            executeCommand(input_buffer[0..input_len]);
            input_len = 0;
            printPrompt();
            control_pressed = true;
        }

        if (control_pressed) {
            last_control_time = now;
        }
    }
}

export fn draw() void {
    wasm96.graphics.background(20, 20, 25);

    for (0..ROWS) |y| {
        for (0..COLS) |x| {
            const char = terminal_buffer[y][x];
            if (char != ' ') {
                const color = color_buffer[y][x];
                wasm96.graphics.setColor(@intCast((color >> 16) & 0xFF), @intCast((color >> 8) & 0xFF), @intCast(color & 0xFF), 255);
                var buf: [1]u8 = .{char};
                wasm96.graphics.textKey(@intCast(x * FONT_WIDTH), @intCast(y * FONT_HEIGHT), "font/spleen/16", &buf);
            }
        }
    }

    // Draw cursor
    const blink = (wasm96.system.millis() / 500) % 2 == 0;
    if (blink) {
        wasm96.graphics.setColor(255, 255, 255, 255);
        wasm96.graphics.rect(@intCast(cursor_x * FONT_WIDTH), @intCast(cursor_y * FONT_HEIGHT + FONT_HEIGHT - 2), FONT_WIDTH, 2);
    }
}

fn executeCommand(cmd_line: []const u8) void {
    if (cmd_line.len == 0) return;

    var it = std.mem.tokenizeAny(u8, cmd_line, " ");
    const cmd = it.next() orelse return;

    if (std.ascii.eqlIgnoreCase(cmd, "help")) {
        print("Commands: ls, cd, mkdir, touch, cat, rm, rmdir, echo, clear, run, install, exit\n");
    } else if (std.ascii.eqlIgnoreCase(cmd, "clear")) {
        for (0..ROWS) |y| {
            for (0..COLS) |x| {
                terminal_buffer[y][x] = ' ';
            }
        }
        cursor_x = 0;
        cursor_y = 0;
    } else if (std.ascii.eqlIgnoreCase(cmd, "echo")) {
        while (it.next()) |arg| {
            print(arg);
            print(" ");
        }
        print("\n");
    } else if (std.ascii.eqlIgnoreCase(cmd, "ls")) {
        ls(it.next() orelse ".");
    } else if (std.ascii.eqlIgnoreCase(cmd, "cd")) {
        cd(it.next() orelse "/");
    } else if (std.ascii.eqlIgnoreCase(cmd, "mkdir")) {
        mkdir(it.next() orelse "");
    } else if (std.ascii.eqlIgnoreCase(cmd, "cat")) {
        cat(it.next() orelse "");
    } else if (std.ascii.eqlIgnoreCase(cmd, "touch")) {
        touch(it.next() orelse "");
    } else if (std.ascii.eqlIgnoreCase(cmd, "rm")) {
        rm(it.next() orelse "");
    } else if (std.ascii.eqlIgnoreCase(cmd, "run")) {
        run(it.next() orelse "", it.rest());
    } else if (std.ascii.eqlIgnoreCase(cmd, "install")) {
        install(&it);
    } else if (std.ascii.eqlIgnoreCase(cmd, "flash")) {
        flash(it.next() orelse "");
    } else if (std.ascii.eqlIgnoreCase(cmd, "rmdir")) {
        rmdir(it.next() orelse "");
    } else if (std.ascii.eqlIgnoreCase(cmd, "exit")) {
        // No-op for now
    } else {
        print("Unknown command: ");
        print(cmd);
        print("\n");
    }
}

fn ls(path: []const u8) void {
    const dir_path = resolvePath(path) catch {
        print("ls: failed to resolve path\n");
        return;
    };
    defer allocator.free(dir_path);

    var dir = std.fs.cwd().openDir(dir_path, .{ .iterate = true }) catch {
        print("ls: cannot open directory\n");
        return;
    };
    defer dir.close();

    var iter = dir.iterate();
    while (iter.next() catch return) |entry| {
        if (entry.kind == .directory) {
            print("[DIR] ");
        }
        print(entry.name);
        print("\n");
    }
}

fn cd(path: []const u8) void {
    const new_path = resolvePath(path) catch {
        print("cd: failed to resolve path\n");
        return;
    };
    defer allocator.free(new_path);

    var dir = std.fs.cwd().openDir(new_path, .{}) catch {
        print("cd: no such directory\n");
        return;
    };
    dir.close();

    @memcpy(current_dir[0..new_path.len], new_path);
    current_dir_len = new_path.len;
}

fn mkdir(path: []const u8) void {
    if (path.len == 0) return;
    const full_path = resolvePath(path) catch return;
    defer allocator.free(full_path);

    std.fs.cwd().makeDir(full_path) catch {
        print("mkdir: failed\n");
    };
}

fn touch(path: []const u8) void {
    if (path.len == 0) return;
    const full_path = resolvePath(path) catch return;
    defer allocator.free(full_path);

    const file = std.fs.cwd().createFile(full_path, .{}) catch {
        print("touch: failed\n");
        return;
    };
    file.close();
}

fn cat(path: []const u8) void {
    if (path.len == 0) return;
    const full_path = resolvePath(path) catch return;
    defer allocator.free(full_path);

    const file = std.fs.cwd().openFile(full_path, .{}) catch {
        print("cat: no such file\n");
        return;
    };
    defer file.close();

    var buf: [1024]u8 = undefined;
    while (true) {
        const bytes_read = file.read(&buf) catch break;
        if (bytes_read == 0) break;
        print(buf[0..bytes_read]);
    }
    print("\n");
}

fn rm(path: []const u8) void {
    if (path.len == 0) return;
    const full_path = resolvePath(path) catch return;
    defer allocator.free(full_path);

    std.fs.cwd().deleteFile(full_path) catch {
        print("rm: failed\n");
    };
}

fn rmdir(path: []const u8) void {
    if (path.len == 0) return;
    const full_path = resolvePath(path) catch return;
    defer allocator.free(full_path);

    std.fs.cwd().deleteDir(full_path) catch {
        print("rmdir: failed\n");
    };
}

fn run(name: []const u8, args: []const u8) void {
    if (name.len == 0) return;

    const full_path = resolvePath(name) catch return;
    defer allocator.free(full_path);

    var file = std.fs.cwd().openFile(full_path, .{}) catch blk: {
        // Try ROMS directory
        var rom_path_buf: [256]u8 = undefined;
        const rom_path = std.fmt.bufPrint(&rom_path_buf, "disk0/ROMS/{s}.w96", .{name}) catch return;
        break :blk std.fs.cwd().openFile(rom_path, .{}) catch {
            print("run: cartridge not found\n");
            return;
        };
    };
    defer file.close();

    const size = file.getEndPos() catch return;
    const data = allocator.alloc(u8, @intCast(size)) catch return;
    defer allocator.free(data);

    _ = file.read(data) catch return;

    wasm96.system.runCartridge(data, args, "");
}

fn flash(path: []const u8) void {
    if (path.len == 0) return;
    const full_path = resolvePath(path) catch return;
    defer allocator.free(full_path);

    const file = std.fs.cwd().openFile(full_path, .{}) catch {
        print("flash: file not found\n");
        return;
    };
    defer file.close();

    const size = file.getEndPos() catch return;
    const data = allocator.alloc(u8, @intCast(size)) catch return;
    defer allocator.free(data);

    _ = file.read(data) catch return;

    wasm96.system.flashCartridge(data);
}

fn install(it: *std.mem.TokenIterator(u8, .any)) void {
    const path = it.next() orelse {
        print("Usage: install <path> [update]\n");
        return;
    };
    var update_flag = false;
    if (it.next()) |arg| {
        if (std.ascii.eqlIgnoreCase(arg, "update")) {
            update_flag = true;
        }
    }

    const full_path = resolvePath(path) catch return;
    defer allocator.free(full_path);

    const file = std.fs.cwd().openFile(full_path, .{}) catch {
        print("install: source file not found\n");
        return;
    };
    defer file.close();

    const filename = std.fs.path.basename(path);
    var dest_path_buf: [256]u8 = undefined;

    // Ensure ROMS exists
    std.fs.cwd().makeDir("disk0/ROMS") catch {};

    const dest_path = std.fmt.bufPrint(&dest_path_buf, "disk0/ROMS/{s}", .{filename}) catch return;

    const dest_file = std.fs.cwd().createFile(dest_path, .{ .exclusive = !update_flag }) catch |err| {
        if (err == error.PathAlreadyExists) {
            print("install: already exists, use update\n");
        } else {
            print("install: failed to create destination\n");
        }
        return;
    };
    defer dest_file.close();

    var buf: [4096]u8 = undefined;
    while (true) {
        const bytes_read = file.read(&buf) catch break;
        if (bytes_read == 0) break;
        _ = dest_file.write(buf[0..bytes_read]) catch break;
    }
    print("Installed ");
    print(filename);
    print("\n");
}

fn resolvePath(path: []const u8) ![]u8 {
    // Handle diskN: prefixes for WASI compatibility (converts disk0:/foo to disk0/foo)
    if (std.mem.indexOfScalar(u8, path, ':')) |idx| {
        const disk = path[0..idx];
        const rest = if (idx + 1 < path.len) std.mem.trimLeft(u8, path[idx + 1 ..], "/") else "";
        return try std.fmt.allocPrint(allocator, "{s}/{s}", .{ disk, rest });
    }

    if (std.mem.startsWith(u8, path, "/")) {
        // Strip leading slash for WASI relative-to-root handling
        return try allocator.dupe(u8, std.mem.trimLeft(u8, path, "/"));
    }

    const base = current_dir[0..current_dir_len];
    if (std.mem.eql(u8, path, ".")) return try allocator.dupe(u8, base);
    if (std.mem.eql(u8, path, "..")) {
        const truncated = std.mem.trimRight(u8, base, "/");
        const last_sep = std.mem.lastIndexOfScalar(u8, truncated, '/') orelse 0;
        return try allocator.dupe(u8, base[0 .. last_sep + 1]);
    }

    const sep = if (std.mem.endsWith(u8, base, "/")) "" else "/";
    return try std.fmt.allocPrint(allocator, "{s}{s}{s}", .{ base, sep, path });
}
