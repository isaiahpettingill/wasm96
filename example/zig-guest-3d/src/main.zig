const std = @import("std");
const wasm96 = @import("wasm96");

/// Simple rolling-sphere game:
/// - You are a sphere rolling on a plane.
/// - A static OBJ environment is rendered for decoration (no collision against it yet).
///
/// Controls (joypad port 0):
/// - DPad: camera pitch
/// - L1/R1: camera yaw
/// - A: accelerate forward (relative to camera yaw)
/// - B: brake
/// - Y: jump
///
/// Notes:
/// - This is intentionally physics-light (semi-implicit Euler + simple ground collision).
/// - wasm96 core currently has `meshCreateObj` stubbed (returns 0) in some versions; if so,
///   you'll still get the plane + sphere, but not the OBJ environment.
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

    fn norm(a: Vec3) Vec3 {
        const l = a.len();
        if (l <= 0.000001) return .{ .x = 0, .y = 0, .z = 0 };
        return a.mul(1.0 / l);
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
    cam_dist: f32 = 12.0,

    // Accumulator
    last_ms: u64 = 0,

    fn reset(self: *Game) void {
        self.pos = .{ .x = 0.0, .y = 1.0, .z = 0.0 };
        self.vel = .{ .x = 0.0, .y = 0.0, .z = 0.0 };
        self.on_ground = false;
        self.cam_yaw = 0.0;
        self.cam_pitch = 0.6;
        self.cam_dist = 12.0;
    }
};

var g: Game = .{};
var obj_loaded: bool = false;

// ---- Geometry helpers ----

