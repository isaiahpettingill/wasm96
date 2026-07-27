const std = @import("std");
const wasmz = @import("wasmz");
const limits = @import("limits.zig");
const host = @import("host.zig");

const Allocator = std.mem.Allocator;

pub const RunResult = enum { running, frame_ready, guest_exited, runtime_error };

const save_magic: u64 = 0x3153565336395755; // UW96SVS1
const save_version: u32 = 1;

const SaveHeader = extern struct {
    magic: u64,
    version: u32,
    header_size: u32,
    fb_width: u32,
    fb_height: u32,
    frame_requested: u32,
    guest_exited: u32,
    presented_frames: u64,
    last_frame_delta_ms: u64,
    guest_alloc_next: u64,
    fb_guest_addr: u64,
    audio_guest_addr: u64,
    controller_guest_addrs: [limits.controller_count]u64,
    memory_size: u64,
    globals_count: u64,
    data_drop_count: u64,
    elem_drop_count: u64,
    tables_count: u64,
    table_values_count: u64,
    framebuffer_size: u64,
    audiobuffer_size: u64,
    sram_size: u64,
};

pub const Core = struct {
    allocator: Allocator,
    engine: ?wasmz.Engine = null,
    store: ?wasmz.Store = null,
    module: ?wasmz.ArcModule = null,
    instance: ?wasmz.Instance = null,
    cartridge: std.ArrayListUnmanaged(u8) = .empty,
    framebuffer: []u32 = &[_]u32{},
    audiobuffer: []i16 = &[_]i16{},
    sram: []u8 = &[_]u8{},
    controllers: [limits.controller_count][limits.controller_mapped_size]u8 = [_][limits.controller_mapped_size]u8{[_]u8{0} ** limits.controller_mapped_size} ** limits.controller_count,
    controller_connected: [limits.controller_count]bool = .{ false, false, false, false },
    controller_device_types: [limits.controller_count]u8 = .{ 0, 0, 0, 0 },
    fb_width: u32 = limits.fb_default_width,
    fb_height: u32 = limits.fb_default_height,
    frame_requested: bool = false,
    guest_exited: bool = false,
    guest_alloc_next: u64 = 0,
    fb_guest_addr: u64 = 0,
    audio_guest_addr: u64 = 0,
    controller_guest_addrs: [limits.controller_count]u64 = .{ 0, 0, 0, 0 },
    presented_frames: u64 = 0,
    last_frame_delta_ms: u64 = 0,
    last_error: std.ArrayListUnmanaged(u8) = .empty,
    debug_log: std.ArrayListUnmanaged(u8) = .empty,

    pub fn init(allocator: Allocator) !Core {
        return .{
            .allocator = allocator,
            .framebuffer = try allocator.alloc(u32, limits.fb_max_pixels),
            .audiobuffer = try allocator.alloc(i16, limits.audio_samples),
            .sram = try allocator.alloc(u8, limits.max_sram_size),
        };
    }

    pub fn deinit(self: *Core) void {
        self.unload();
        self.cartridge.deinit(self.allocator);
        self.last_error.deinit(self.allocator);
        self.debug_log.deinit(self.allocator);
        if (self.framebuffer.len != 0) self.allocator.free(self.framebuffer);
        if (self.audiobuffer.len != 0) self.allocator.free(self.audiobuffer);
        if (self.sram.len != 0) self.allocator.free(self.sram);
        self.* = undefined;
    }

    pub fn loadCartridge(self: *Core, data: []const u8) bool {
        self.clearError();
        self.unload();
        if (data.len == 0 or data.len > limits.max_cartridge_size) return self.fail("invalid cartridge size");
        self.cartridge.clearRetainingCapacity();
        self.cartridge.appendSlice(self.allocator, data) catch return self.fail("out of memory storing cartridge");
        return self.instantiate(self.cartridge.items);
    }

    pub fn loadCartridgeFromPath(self: *Core, path: [:0]const u8) bool {
        const io = std.Io.Threaded.global_single_threaded.io();
        const bytes = std.Io.Dir.cwd().readFileAlloc(io, path, self.allocator, .limited(limits.max_cartridge_size)) catch return self.fail("unable to read cartridge");
        defer self.allocator.free(bytes);
        return self.loadCartridge(bytes);
    }

    pub fn reset(self: *Core) bool {
        if (self.cartridge.items.len == 0) return false;
        const saved_sram = self.allocator.dupe(u8, self.sram) catch return false;
        defer self.allocator.free(saved_sram);
        const ok = self.instantiate(self.cartridge.items);
        if (ok) @memcpy(self.sram, saved_sram);
        return ok;
    }

    pub fn unload(self: *Core) void {
        if (self.instance) |*inst| inst.deinit();
        self.instance = null;
        if (self.module) |*arc| {
            if (arc.releaseUnwrap()) |m| {
                var mm = m;
                mm.deinit();
            }
        }
        self.module = null;
        if (self.store) |*store| store.deinit();
        self.store = null;
        if (self.engine) |*engine| engine.deinit();
        self.engine = null;
        self.resetRuntimeState();
    }

    pub fn runFrame(self: *Core) RunResult {
        self.clearError();
        self.frame_requested = false;
        if (self.instance == null) return .runtime_error;
        const result = self.instance.?.call("wasm96_update", &.{}) catch |err| {
            self.setErrorFmt("wasm96_update failed: {s}", .{@errorName(err)});
            return .runtime_error;
        };
        switch (result) {
            .ok => return if (self.guest_exited) .guest_exited else if (self.frame_requested) .frame_ready else .running,
            .trap => |trap| {
                const msg = trap.allocPrint(self.allocator) catch null;
                if (msg) |text| {
                    defer self.allocator.free(text);
                    self.setError(text);
                } else self.setError("wasm trap");
                return .runtime_error;
            },
        }
    }

    pub fn setControllerButtons(self: *Core, index: usize, levels: [12]u8) void {
        if (index >= limits.controller_count) return;
        self.controllers[index][0..3].* = .{ 0, 0, 0 };
        for (levels, 0..) |level, button| {
            const bit = button * 2;
            self.controllers[index][bit / 8] |= (level & 0x3) << @intCast(bit % 8);
        }
    }

    pub fn setControllerDevice(self: *Core, index: usize, connected: bool, device_type: u8) void {
        if (index >= limits.controller_count) return;
        self.controller_connected[index] = connected;
        self.controller_device_types[index] = if (connected) device_type else 0;
    }

    pub fn clearControllers(self: *Core) void {
        for (&self.controllers) |*controller| @memset(controller, 0);
    }

    pub fn fbPitch(self: *const Core) usize { return self.fb_width * @sizeOf(u32); }
    pub fn fbBytes(self: *const Core) []const u8 { return std.mem.sliceAsBytes(self.framebuffer[0 .. self.fb_width * self.fb_height]); }
    pub fn audioBytes(self: *const Core) []const u8 { return std.mem.sliceAsBytes(self.audiobuffer); }
    pub fn sramBytes(self: *Core) []u8 { return self.sram; }
    pub fn lastRuntimeError(self: *const Core) []const u8 { return self.last_error.items; }

    pub fn serializeSize(self: *Core) usize {
        const inst = &(self.instance orelse return 0);
        var table_values: usize = 0;
        for (inst.module.value.tables) |table| table_values += table.len;
        return @sizeOf(SaveHeader) + inst.memory.bytes().len + inst.globals.len * @sizeOf(u64) +
            inst.data_segments_dropped.len + inst.elem_segments_dropped.len +
            inst.module.value.tables.len * @sizeOf(u64) + table_values * @sizeOf(u32) +
            self.framebuffer.len * @sizeOf(u32) + self.audiobuffer.len * @sizeOf(i16) + self.sram.len;
    }

    pub fn serialize(self: *Core, out: []u8) bool {
        const inst = &(self.instance orelse return false);
        const need = self.serializeSize();
        if (out.len < need) return false;
        var table_values: usize = 0;
        for (inst.module.value.tables) |table| table_values += table.len;
        const header = SaveHeader{
            .magic = save_magic,
            .version = save_version,
            .header_size = @sizeOf(SaveHeader),
            .fb_width = self.fb_width,
            .fb_height = self.fb_height,
            .frame_requested = if (self.frame_requested) 1 else 0,
            .guest_exited = if (self.guest_exited) 1 else 0,
            .presented_frames = self.presented_frames,
            .last_frame_delta_ms = self.last_frame_delta_ms,
            .guest_alloc_next = self.guest_alloc_next,
            .fb_guest_addr = self.fb_guest_addr,
            .audio_guest_addr = self.audio_guest_addr,
            .controller_guest_addrs = self.controller_guest_addrs,
            .memory_size = inst.memory.bytes().len,
            .globals_count = inst.globals.len,
            .data_drop_count = inst.data_segments_dropped.len,
            .elem_drop_count = inst.elem_segments_dropped.len,
            .tables_count = inst.module.value.tables.len,
            .table_values_count = table_values,
            .framebuffer_size = self.framebuffer.len * @sizeOf(u32),
            .audiobuffer_size = self.audiobuffer.len * @sizeOf(i16),
            .sram_size = self.sram.len,
        };
        var writer = SliceWriter{ .buf = out };
        writer.write(std.mem.asBytes(&header));
        writer.write(inst.memory.bytes());
        for (inst.globals) |global| writer.writeInt(u64, global.value.toBits64());
        writer.writeBools(inst.data_segments_dropped);
        writer.writeBools(inst.elem_segments_dropped);
        for (inst.module.value.tables) |table| {
            writer.writeInt(u64, table.len);
            writer.write(std.mem.sliceAsBytes(table));
        }
        writer.write(std.mem.sliceAsBytes(self.framebuffer));
        writer.write(std.mem.sliceAsBytes(self.audiobuffer));
        writer.write(self.sram);
        return writer.pos == need;
    }

    pub fn unserialize(self: *Core, data: []const u8) bool {
        const inst = &(self.instance orelse return false);
        if (data.len < @sizeOf(SaveHeader)) return false;
        var reader = SliceReader{ .buf = data };
        const header = reader.readStruct(SaveHeader) orelse return false;
        if (header.magic != save_magic or header.version != save_version or header.header_size != @sizeOf(SaveHeader)) return false;
        if (header.memory_size > limits.max_guest_ram_size or header.sram_size != self.sram.len) return false;
        if (header.globals_count != inst.globals.len or header.data_drop_count != inst.data_segments_dropped.len) return false;
        if (header.elem_drop_count != inst.elem_segments_dropped.len or header.tables_count != inst.module.value.tables.len) return false;
        if (header.framebuffer_size != self.framebuffer.len * @sizeOf(u32) or header.audiobuffer_size != self.audiobuffer.len * @sizeOf(i16)) return false;

        if (!ensureMemorySize(inst, header.memory_size)) return false;
        const mem = inst.memory.bytes();
        const mem_src = reader.read(@intCast(header.memory_size)) orelse return false;
        @memcpy(mem[0..mem_src.len], mem_src);
        if (mem.len > mem_src.len) @memset(mem[mem_src.len..], 0);
        for (inst.globals) |*global| global.value = wasmz.RawVal.fromBits64(reader.readInt(u64) orelse return false);
        if (!reader.readBools(inst.data_segments_dropped)) return false;
        if (!reader.readBools(inst.elem_segments_dropped)) return false;
        var seen_table_values: u64 = 0;
        for (inst.module.value.tables) |table| {
            const len = reader.readInt(u64) orelse return false;
            if (len != table.len) return false;
            const bytes = reader.read(table.len * @sizeOf(u32)) orelse return false;
            @memcpy(std.mem.sliceAsBytes(table), bytes);
            seen_table_values += table.len;
        }
        if (seen_table_values != header.table_values_count) return false;
        @memcpy(std.mem.sliceAsBytes(self.framebuffer), reader.read(self.framebuffer.len * @sizeOf(u32)) orelse return false);
        @memcpy(std.mem.sliceAsBytes(self.audiobuffer), reader.read(self.audiobuffer.len * @sizeOf(i16)) orelse return false);
        @memcpy(self.sram, reader.read(self.sram.len) orelse return false);
        self.fb_width = std.math.clamp(header.fb_width, 1, limits.fb_max_width);
        self.fb_height = std.math.clamp(header.fb_height, 1, limits.fb_max_height);
        self.frame_requested = header.frame_requested != 0;
        self.guest_exited = header.guest_exited != 0;
        self.presented_frames = header.presented_frames;
        self.last_frame_delta_ms = header.last_frame_delta_ms;
        self.guest_alloc_next = header.guest_alloc_next;
        self.fb_guest_addr = header.fb_guest_addr;
        self.audio_guest_addr = header.audio_guest_addr;
        self.controller_guest_addrs = header.controller_guest_addrs;
        return true;
    }

    fn instantiate(self: *Core, data: []const u8) bool {
        self.unload();
        self.resetRuntimeState();
        @memset(self.framebuffer, 0);
        @memset(self.audiobuffer, 0);
        const engine = wasmz.Engine.init(self.allocator, .{ .mem_limit_bytes = limits.max_guest_ram_size }) catch return self.fail("engine init failed");
        self.engine = engine;
        var arc_module = wasmz.Module.compileArc(self.engine.?, data) catch |err| {
            self.setErrorFmt("module compile failed: {s}", .{@errorName(err)});
            return false;
        };
        self.module = arc_module;
        var store = wasmz.Store.init(self.allocator, self.engine.?, std.Io.Threaded.global_single_threaded.io()) catch return self.fail("store init failed");
        store.linkBudget();
        self.store = store;
        var linker = wasmz.Linker.empty;
        defer linker.deinit(self.allocator);
        host.addToLinker(&linker, self.allocator, self) catch return self.fail("host linker setup failed");
        const inst = wasmz.Instance.init(&self.store.?, arc_module.retain(), linker) catch |err| {
            self.setErrorFmt("instantiate failed: {s}", .{@errorName(err)});
            return false;
        };
        self.instance = inst;
        const host_arena_base = self.instance.?.memory.bytes().len;
        self.guest_alloc_next = host_arena_base;
        const host_arena_size = limits.fb_max_size + limits.audio_mapped_size +
            limits.controller_count * limits.controller_mapped_size + 64 * 1024;
        if (!ensureMemorySize(&self.instance.?, host_arena_base + host_arena_size)) return self.fail("guest memory reserve failed");
        if (self.instance.?.runStartFunction() catch |err| {
            self.setErrorFmt("start section failed: {s}", .{@errorName(err)});
            return false;
        }) |result| if (result == .trap) return self.fail("start section trapped");
        if (self.instance.?.initializeReactor() catch |err| {
            self.setErrorFmt("_initialize failed: {s}", .{@errorName(err)});
            return false;
        }) |result| if (result == .trap) return self.fail("_initialize trapped");
        if (self.instance.?.module.value.exports.get("wasm96_update") == null) return self.fail("missing wasm96_update export");
        return true;
    }

    fn resetRuntimeState(self: *Core) void {
        self.fb_width = limits.fb_default_width;
        self.fb_height = limits.fb_default_height;
        self.frame_requested = false;
        self.guest_exited = false;
        self.guest_alloc_next = 0;
        self.fb_guest_addr = 0;
        self.audio_guest_addr = 0;
        self.controller_guest_addrs = .{ 0, 0, 0, 0 };
        self.presented_frames = 0;
        self.last_frame_delta_ms = 0;
        self.debug_log.clearRetainingCapacity();
    }

    fn clearError(self: *Core) void { self.last_error.clearRetainingCapacity(); }
    fn setError(self: *Core, msg: []const u8) void {
        self.last_error.clearRetainingCapacity();
        self.last_error.appendSlice(self.allocator, msg) catch {};
    }
    fn setErrorFmt(self: *Core, comptime fmt: []const u8, args: anytype) void {
        self.last_error.clearRetainingCapacity();
        const msg = std.fmt.allocPrint(self.allocator, fmt, args) catch return;
        defer self.allocator.free(msg);
        self.last_error.appendSlice(self.allocator, msg) catch {};
    }
    fn fail(self: *Core, msg: []const u8) bool {
        self.setError(msg);
        return false;
    }
};

