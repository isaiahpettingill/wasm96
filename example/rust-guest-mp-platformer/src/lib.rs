#![no_std]

// Simple 2P co-op platformer example for wasm96 (Immediate Mode).
//
// Controls (both ports 0 and 1):
// - D-pad Left/Right: move
// - A: jump
// - Start: respawn at checkpoint
//
// Goal:
// - Reach the flag together (both players inside the goal zone).
//
// Rust 2024 note:
// - `static mut` references are denied by default (`static_mut_refs`).
// - This example stores all mutable game state in a single `State` and only ever
//   accesses it by making a temporary *copy* of the state, mutating the copy,
//   and then writing it back. This avoids taking `&` or `&mut` references to
//   `static mut` variables.

use wasm96_sdk::prelude::*;

const W: i32 = 320;
const H: i32 = 240;

const GRAVITY: i32 = 1;
const MAX_FALL: i32 = 10;

const MOVE_SPEED: i32 = 2;
const JUMP_VEL: i32 = -9;

const PLAYER_W: i32 = 12;
const PLAYER_H: i32 = 16;

const MAX_PLATFORMS: usize = 8;

#[derive(Copy, Clone)]
struct RectI {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl RectI {
    const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    fn right(self) -> i32 {
        self.x + self.w
    }

    fn bottom(self) -> i32 {
        self.y + self.h
    }
}

fn intersects(a: RectI, b: RectI) -> bool {
    a.x < b.right() && a.right() > b.x && a.y < b.bottom() && a.bottom() > b.y
}

fn clamp_i32(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

#[derive(Copy, Clone)]
struct Player {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    on_ground: bool,
    last_jump_down: bool,
}

impl Player {
    const fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            vx: 0,
            vy: 0,
            on_ground: false,
            last_jump_down: false,
        }
    }

    fn rect(self) -> RectI {
        RectI::new(self.x, self.y, PLAYER_W, PLAYER_H)
    }
}

#[derive(Copy, Clone)]
enum GameState {
    Playing,
    Won { won_at_ms: u64 },
}

#[derive(Copy, Clone)]
struct State {
    game: GameState,
    p1: Player,
    p2: Player,
    checkpoint_x: i32,
    checkpoint_y: i32,
    camera_x: i32,
}

impl State {
    const fn new() -> Self {
        Self {
            game: GameState::Playing,
            p1: Player::new(20, 20),
            p2: Player::new(40, 20),
            checkpoint_x: 20,
            checkpoint_y: 20,
            camera_x: 0,
        }
    }
}

static mut STATE: State = State::new();

static PLATFORMS: [RectI; MAX_PLATFORMS] = [
    // floor
    RectI::new(0, 220, 900, 20),
    // small steps
    RectI::new(80, 192, 60, 8),
    RectI::new(160, 168, 60, 8),
    RectI::new(240, 144, 60, 8),
    // mid platforms
    RectI::new(360, 180, 90, 8),
    RectI::new(480, 150, 90, 8),
    // checkpoint ledge
    RectI::new(600, 190, 80, 8),
    // goal ledge (near flag)
    RectI::new(760, 170, 80, 8),
];

fn read_left(port: u32) -> bool {
    input::is_button_down(port, Button::Left)
}
fn read_right(port: u32) -> bool {
    input::is_button_down(port, Button::Right)
}
fn read_jump(port: u32) -> bool {
    input::is_button_down(port, Button::A)
}
fn read_respawn(port: u32) -> bool {
    input::is_button_down(port, Button::Start)
}

fn world_bounds() -> RectI {
    // A simple world box (x from 0..900, y from 0..240)
    RectI::new(0, 0, 900, H)
}

fn resolve_world_bounds(p: Player) -> Player {
    let mut p = p;
    let b = world_bounds();

    p.x = clamp_i32(p.x, b.x, b.right() - PLAYER_W);

    // If the player falls below the world, clamp and reset velocity.
    if p.y > b.bottom() + 60 {
        p.y = b.bottom() - PLAYER_H;
        p.vy = 0;
        p.on_ground = true;
    }

    p
}