fn pushSphereVerts(
    buf: []f32,
    write_index: *usize,
    radius: f32,
    stacks: u32,
    slices: u32,
) void {
    // Vertex layout: [x,y,z, u,v, nx,ny,nz]
    // We build a sphere as (stacks+1)*(slices+1) vertices with UV.
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
    // Two triangles per quad on the sphere grid.
    const stride = slices + 1;
    var i: u32 = 0;
    while (i < stacks) : (i += 1) {
        var j: u32 = 0;
        while (j < slices) : (j += 1) {
            const a: u32 = i * stride + j;
            const b: u32 = (i + 1) * stride + j;
            const c: u32 = (i + 1) * stride + (j + 1);
            const d: u32 = i * stride + (j + 1);

            // (a,b,c) and (a,c,d)
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
    // Simple quad made of 2 triangles, normals up, UV tiled.
    const tile: f32 = 20.0;
    const vertices = [_]f32{
        // x, y, z,    u, v,   nx, ny, nz
        -half_extent, y, -half_extent, 0.0,  0.0,  0.0, 1.0, 0.0,
        half_extent,  y, -half_extent, tile, 0.0,  0.0, 1.0, 0.0,
        half_extent,  y, half_extent,  tile, tile, 0.0, 1.0, 0.0,
        -half_extent, y, half_extent,  0.0,  tile, 0.0, 1.0, 0.0,
    };

    const indices = [_]u32{
        0, 1, 2,
        0, 2, 3,
    };

    _ = wasm96.graphics.meshCreate(key, &vertices, &indices);
}

// ---- Game loop ----

fn dtSeconds(now_ms: u64, last_ms: *u64) f32 {
    if (last_ms.* == 0) {
        last_ms.* = now_ms;
        return 1.0 / 60.0;
    }
    var dt_ms: u64 = now_ms - last_ms.*;
    last_ms.* = now_ms;

    // Clamp to avoid huge steps on pauses.
    if (dt_ms > 50) dt_ms = 50;

    return @as(f32, @floatFromInt(dt_ms)) / 1000.0;
}

fn updateGame(dt: f32) void {
    // Tunables (not physically accurate; "feel" based)
    const gravity: f32 = -22.0;
    const accel: f32 = 28.0;
    const max_speed: f32 = 14.0;
    const ground_friction: f32 = 4.0;
    const brake_friction: f32 = 16.0;
    const air_friction: f32 = 0.6;
    const ground_y: f32 = 0.0;
    const radius: f32 = 0.6;

    // Jump (Y)
    const jump_speed: f32 = 9.0;
    if (wasm96.input.isButtonDown(0, .y) and g.on_ground) {
        g.vel.y = jump_speed;
        g.on_ground = false;
    }

    // Reset (Start)
    if (wasm96.input.isButtonDown(0, .start)) {
        g.reset();
        return;
    }

    // Camera control
    const yaw_speed: f32 = 2.2; // radians/sec
    const pitch_speed: f32 = 1.6; // radians/sec
    const pitch_min: f32 = 0.2;
    const pitch_max: f32 = 1.2;

    // L1/R1 yaw
    if (wasm96.input.isButtonDown(0, .l1)) g.cam_yaw -= yaw_speed * dt;
    if (wasm96.input.isButtonDown(0, .r1)) g.cam_yaw += yaw_speed * dt;

    // D-pad pitch
    if (wasm96.input.isButtonDown(0, .up)) g.cam_pitch += pitch_speed * dt;
    if (wasm96.input.isButtonDown(0, .down)) g.cam_pitch -= pitch_speed * dt;

    if (g.cam_pitch < pitch_min) g.cam_pitch = pitch_min;
    if (g.cam_pitch > pitch_max) g.cam_pitch = pitch_max;

    // Throttle/brake
    const accelerate = wasm96.input.isButtonDown(0, .a);
    const braking = wasm96.input.isButtonDown(0, .b);

    // Forward direction relative to camera yaw (world XZ)
    const forward = Vec3{
        .x = @sin(g.cam_yaw),
        .y = 0.0,
        .z = @cos(g.cam_yaw),
    };

    // Apply forward acceleration (only when holding A)
    if (accelerate) {
        g.vel.x += forward.x * accel * dt;
        g.vel.z += forward.z * accel * dt;
    }

    // Friction / braking (different on ground vs air)
    const fric_base = if (g.on_ground) ground_friction else air_friction;
    const fric = if (braking and g.on_ground) brake_friction else fric_base;
    const decay = @exp(-fric * dt);
    g.vel.x *= decay;
    g.vel.z *= decay;

    // Clamp horizontal speed (fix Zig parsing by avoiding method-call syntax here)
    const hv = Vec3.clampLen(Vec3{ .x = g.vel.x, .y = 0.0, .z = g.vel.z }, max_speed);
    g.vel.x = hv.x;
    g.vel.z = hv.z;

    // Gravity
    g.vel.y += gravity * dt;

    // Integrate
    g.pos = g.pos.add(g.vel.mul(dt));

    // Ground collision: keep sphere resting on plane at y = ground_y + radius
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

    // Meshes
    buildSphereMesh("player_sphere", 0.6);
    buildPlaneMesh("ground_plane", 80.0, 0.0);

    // Register and bind a texture for the ground plane so 3D texturing is exercised.
    // Prefer a PNG (RGBA / alpha respected) if available; fallback to a JPEG (opaque).
    // NOTE: The 3D pipeline uses keyed images (pngRegister/jpegRegister) + meshSetTexture.
    const ground_tex_key = "ground_tex";

    // Try PNG first (there is at least one PNG in the assets set).
    const ground_png = @embedFile("Textures/Flag.png");
    if (wasm96.graphics.pngRegister(ground_tex_key, ground_png)) {
        _ = wasm96.graphics.meshSetTexture("ground_plane", ground_tex_key);
    } else {
        // Fallback to a JPEG if PNG registration fails for any reason.
        const ground_jpg = @embedFile("Textures/Concrete_Sidewalk.jpg");
        if (wasm96.graphics.jpegRegister(ground_tex_key, ground_jpg)) {
            _ = wasm96.graphics.meshSetTexture("ground_plane", ground_tex_key);
        }
    }

    // Try to load OBJ environment for visuals.
    // If the core doesn't implement OBJ yet, this will return false.
    // Embed the OBJ bytes at compile time.
    const obj_bytes = @embedFile("Castelia City.obj");
    obj_loaded = wasm96.graphics.meshCreateObj("environment_obj", obj_bytes);

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
    // 2D buffer background only affects overlay; the core's 3D clear may differ.
    // Still, we set a background and then draw geometry.
    wasm96.graphics.background(18, 18, 22);

    // Camera: orbit behind player based on yaw/pitch.
    const ty = g.pos.y;
    const target = Vec3{ .x = g.pos.x, .y = ty, .z = g.pos.z };

    const cy = @sin(g.cam_pitch);
    const cp = @cos(g.cam_pitch);
    const sx = @sin(g.cam_yaw);
    const cx = @cos(g.cam_yaw);

    // Spherical-ish placement: behind the target.
    const cam_pos = Vec3{
        .x = target.x + (sx * cp) * g.cam_dist,
        .y = target.y + cy * g.cam_dist + 2.0,
        .z = target.z + (cx * cp) * g.cam_dist,
    };

    wasm96.graphics.cameraPerspective(1.0, @as(f32, @floatFromInt(SCREEN_W)) / @as(f32, @floatFromInt(SCREEN_H)), 0.1, 400.0);
    wasm96.graphics.cameraLookAt(cam_pos.x, cam_pos.y, cam_pos.z, target.x, target.y, target.z, 0.0, 1.0, 0.0);

    // Lighting color is driven by current 2D draw color in the core shader.
    // We'll set colors per draw call.

    // Draw environment (decor)
    if (obj_loaded) {
        wasm96.graphics.setColor(200, 200, 200, 255);
        // The OBJ uses inch-ish units and huge coordinates; scale it down aggressively.
        // We also translate it so the player starts near the origin.
        wasm96.graphics.meshDraw("environment_obj", 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.00008, 0.00008, 0.00008);
    }

    // Draw ground plane (play surface)
    wasm96.graphics.setColor(70, 80, 90, 255);
    wasm96.graphics.meshDraw("ground_plane", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);

    // Draw player sphere
    wasm96.graphics.setColor(230, 90, 120, 255);

    // Fake roll: rotate around axis perpendicular to velocity for visual rolling.
    // This doesn't affect physics, only rendering.
    const vx = g.vel.x;
    const vz = g.vel.z;
    const speed = @sqrt(vx * vx + vz * vz);

    var roll_axis_x: f32 = 0.0;
    var roll_axis_z: f32 = 0.0;
    if (speed > 0.001) {
        // If moving in +Z, roll around +X, etc.
        roll_axis_x = -vz / speed;
        roll_axis_z = vx / speed;
    }

    // A tiny accumulated roll angle derived from position distance traveled.
    // (Not perfect; works well enough visually.)
    const roll_angle = (g.pos.x * roll_axis_z + g.pos.z * -roll_axis_x) * 1.2;

    // We only have Euler angles API; approximate by mapping axis to Euler.
    // If moving mostly forward/back, use X rotation; if mostly sideways, use Z rotation.
    const rx = roll_axis_x * roll_angle;
    const rz = roll_axis_z * roll_angle;

    wasm96.graphics.meshDraw("player_sphere", g.pos.x, g.pos.y, g.pos.z, rx, 0.0, rz, 1.0, 1.0, 1.0);

    // HUD (2D overlay)
    wasm96.graphics.setColor(255, 255, 255, 255);
    wasm96.graphics.textKey(10, 10, "spleen", "Rolling Sphere (DPad pitch, L1/R1 yaw, A accel, B brake, Y jump)");
    if (!obj_loaded) {
        wasm96.graphics.setColor(255, 220, 140, 255);
        wasm96.graphics.textKey(10, 26, "spleen", "Note: OBJ environment not loaded (core may stub meshCreateObj)");
    }
}