fn ensureMemorySize(inst: *wasmz.Instance, required: u64) bool {
    const current = inst.memory.bytes().len;
    if (required <= current) return true;
    const page = 64 * 1024;
    const required_pages = (required + page - 1) / page;
    const current_pages = (current + page - 1) / page;
    const old = inst.memory.grow(required_pages - current_pages);
    return old != std.math.maxInt(u64);
}

const SliceWriter = struct {
    buf: []u8,
    pos: usize = 0,

    fn write(self: *SliceWriter, bytes: []const u8) void {
        @memcpy(self.buf[self.pos..][0..bytes.len], bytes);
        self.pos += bytes.len;
    }

    fn writeInt(self: *SliceWriter, comptime T: type, value: T) void {
        var copy = value;
        self.write(std.mem.asBytes(&copy));
    }

    fn writeBools(self: *SliceWriter, values: []const bool) void {
        for (values) |value| {
            self.buf[self.pos] = if (value) 1 else 0;
            self.pos += 1;
        }
    }
};

const SliceReader = struct {
    buf: []const u8,
    pos: usize = 0,

    fn read(self: *SliceReader, len: usize) ?[]const u8 {
        if (self.pos + len > self.buf.len) return null;
        const out = self.buf[self.pos..][0..len];
        self.pos += len;
        return out;
    }

    fn readStruct(self: *SliceReader, comptime T: type) ?T {
        const bytes = self.read(@sizeOf(T)) orelse return null;
        return std.mem.bytesToValue(T, bytes[0..@sizeOf(T)]);
    }

    fn readInt(self: *SliceReader, comptime T: type) ?T { return self.readStruct(T); }

    fn readBools(self: *SliceReader, out: []bool) bool {
        const bytes = self.read(out.len) orelse return false;
        for (bytes, 0..) |byte, i| out[i] = byte != 0;
        return true;
    }
};