fn move_and_collide(p: Player) -> Player {
    let mut p = p;

    // Horizontal move
    p.x += p.vx;
    let mut r = p.rect();
    for plat in PLATFORMS.iter().copied() {
        if intersects(r, plat) {
            if p.vx > 0 {
                // moving right, hit left side of platform
                p.x = plat.x - PLAYER_W;
            } else if p.vx < 0 {
                // moving left
                p.x = plat.right();
            }
            p.vx = 0;
            r = p.rect();
        }
    }

    // Vertical move
    p.y += p.vy;
    p.on_ground = false;
    r = p.rect();
    for plat in PLATFORMS.iter().copied() {
        if intersects(r, plat) {
            if p.vy > 0 {
                // falling: land on top
                p.y = plat.y - PLAYER_H;
                p.vy = 0;
                p.on_ground = true;
            } else if p.vy < 0 {
                // rising: bonk head
                p.y = plat.bottom();
                p.vy = 0;
            }
            r = p.rect();
        }
    }

    resolve_world_bounds(p)
}

fn apply_physics(p: Player) -> Player {
    let mut p = p;

    // gravity
    p.vy += GRAVITY;
    if p.vy > MAX_FALL {
        p.vy = MAX_FALL;
    }

    move_and_collide(p)
}

fn update_player(port: u32, p: Player) -> Player {
    let mut p = p;

    let l = read_left(port);
    let r = read_right(port);

    p.vx = if l && !r {
        -MOVE_SPEED
    } else if r && !l {
        MOVE_SPEED
    } else {
        0
    };

    let jump_down = read_jump(port);
    let jump_pressed = jump_down && !p.last_jump_down;

    if jump_pressed && p.on_ground {
        p.vy = JUMP_VEL;
        p.on_ground = false;
    }
    p.last_jump_down = jump_down;

    apply_physics(p)
}

fn midpoint_x(a: i32, b: i32) -> i32 {
    (a + b) / 2
}

fn goal_zone() -> RectI {
    // A small area near the "flag" that both players must be in
    RectI::new(820, 120, 30, 60)
}

fn checkpoint_zone() -> RectI {
    RectI::new(620, 150, 40, 60)
}

fn try_update_checkpoint(mut s: State) -> State {
    let cz = checkpoint_zone();
    if intersects(s.p1.rect(), cz) || intersects(s.p2.rect(), cz) {
        s.checkpoint_x = 610;
        s.checkpoint_y = 150;
    }
    s
}

fn try_win(mut s: State) -> State {
    let g = goal_zone();
    if matches!(s.game, GameState::Playing)
        && intersects(s.p1.rect(), g)
        && intersects(s.p2.rect(), g)
    {
        s.game = GameState::Won {
            won_at_ms: system::millis(),
        };
    }
    s
}

fn respawn_if_requested(mut s: State) -> State {
    let wants_respawn = read_respawn(0) || read_respawn(1);
    if !wants_respawn {
        return s;
    }

    let cx = s.checkpoint_x;
    let cy = s.checkpoint_y;

    let mut p1 = s.p1;
    p1.x = cx;
    p1.y = cy;
    p1.vx = 0;
    p1.vy = 0;
    p1.on_ground = false;

    let mut p2 = s.p2;
    p2.x = cx + 24;
    p2.y = cy;
    p2.vx = 0;
    p2.vy = 0;
    p2.on_ground = false;

    s.p1 = p1;
    s.p2 = p2;
    s.game = GameState::Playing;

    s
}

fn separate_players(mut s: State) -> State {
    // Simple co-op bump: prevent players from overlapping by separating on x-axis.
    let r1 = s.p1.rect();
    let r2 = s.p2.rect();
    if intersects(r1, r2) {
        if r1.x < r2.x {
            s.p1.x -= 1;
            s.p2.x += 1;
        } else {
            s.p1.x += 1;
            s.p2.x -= 1;
        }
        // Keep within bounds after separation
        s.p1 = resolve_world_bounds(s.p1);
        s.p2 = resolve_world_bounds(s.p2);
    }
    s
}

fn update_camera(mut s: State) -> State {
    let mid = midpoint_x(s.p1.x + PLAYER_W / 2, s.p2.x + PLAYER_W / 2);
    let target_cam = mid - (W / 2);
    let max_cam = world_bounds().right() - W;
    s.camera_x = clamp_i32(target_cam, 0, max_cam);
    s
}

