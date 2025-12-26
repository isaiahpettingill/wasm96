const std = @import("std");
const wasm96 = @import("wasm96");

/// Rolling-sphere sandbox:
/// - You roll around on a flat ground plane.
/// - Two bird OBJ models are loaded and rendered as simple scene props.
///
/// Controls (joypad port 0):
/// - DPad: camera pitch
/// - L1/R1: camera yaw
/// - A: accelerate forward (relative to camera yaw)
/// - B: brake
/// - Y: jump
/// - Start: reset
///
/// Notes:
/// - Physics is intentionally simple (semi-implicit Euler + ground collision).
/// - If the core has `meshCreateObj` stubbed, the birds won't render (but you can still roll).
const SCREEN_W: u32 = 640;
const SCREEN_H: u32 = 480;

const Vec3 = struct {
    x: f32,
    y: f32,
    z: f32,

    fn add(a: Vec3, b: Vec3) Vec3 {
        return .{ .x = a.x + b.x, .y = a.y + b.y, .z = a.z + b.z };
    }

    fn sub(a: Vec3, b: Vec3) Vec3 {
        return .{ .x = a.x - b.x, .y = a.y - b.y, .z = a.z - b.z };
    }

    fn mul(a: Vec3, s: f32) Vec3 {
        return .{ .x = a.x * s, .y = a.y * s, .z = a.z * s };
    }

    fn dot(a: Vec3, b: Vec3) f32 {
        return a.x * b.x + a.y * b.y + a.z * b.z;
    }

    fn len(a: Vec3) f32 {
        return @sqrt(a.dot(a));
    }

    fn clampLen(a: Vec3, max_len: f32) Vec3 {
        const l = a.len();
        if (l <= max_len) return a;
        return a.mul(max_len / l);
    }
};

const Game = struct {
    // Player
    pos: Vec3 = .{ .x = 0.0, .y = 1.0, .z = 0.0 },
    vel: Vec3 = .{ .x = 0.0, .y = 0.0, .z = 0.0 },
    on_ground: bool = false,

    // Camera
    cam_yaw: f32 = 0.0,
    cam_pitch: f32 = 0.6,
    cam_roll: f32 = 0.0,
    cam_dist: f32 = 12.0,

    // Accumulator
    last_ms: u64 = 0,

    fn reset(self: *Game) void {
        self.pos = .{ .x = 0.0, .y = 1.0, .z = 0.0 };
        self.vel = .{ .x = 0.0, .y = 0.0, .z = 0.0 };
        self.on_ground = false;
        self.cam_yaw = 0.0;
        self.cam_pitch = 0.6;
        self.cam_roll = 0.0;
        self.cam_dist = 12.0;
    }
};

var g: Game = .{};

var bird1_loaded: bool = false;
var bird2_loaded: bool = false;

// ---- Geometry helpers ----

fn pushSphereVerts(
    buf: []f32,
    write_index: *usize,
    radius: f32,
    stacks: u32,
    slices: u32,
) void {
    // Vertex layout: [x,y,z, u,v, nx,ny,nz]
    var i: u32 = 0;
    while (i <= stacks) : (i += 1) {
        const v = @as(f32, @floatFromInt(i)) / @as(f32, @floatFromInt(stacks));
        const phi = v * std.math.pi; // 0..pi
        const sin_phi = @sin(phi);
        const cos_phi = @cos(phi);

        var j: u32 = 0;
        while (j <= slices) : (j += 1) {
            const u = @as(f32, @floatFromInt(j)) / @as(f32, @floatFromInt(slices));
            const theta = u * (2.0 * std.math.pi); // 0..2pi
            const sin_theta = @sin(theta);
            const cos_theta = @cos(theta);

            const nx = cos_theta * sin_phi;
            const ny = cos_phi;
            const nz = sin_theta * sin_phi;

            const x = nx * radius;
            const y = ny * radius;
            const z = nz * radius;

            const idx = write_index.*;
            buf[idx + 0] = x;
            buf[idx + 1] = y;
            buf[idx + 2] = z;
            buf[idx + 3] = u;
            buf[idx + 4] = 1.0 - v;
            buf[idx + 5] = nx;
            buf[idx + 6] = ny;
            buf[idx + 7] = nz;
            write_index.* += 8;
        }
    }
}

fn pushSphereIndices(
    buf: []u32,
    write_index: *usize,
    stacks: u32,
    slices: u32,
) void {
    const stride = slices + 1;
    var i: u32 = 0;
    while (i < stacks) : (i += 1) {
        var j: u32 = 0;
        while (j < slices) : (j += 1) {
            const a: u32 = i * stride + j;
            const b: u32 = (i + 1) * stride + j;
            const c: u32 = (i + 1) * stride + (j + 1);
            const d: u32 = i * stride + (j + 1);

            const w = write_index.*;
            buf[w + 0] = a;
            buf[w + 1] = b;
            buf[w + 2] = c;
            buf[w + 3] = a;
            buf[w + 4] = c;
            buf[w + 5] = d;
            write_index.* += 6;
        }
    }
}

fn buildSphereMesh(key: []const u8, radius: f32) void {
    const stacks: u32 = 16;
    const slices: u32 = 24;

    const vert_count: usize = @as(usize, @intCast((stacks + 1) * (slices + 1)));
    const idx_count: usize = @as(usize, @intCast(stacks * slices * 6));

    var verts: [vert_count * 8]f32 = undefined;
    var inds: [idx_count]u32 = undefined;

    var vw: usize = 0;
    pushSphereVerts(&verts, &vw, radius, stacks, slices);

    var iw: usize = 0;
    pushSphereIndices(&inds, &iw, stacks, slices);

    _ = wasm96.graphics.meshCreate(key, verts[0..], inds[0..]);
}

fn buildPlaneMesh(key: []const u8, half_extent: f32, y: f32) void {
    // Flat quad; we'll render a procedural grid using simple line meshes laid on top.
    const vertices = [_]f32{
        // x, y, z,    u, v,   nx, ny, nz
        -half_extent, y, -half_extent, 0.0, 0.0, 0.0, 1.0, 0.0,
        half_extent,  y, -half_extent, 1.0, 0.0, 0.0, 1.0, 0.0,
        half_extent,  y, half_extent,  1.0, 1.0, 0.0, 1.0, 0.0,
        -half_extent, y, half_extent,  0.0, 1.0, 0.0, 1.0, 0.0,
    };

    const indices = [_]u32{
        0, 1, 2,
        0, 2, 3,
    };

    _ = wasm96.graphics.meshCreate(key, &vertices, &indices);
}

fn buildGridLinesMeshes(half_extent: f32, y: f32, step: f32) void {
    // Build two reusable thin-quad meshes:
    // - "grid_line_x": runs along X (varying X, constant Z)
    // - "grid_line_z": runs along Z (varying Z, constant X)
    //
    // We draw many instances with meshDraw translations to form a grid.
    const thickness: f32 = 0.03;

    // Line along X at z=0 (thin in Z)
    const vx = [_]f32{
        -half_extent, y, -thickness, 0.0, 0.0, 0.0, 1.0, 0.0,
        half_extent,  y, -thickness, 1.0, 0.0, 0.0, 1.0, 0.0,
        half_extent,  y, thickness,  1.0, 1.0, 0.0, 1.0, 0.0,
        -half_extent, y, thickness,  0.0, 1.0, 0.0, 1.0, 0.0,
    };
    const ix = [_]u32{ 0, 1, 2, 0, 2, 3 };
    _ = wasm96.graphics.meshCreate("grid_line_x", &vx, &ix);

    // Line along Z at x=0 (thin in X)
    const vz = [_]f32{
        -thickness, y, -half_extent, 0.0, 0.0, 0.0, 1.0, 0.0,
        thickness,  y, -half_extent, 1.0, 0.0, 0.0, 1.0, 0.0,
        thickness,  y, half_extent,  1.0, 1.0, 0.0, 1.0, 0.0,
        -thickness, y, half_extent,  0.0, 1.0, 0.0, 1.0, 0.0,
    };
    const iz = [_]u32{ 0, 1, 2, 0, 2, 3 };
    _ = wasm96.graphics.meshCreate("grid_line_z", &vz, &iz);

    _ = step;
}