fn draw_world(camera_x: i32, checkpoint_x: i32, checkpoint_y: i32) {
    // Background
    graphics::background(18, 18, 28);

    // Platforms
    graphics::set_color(90, 90, 110, 255);
    for plat in PLATFORMS.iter().copied() {
        graphics::rect(plat.x - camera_x, plat.y, plat.w as u32, plat.h as u32);
    }

    // Checkpoint marker
    graphics::set_color(80, 200, 255, 255);
    graphics::rect(checkpoint_x - camera_x, checkpoint_y - 10, 6, 10);

    // Flag + goal zone
    let g = goal_zone();
    graphics::set_color(255, 240, 80, 255);
    graphics::rect(g.x - camera_x + 10, g.y, 3, g.h as u32);
    graphics::set_color(255, 80, 80, 255);
    graphics::rect(g.x - camera_x + 13, g.y + 6, 12, 8);

    // Goal zone outline (for clarity)
    graphics::set_color(255, 255, 255, 80);
    graphics::rect_outline(g.x - camera_x, g.y, g.w as u32, g.h as u32);
}

fn draw_player(camera_x: i32, p: Player, r: u8, g: u8, b: u8) {
    graphics::set_color(r, g, b, 255);
    graphics::rect(p.x - camera_x, p.y, PLAYER_W as u32, PLAYER_H as u32);
    graphics::set_color(255, 255, 255, 130);
    graphics::rect_outline(p.x - camera_x, p.y, PLAYER_W as u32, PLAYER_H as u32);

    // Tiny "feet" indicator if on ground
    if p.on_ground {
        graphics::set_color(255, 255, 255, 80);
        graphics::line(
            p.x - camera_x + 2,
            p.y + PLAYER_H,
            p.x - camera_x + PLAYER_W - 2,
            p.y + PLAYER_H,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn setup() {
    graphics::set_size(W as u32, H as u32);
    audio::init(44100);

    unsafe {
        STATE = State::new();
    }

    system::log("wasm96: 2P co-op platformer example loaded");
}

#[unsafe(no_mangle)]
pub extern "C" fn update() {
    // Work on a local copy to avoid references to `static mut`.
    let mut s = unsafe { STATE };

    match s.game {
        GameState::Won { .. } => {
            // Keep players frozen (still allow respawn).
            s.p1.vx = 0;
            s.p1.vy = 0;
            s.p2.vx = 0;
            s.p2.vy = 0;

            s = respawn_if_requested(s);
        }
        GameState::Playing => {
            s.p1 = update_player(0, s.p1);
            s.p2 = update_player(1, s.p2);

            s = separate_players(s);
            s = try_update_checkpoint(s);
            s = try_win(s);
            s = respawn_if_requested(s);
        }
    }

    s = update_camera(s);

    unsafe {
        STATE = s;
    }

    // Libretro audio: push silence each frame.
    // 44100 Hz / 60 FPS = 735 samples per frame. Stereo => 1470 i16 samples.
    let silence = [0i16; 1470];
    audio::push_samples(&silence);
}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    // Read a copy of state (no references to `static mut`).
    let s = unsafe { STATE };

    draw_world(s.camera_x, s.checkpoint_x, s.checkpoint_y);

    draw_player(s.camera_x, s.p1, 120, 255, 120);
    draw_player(s.camera_x, s.p2, 120, 180, 255);

    // Simple HUD bars at top showing who's inside the goal zone.
    let g = goal_zone();
    let p1_in = intersects(s.p1.rect(), g);
    let p2_in = intersects(s.p2.rect(), g);

    graphics::set_color(255, 255, 255, 80);
    graphics::rect_outline(6, 6, 100, 10);
    graphics::rect_outline(6, 20, 100, 10);

    graphics::set_color(
        if p1_in { 120 } else { 40 },
        if p1_in { 255 } else { 40 },
        60,
        200,
    );
    graphics::rect(7, 7, 98, 8);

    graphics::set_color(
        60,
        if p2_in { 200 } else { 40 },
        if p2_in { 255 } else { 40 },
        200,
    );
    graphics::rect(7, 21, 98, 8);

    // Win overlay
    if let GameState::Won { won_at_ms } = s.game {
        let now = system::millis();
        let t = (now - won_at_ms) as i32;

        // flash a banner for a bit
        let alpha = if (t / 250) % 2 == 0 { 200 } else { 90 };
        graphics::set_color(0, 0, 0, alpha as u8);
        graphics::rect(0, 90, W as u32, 60);

        graphics::set_color(255, 255, 255, 200);
        graphics::rect_outline(10, 95, (W - 20) as u32, 50);

        // No text API available; show a "celebration" pattern.
        graphics::set_color(255, 240, 80, 220);
        for i in 0..10 {
            let x = 30 + i * 26;
            graphics::circle(x, 120, 4);
        }
    }
}