// ---- Game loop ----

fn dtSeconds(now_ms: u64, last_ms: *u64) f32 {
    if (last_ms.* == 0) {
        last_ms.* = now_ms;
        return 1.0 / 60.0;
    }
    var dt_ms: u64 = now_ms - last_ms.*;
    last_ms.* = now_ms;

    if (dt_ms > 50) dt_ms = 50;

    return @as(f32, @floatFromInt(dt_ms)) / 1000.0;
}

fn updateGame(dt: f32) void {
    const gravity: f32 = -22.0;
    const accel: f32 = 28.0;
    const max_speed: f32 = 14.0;
    const ground_friction: f32 = 4.0;
    const brake_friction: f32 = 16.0;
    const air_friction: f32 = 0.6;
    const ground_y: f32 = 0.0;
    const radius: f32 = 0.6;

    const jump_speed: f32 = 9.0;
    if (wasm96.input.isButtonDown(0, .y) and g.on_ground) {
        g.vel.y = jump_speed;
        g.on_ground = false;
    }

    if (wasm96.input.isButtonDown(0, .start)) {
        g.reset();
        return;
    }

    // Camera control (requested mapping)
    // - L1/R1 = yaw
    // - X/Y  = pitch
    // - A/B  = roll
    const yaw_speed: f32 = 2.2;
    const pitch_speed: f32 = 1.6;
    const roll_speed: f32 = 2.0;

    const pitch_min: f32 = -1.2;
    const pitch_max: f32 = 1.2;

    if (wasm96.input.isButtonDown(0, .l1)) g.cam_yaw -= yaw_speed * dt;
    if (wasm96.input.isButtonDown(0, .r1)) g.cam_yaw += yaw_speed * dt;

    if (wasm96.input.isButtonDown(0, .x)) g.cam_pitch += pitch_speed * dt;
    if (wasm96.input.isButtonDown(0, .y)) g.cam_pitch -= pitch_speed * dt;

    if (wasm96.input.isButtonDown(0, .a)) g.cam_roll -= roll_speed * dt;
    if (wasm96.input.isButtonDown(0, .b)) g.cam_roll += roll_speed * dt;

    if (g.cam_pitch < pitch_min) g.cam_pitch = pitch_min;
    if (g.cam_pitch > pitch_max) g.cam_pitch = pitch_max;

    // Movement (requested): D-Pad moves the player relative to camera/viewport.
    // Up = forward, Down = backward, Left/Right = strafe
    const move_forward = wasm96.input.isButtonDown(0, .up);
    const move_back = wasm96.input.isButtonDown(0, .down);
    const move_left = wasm96.input.isButtonDown(0, .left);
    const move_right = wasm96.input.isButtonDown(0, .right);

    // Camera-relative movement:
    // The camera is positioned at:
    //   cam_pos = target + ( sin(yaw)*cos(pitch), sin(pitch), cos(yaw)*cos(pitch) ) * dist
    // so the camera looks back toward the target. That means "screen forward" (into the view)
    // corresponds to the *negative* yaw-forward direction in the XZ plane.
    const forward = Vec3{
        .x = -@sin(g.cam_yaw),
        .y = 0.0,
        .z = -@cos(g.cam_yaw),
    };
    const right = Vec3{
        .x = @cos(g.cam_yaw),
        .y = 0.0,
        .z = -@sin(g.cam_yaw),
    };

    var move: Vec3 = .{ .x = 0.0, .y = 0.0, .z = 0.0 };
    if (move_forward) move = move.add(forward);
    if (move_back) move = move.sub(forward);
    if (move_right) move = move.add(right);
    if (move_left) move = move.sub(right);

    const move_len = Vec3.len(move);
    if (move_len > 0.001) {
        const inv = 1.0 / move_len;
        move = move.mul(inv);

        g.vel.x += move.x * accel * dt;
        g.vel.z += move.z * accel * dt;
    }

    // Friction / damping:
    // - When you're on the ground and not pushing movement input, apply stronger friction
    //   so the sphere comes to rest.
    // - In the air, use light damping.
    const has_move_input = move_forward or move_back or move_left or move_right;
    const fric_base = if (g.on_ground) ground_friction else air_friction;
    const fric = if (g.on_ground and !has_move_input) brake_friction else fric_base;
    const decay = @exp(-fric * dt);
    g.vel.x *= decay;
    g.vel.z *= decay;

    const hv = Vec3.clampLen(Vec3{ .x = g.vel.x, .y = 0.0, .z = g.vel.z }, max_speed);
    g.vel.x = hv.x;
    g.vel.z = hv.z;

    g.vel.y += gravity * dt;

    g.pos = g.pos.add(g.vel.mul(dt));

    const min_y = ground_y + radius;
    if (g.pos.y <= min_y) {
        g.pos.y = min_y;
        if (g.vel.y < 0.0) g.vel.y = 0.0;
        g.on_ground = true;
    } else {
        g.on_ground = false;
    }
}

fn setupScene() void {
    wasm96.graphics.setSize(SCREEN_W, SCREEN_H);
    wasm96.graphics.set3d(true);

    _ = wasm96.graphics.fontRegisterSpleen("spleen", 12);

    buildSphereMesh("player_sphere", 0.6);

    // Ground: base plane + procedural grid lines.
    buildPlaneMesh("ground_plane", 80.0, 0.0);
    buildGridLinesMeshes(80.0, 0.01, 2.0);

    // Load and render the two bird OBJ models in this example directory.
    // These are small/lightweight and should be easier to validate than a huge scene.
    const bird1_bytes = @embedFile("12248_Bird_v1_L2.obj");
    const bird2_bytes = @embedFile("12249_Bird_v1_L2.obj");
    bird1_loaded = wasm96.graphics.meshCreateObj("bird_12248", bird1_bytes);
    bird2_loaded = wasm96.graphics.meshCreateObj("bird_12249", bird2_bytes);

    // Register textures and bind them to the OBJ meshes.
    //
    // The bird MTL files reference `*_diff.jpg` for map_Kd/map_Ka; the core doesn't
    // currently auto-load MTL, so we bind the diffuse texture manually.
    const bird1_diff_jpg = @embedFile("12248_Bird_v1_diff.jpg");
    const bird2_diff_jpg = @embedFile("12249_Bird_v1_diff.jpg");

    _ = wasm96.graphics.jpegRegister("tex_bird_12248_diff", bird1_diff_jpg);
    _ = wasm96.graphics.jpegRegister("tex_bird_12249_diff", bird2_diff_jpg);

    if (bird1_loaded) {
        _ = wasm96.graphics.meshSetTexture("bird_12248", "tex_bird_12248_diff");
    }
    if (bird2_loaded) {
        _ = wasm96.graphics.meshSetTexture("bird_12249", "tex_bird_12249_diff");
    }

    // 2D-only fallback HUD primitives:
    // Create a tiny 2D "pixel" mesh (a 1x1 quad) so we can draw visible HUD blocks
    // even if font rendering / overlay composition is broken in a given core build.
    // We draw it in 3D with the camera temporarily set to a fixed orthographic-like view
    // (implemented here as a close perspective with a large plane), so it doesn't depend
    // on the software framebuffer overlay at all.
    const hud_px_verts = [_]f32{
        // x, y, z,    u, v,   nx, ny, nz
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
        1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0,
        0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
    };
    const hud_px_inds = [_]u32{ 0, 1, 2, 0, 2, 3 };
    _ = wasm96.graphics.meshCreate("hud_px", &hud_px_verts, &hud_px_inds);

    g.reset();
}

export fn setup() void {
    setupScene();
}

export fn update() void {
    const now_ms = wasm96.system.millis();
    const dt = dtSeconds(now_ms, &g.last_ms);
    updateGame(dt);
}

export fn draw() void {
    wasm96.graphics.background(18, 18, 22);

    const target = Vec3{ .x = g.pos.x, .y = g.pos.y, .z = g.pos.z };

    const cy = @sin(g.cam_pitch);
    const cp = @cos(g.cam_pitch);
    const sx = @sin(g.cam_yaw);
    const cx = @cos(g.cam_yaw);

    const cam_pos = Vec3{
        .x = target.x + (sx * cp) * g.cam_dist,
        .y = target.y + cy * g.cam_dist + 2.0,
        .z = target.z + (cx * cp) * g.cam_dist,
    };

    wasm96.graphics.cameraPerspective(1.0, @as(f32, @floatFromInt(SCREEN_W)) / @as(f32, @floatFromInt(SCREEN_H)), 0.1, 400.0);

    // Apply camera roll by rolling the up-vector around the forward axis (camera -> target).
    const fwd = Vec3{
        .x = target.x - cam_pos.x,
        .y = target.y - cam_pos.y,
        .z = target.z - cam_pos.z,
    };
    const fwd_len = Vec3.len(fwd);
    const fwd_n = if (fwd_len > 0.0001) fwd.mul(1.0 / fwd_len) else Vec3{ .x = 0.0, .y = 0.0, .z = 1.0 };

    const base_up = Vec3{ .x = 0.0, .y = 1.0, .z = 0.0 };
    const c = @cos(g.cam_roll);
    const s = @sin(g.cam_roll);

    // Rodrigues' rotation formula (rotate base_up around fwd_n by cam_roll).
    // Use intermediate terms for clarity and to keep Zig parsing happy.
    const cross = Vec3{
        .x = fwd_n.y * base_up.z - fwd_n.z * base_up.y,
        .y = fwd_n.z * base_up.x - fwd_n.x * base_up.z,
        .z = fwd_n.x * base_up.y - fwd_n.y * base_up.x,
    };
    const dot = Vec3.dot(fwd_n, base_up);

    const up = Vec3.add(
        Vec3.add(base_up.mul(c), cross.mul(s)),
        fwd_n.mul(dot * (1.0 - c)),
    );

    wasm96.graphics.cameraLookAt(cam_pos.x, cam_pos.y, cam_pos.z, target.x, target.y, target.z, up.x, up.y, up.z);

    // Ground base
    wasm96.graphics.setColor(28, 30, 34, 255);
    wasm96.graphics.meshDraw("ground_plane", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);

    // Procedural grid lines
    // Minor lines every 2 units, major lines every 10 units, plus axis highlight.
    const half_extent: f32 = 80.0;
    const step: f32 = 2.0;

    var i: i32 = @as(i32, @intFromFloat(-half_extent / step));
    const i_max: i32 = @as(i32, @intFromFloat(half_extent / step));
    while (i <= i_max) : (i += 1) {
        const z = @as(f32, @floatFromInt(i)) * step;
        const major = (@rem(@abs(i), 5) == 0);

        if (z == 0.0) {
            wasm96.graphics.setColor(70, 120, 220, 255);
        } else if (major) {
            wasm96.graphics.setColor(55, 60, 70, 255);
        } else {
            wasm96.graphics.setColor(40, 44, 52, 255);
        }
        wasm96.graphics.meshDraw("grid_line_x", 0.0, 0.0, z, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    }

    var j: i32 = @as(i32, @intFromFloat(-half_extent / step));
    const j_max: i32 = @as(i32, @intFromFloat(half_extent / step));
    while (j <= j_max) : (j += 1) {
        const x = @as(f32, @floatFromInt(j)) * step;
        const major = (@rem(@abs(j), 5) == 0);

        if (x == 0.0) {
            wasm96.graphics.setColor(220, 90, 90, 255);
        } else if (major) {
            wasm96.graphics.setColor(55, 60, 70, 255);
        } else {
            wasm96.graphics.setColor(40, 44, 52, 255);
        }
        wasm96.graphics.meshDraw("grid_line_z", x, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    }

    // Birds (scene props)
    // If they load, place them near the origin so you can immediately see them while rolling.
    if (bird1_loaded) {
        wasm96.graphics.setColor(230, 230, 230, 255);
        // Birds were sideways / facing into the ground: rotate them upright.
        // Assumption: OBJ authored Z-up or different forward; adjust with +/-90° on X.
        wasm96.graphics.meshDraw("bird_12248", 4.0, 0.0, 6.0, -1.5707963, 0.8, 0.0, 0.06, 0.06, 0.06);
    }
    if (bird2_loaded) {
        wasm96.graphics.setColor(230, 230, 230, 255);
        wasm96.graphics.meshDraw("bird_12249", -5.0, 0.0, 7.0, -1.5707963, -0.6, 0.0, 0.06, 0.06, 0.06);
    }

    // Player sphere
    wasm96.graphics.setColor(230, 90, 120, 255);

    const vx = g.vel.x;
    const vz = g.vel.z;
    const speed = @sqrt(vx * vx + vz * vz);

    var roll_axis_x: f32 = 0.0;
    var roll_axis_z: f32 = 0.0;
    if (speed > 0.001) {
        roll_axis_x = -vz / speed;
        roll_axis_z = vx / speed;
    }

    const roll_angle = (g.pos.x * roll_axis_z + g.pos.z * -roll_axis_x) * 1.2;
    const rx = roll_axis_x * roll_angle;
    const rz = roll_axis_z * roll_angle;

    wasm96.graphics.meshDraw("player_sphere", g.pos.x, g.pos.y, g.pos.z, rx, 0.0, rz, 1.0, 1.0, 1.0);

    // HUD (primary path: 2D overlay text)
    wasm96.graphics.setColor(255, 255, 255, 255);
    wasm96.graphics.textKey(10, 10, "spleen", "Roll on Ground (DPad move, L1/R1 yaw, X/Y pitch, A/B roll, Y jump, Start reset)");
    if (!bird1_loaded or !bird2_loaded) {
        wasm96.graphics.setColor(255, 220, 140, 255);
        wasm96.graphics.textKey(10, 26, "spleen", "Note: birds not loaded (core may stub meshCreateObj)");
    }

    // HUD fallback: draw obvious 3D "status blocks" in front of the camera.
    // This doesn't rely on software framebuffer overlay composition.
    //
    // Legend:
    // - white bar: always drawn (proves draw loop is running)
    // - green square: bird1_loaded
    // - cyan square: bird2_loaded
    // - yellow square: font registered (we assume true if the font call returns true, but we don't store it; this is just a constant drawn bar)
    //
    // We place these relative to the camera by temporarily switching to a tiny near-field view.
    // This is intentionally simple: an anchored plane near the camera that should always be visible.
    wasm96.graphics.cameraPerspective(0.9, @as(f32, @floatFromInt(SCREEN_W)) / @as(f32, @floatFromInt(SCREEN_H)), 0.01, 50.0);
    wasm96.graphics.cameraLookAt(cam_pos.x, cam_pos.y, cam_pos.z, target.x, target.y, target.z, 0.0, 1.0, 0.0);

    // Put the blocks "in the air" near world origin so you can find them even if the camera logic is odd.
    // (If you can see the grid but not these, 3D mesh drawing itself is failing.)
    const hud_y: f32 = 3.5;
    const hud_z: f32 = -2.0;

    // Always-on bar
    wasm96.graphics.setColor(240, 240, 240, 255);
    wasm96.graphics.meshDraw("hud_px", -2.0, hud_y, hud_z, 0.0, 0.0, 0.0, 4.0, 0.18, 1.0);

    // Bird flags
    if (bird1_loaded) {
        wasm96.graphics.setColor(90, 220, 120, 255);
    } else {
        wasm96.graphics.setColor(70, 70, 70, 255);
    }
    wasm96.graphics.meshDraw("hud_px", -2.0, hud_y + 0.25, hud_z, 0.0, 0.0, 0.0, 0.25, 0.25, 1.0);

    if (bird2_loaded) {
        wasm96.graphics.setColor(80, 220, 220, 255);
    } else {
        wasm96.graphics.setColor(70, 70, 70, 255);
    }
    wasm96.graphics.meshDraw("hud_px", -1.65, hud_y + 0.25, hud_z, 0.0, 0.0, 0.0, 0.25, 0.25, 1.0);
}
