use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use wasm96_sdk::prelude::*;

const SCREEN_W: u32 = 320;
const SCREEN_H: u32 = 240;
const TILE_SIZE: i32 = 14;
const CHUNK_SIZE: i32 = 32;

const PLAYER_W: f32 = 10.0;
const PLAYER_H: f32 = 14.0;

const FONT_KEY_SPLEEN_16: &str = "font/spleen/16";

const DWARF_IDLE: &[u8] = include_bytes!("../assets/dwarf/Idle.aseprite");
const DWARF_MOVE: &[u8] = include_bytes!("../assets/dwarf/Moving.aseprite");
const DWARF_SWING: &[u8] = include_bytes!("../assets/dwarf/Swing.aseprite");
const DWARF_SHOVEL: &[u8] = include_bytes!("../assets/dwarf/Shoveling.aseprite");
const DWARF_MUSIC_QOA: &[u8] = include_bytes!("../assets/dwarf.qoa");

const DWARF_IDLE_KEY: &str = "dwarf/idle";
const DWARF_MOVE_KEY: &str = "dwarf/move";
const DWARF_SWING_KEY: &str = "dwarf/swing";
const DWARF_SHOVEL_KEY: &str = "dwarf/shovel";
const DWARF_DEFAULT_TAG: &str = "";

const ORE_BASE: u16 = 100;

const ITEM_COUNT: usize = 6;
const ITEM_NAMES: [&str; ITEM_COUNT] = ["Copper", "Iron", "Gold", "Ruby", "Emerald", "Sapphire"];

static GAME_STATE: LazyLock<Mutex<GameState>> = LazyLock::new(|| Mutex::new(GameState::new()));

#[derive(Clone)]
struct Player {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    facing: i32,
    on_ground: bool,
}

#[derive(Clone)]
struct TileCell {
    tile_id: u16,
}

#[derive(Clone)]
struct Chunk {
    tiles: Vec<TileCell>,
}

#[derive(Clone)]
struct World {
    seed: u32,
    chunks: HashMap<(i32, i32), Chunk>,
}

struct GameState {
    initialized: bool,
    world: World,
    player: Player,
    inventory: [u32; ITEM_COUNT],
    mining_timer_ms: u32,
    mining_target: Option<(i32, i32)>,
    last_ms: u64,
}

impl GameState {
    fn new() -> Self {
        Self {
            initialized: false,
            world: World::new(0),
            player: Player {
                x: (SCREEN_W as f32 - PLAYER_W) * 0.5,
                y: (SCREEN_H as f32 - PLAYER_H) * 0.5,
                vx: 0.0,
                vy: 0.0,
                facing: 1,
                on_ground: false,
            },
            inventory: [0; ITEM_COUNT],
            mining_timer_ms: 0,
            mining_target: None,
            last_ms: 0,
        }
    }
}

impl World {
    fn new(seed: u32) -> Self {
        Self {
            seed,
            chunks: HashMap::new(),
        }
    }

    fn get_tile_id(&mut self, tx: i32, ty: i32) -> u16 {
        let (cx, cy, lx, ly) = chunk_coords(tx, ty);
        let chunk = self.chunks.entry((cx, cy)).or_insert_with(|| {
            let mut tiles = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
            for local_y in 0..CHUNK_SIZE {
                for local_x in 0..CHUNK_SIZE {
                    let world_x = cx * CHUNK_SIZE + local_x;
                    let world_y = cy * CHUNK_SIZE + local_y;
                    let tile_id = generate_tile(self.seed, world_x, world_y);
                    tiles.push(TileCell { tile_id });
                }
            }
            Chunk { tiles }
        });

        let index = (ly * CHUNK_SIZE + lx) as usize;
        chunk.tiles[index].tile_id
    }

    fn set_tile_id(&mut self, tx: i32, ty: i32, tile_id: u16) {
        let (cx, cy, lx, ly) = chunk_coords(tx, ty);
        let chunk = self.chunks.entry((cx, cy)).or_insert_with(|| {
            let mut tiles = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
            for local_y in 0..CHUNK_SIZE {
                for local_x in 0..CHUNK_SIZE {
                    let world_x = cx * CHUNK_SIZE + local_x;
                    let world_y = cy * CHUNK_SIZE + local_y;
                    let tile = generate_tile(self.seed, world_x, world_y);
                    tiles.push(TileCell { tile_id: tile });
                }
            }
            Chunk { tiles }
        });

        let index = (ly * CHUNK_SIZE + lx) as usize;
        chunk.tiles[index].tile_id = tile_id;
    }

    fn mine_tile(&mut self, tx: i32, ty: i32) -> u16 {
        let id = self.get_tile_id(tx, ty);
        if id == 0 {
            return 0;
        }
        self.set_tile_id(tx, ty, 0);
        id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn setup() {
    graphics::set_size(SCREEN_W, SCREEN_H);
    graphics::font_register_spleen(FONT_KEY_SPLEEN_16, 16);

    let mut state = GAME_STATE.lock().unwrap();
    if state.initialized {
        return;
    }

    let seed = system::millis() as u32 ^ 0x9E3779B9;
    state.world = World::new(seed);

    state.player.x = 0.0;
    // Find surface at x=0
    let h_noise = noise2d(seed ^ 0x1234, 0.0, 0.0);
    let surface_ty = 10 + (h_noise * 12.0) as i32;
    state.player.y = (surface_ty as f32 - 2.0) * TILE_SIZE as f32;

    state.player.vx = 0.0;
    state.player.vy = 0.0;
    state.player.on_ground = false;

    let center_tx = (state.player.x / TILE_SIZE as f32).floor() as i32;
    let center_ty = (state.player.y / TILE_SIZE as f32).floor() as i32;
    // Clear small area around spawn
    for ty in (center_ty - 1)..=(center_ty + 2) {
        for tx in (center_tx - 1)..=(center_tx + 1) {
            state.world.set_tile_id(tx, ty, 0);
        }
    }

    graphics::aseprite_register(DWARF_IDLE_KEY, DWARF_IDLE);
    graphics::aseprite_register(DWARF_MOVE_KEY, DWARF_MOVE);
    graphics::aseprite_register(DWARF_SWING_KEY, DWARF_SWING);
    graphics::aseprite_register(DWARF_SHOVEL_KEY, DWARF_SHOVEL);

    audio::init(44_100);
    audio::play_qoa(DWARF_MUSIC_QOA);

    state.last_ms = system::millis();
    state.initialized = true;
}

#[unsafe(no_mangle)]
pub extern "C" fn update() {
    let now = system::millis();
    let mut state = GAME_STATE.lock().unwrap();
    if !state.initialized {
        return;
    }

    let dt_ms = (now - state.last_ms).min(100) as u32;
    state.last_ms = now;

    // Better physics: Horizontal movement with acceleration and friction
    let mut move_dir = 0;
    if input::is_button_down(0, Button::Left) {
        move_dir -= 1;
    }
    if input::is_button_down(0, Button::Right) {
        move_dir += 1;
    }

    let accel = 0.4;
    let friction = 0.85;
    let max_speed = 2.5;

    if move_dir != 0 {
        state.player.vx += move_dir as f32 * accel;
        state.player.facing = move_dir;
    } else {
        state.player.vx *= friction;
    }
    state.player.vx = state.player.vx.clamp(-max_speed, max_speed);

    let gravity = 0.25;
    state.player.vy += gravity;
    if state.player.vy > 6.0 {
        state.player.vy = 6.0;
    }

    // Sub-pixel collision resolution
    let steps = 4;
    for _ in 0..steps {
        let (px, py, vx, _) = (
            state.player.x,
            state.player.y,
            state.player.vx,
            state.player.vy,
        );
        let dx = vx / steps as f32;
        let new_x = px + dx;
        if !collides(&mut state.world, new_x, py, PLAYER_W, PLAYER_H) {
            state.player.x = new_x;
        } else {
            state.player.vx = 0.0;
        }

        let (px, py, _, vy) = (
            state.player.x,
            state.player.y,
            state.player.vx,
            state.player.vy,
        );
        let dy = vy / steps as f32;
        let new_y = py + dy;
        if !collides(&mut state.world, px, new_y, PLAYER_W, PLAYER_H) {
            state.player.y = new_y;
        } else {
            if vy > 0.0 {
                state.player.on_ground = true;
            }
            state.player.vy = 0.0;
        }
    }

    let cur_x = state.player.x;
    let cur_y = state.player.y;
    state.player.on_ground = collides(&mut state.world, cur_x, cur_y + 1.0, PLAYER_W, PLAYER_H);

    if state.player.on_ground && input::is_button_down(0, Button::B) {
        state.player.vy = -4.5;
        state.player.on_ground = false;
    }

    if state.mining_timer_ms > 0 {
        state.mining_timer_ms = state.mining_timer_ms.saturating_sub(dt_ms);
    }

    // Determine mining direction from D-pad
    let mut attack_dx = state.player.facing;
    let mut attack_dy = 0;
    if input::is_button_down(0, Button::Up) {
        attack_dy = -1;
        attack_dx = 0;
    } else if input::is_button_down(0, Button::Down) {
        attack_dy = 1;
        attack_dx = 0;
    }

    let mining_input = input::is_button_down(0, Button::A) || input::is_mouse_down(0);
    if mining_input && state.mining_timer_ms == 0 {
        let target_tx;
        let target_ty;

        if input::is_mouse_down(0) {
            let (cam_x, cam_y) = camera_origin(&state.player);
            let mx = input::get_mouse_x() as f32 + cam_x;
            let my = input::get_mouse_y() as f32 + cam_y;
            target_tx = (mx / TILE_SIZE as f32).floor() as i32;
            target_ty = (my / TILE_SIZE as f32).floor() as i32;
        } else {
            let (px, py) = (state.player.x, state.player.y);
            let offset_x = attack_dx as f32 * TILE_SIZE as f32 * 1.1;
            let offset_y = if attack_dy != 0 {
                attack_dy as f32 * TILE_SIZE as f32 * 1.1
            } else {
                0.0
            };
            let wx = px + PLAYER_W * 0.5 + offset_x;
            let wy = py + PLAYER_H * 0.5 + offset_y;
            target_tx = (wx / TILE_SIZE as f32).floor() as i32;
            target_ty = (wy / TILE_SIZE as f32).floor() as i32;
        }

        let tile_id = state.world.get_tile_id(target_tx, target_ty);
        if tile_id != 0 {
            let (px, py) = (state.player.x, state.player.y);
            let tx_center = target_tx * TILE_SIZE + TILE_SIZE / 2;
            let ty_center = target_ty * TILE_SIZE + TILE_SIZE / 2;
            let px_center = (px + PLAYER_W * 0.5) as i32;
            let py_center = (py + PLAYER_H * 0.5) as i32;

            if (tx_center - px_center).abs() < TILE_SIZE * 4
                && (ty_center - py_center).abs() < TILE_SIZE * 4
            {
                state.mining_target = Some((target_tx, target_ty));
                let is_dirt = tile_id == 1;
                state.mining_timer_ms = if is_dirt { 80 } else { 150 };

                let mined = state.world.mine_tile(target_tx, target_ty);
                if let Some(item_index) = item_for_tile(mined) {
                    state.inventory[item_index] = state.inventory[item_index].saturating_add(1);
                }
            }
        } else if !input::is_mouse_down(0) {
            // Auto-target block in direction of attack if primary target is air
            for d in 1..3 {
                let d_tx = target_tx + attack_dx * d;
                let d_ty = target_ty + attack_dy * d;
                let tid = state.world.get_tile_id(d_tx, d_ty);
                if tid != 0 {
                    let (px, py) = (state.player.x, state.player.y);
                    let tx_center = d_tx * TILE_SIZE + TILE_SIZE / 2;
                    let ty_center = d_ty * TILE_SIZE + TILE_SIZE / 2;
                    let px_center = (px + PLAYER_W * 0.5) as i32;
                    let py_center = (py + PLAYER_H * 0.5) as i32;
                    if (tx_center - px_center).abs() < TILE_SIZE * 4
                        && (ty_center - py_center).abs() < TILE_SIZE * 4
                    {
                        state.mining_target = Some((d_tx, d_ty));
                        state.mining_timer_ms = if tid == 1 { 80 } else { 150 };
                        let mined = state.world.mine_tile(d_tx, d_ty);
                        if let Some(item_index) = item_for_tile(mined) {
                            state.inventory[item_index] =
                                state.inventory[item_index].saturating_add(1);
                        }
                        break;
                    }
                }
            }
        }
    }

    if state.mining_timer_ms == 0 {
        state.mining_target = None;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    let mut state = GAME_STATE.lock().unwrap();
    if !state.initialized {
        return;
    }

    graphics::background(10, 12, 18);
    graphics::no_stroke();

    let (cam_x, cam_y) = camera_origin(&state.player);

    draw_tiles(&mut state, cam_x, cam_y);

    draw_player(&mut state, cam_x, cam_y);

    draw_hud(&state);

    draw_mouse_crosshair(cam_x, cam_y);
}

fn camera_origin(player: &Player) -> (f32, f32) {
    let cam_x = player.x - (SCREEN_W as f32) * 0.5 + PLAYER_W * 0.5;
    let cam_y = player.y - (SCREEN_H as f32) * 0.5 + PLAYER_H * 0.5;
    (cam_x, cam_y)
}

fn draw_tiles(state: &mut GameState, cam_x: f32, cam_y: f32) {
    let start_tx = div_floor(cam_x.floor() as i32, TILE_SIZE) - 2;
    let start_ty = div_floor(cam_y.floor() as i32, TILE_SIZE) - 2;
    let end_tx = div_floor((cam_x + SCREEN_W as f32).floor() as i32, TILE_SIZE) + 2;
    let end_ty = div_floor((cam_y + SCREEN_H as f32).floor() as i32, TILE_SIZE) + 2;

    for ty in start_ty..=end_ty {
        for tx in start_tx..=end_tx {
            let tile_id = state.world.get_tile_id(tx, ty);
            if tile_id == 0 {
                continue;
            }
            let sx = tx * TILE_SIZE - cam_x as i32;
            let sy = ty * TILE_SIZE - cam_y as i32;

            match tile_id {
                1 => {
                    // Dirt
                    graphics::fill(101, 67, 33, 255);
                    graphics::rect(sx, sy, TILE_SIZE as u32, TILE_SIZE as u32);
                    graphics::fill(80, 50, 20, 255);
                    graphics::rect(sx, sy + TILE_SIZE - 2, TILE_SIZE as u32, 2);
                }
                2 => {
                    // Stone
                    graphics::fill(100, 100, 100, 255);
                    graphics::rect(sx, sy, TILE_SIZE as u32, TILE_SIZE as u32);
                }
                id if id >= ORE_BASE => {
                    let ore_idx = (id - ORE_BASE) as usize;
                    graphics::fill(100, 100, 100, 255);
                    graphics::rect(sx, sy, TILE_SIZE as u32, TILE_SIZE as u32);
                    // Draw ore "spec" as a circle or diamond
                    let color = match ore_idx {
                        0 => (255, 215, 0),   // Gold
                        1 => (192, 192, 192), // Silver/Iron
                        2 => (0, 255, 255),   // Diamond/Cyan
                        3 => (255, 0, 0),     // Ruby/Red
                        _ => (255, 255, 255),
                    };
                    graphics::fill(color.0, color.1, color.2, 255);
                    graphics::circle(
                        sx + TILE_SIZE / 2,
                        sy + TILE_SIZE / 2,
                        (TILE_SIZE / 4) as u32,
                    );
                }
                _ => {}
            }
        }
    }
}

fn draw_player(state: &mut GameState, cam_x: f32, cam_y: f32) {
    let (px, py, vx, facing) = (
        state.player.x,
        state.player.y,
        state.player.vx,
        state.player.facing,
    );
    let mining_timer = state.mining_timer_ms;
    let mining_target = state.mining_target;

    let moving = vx.abs() > 0.1;
    let mining = mining_timer > 0;

    let mut anim_key = if moving {
        DWARF_MOVE_KEY
    } else {
        DWARF_IDLE_KEY
    };

    if mining {
        let (is_dirt, is_down) = if let Some((tx, ty)) = mining_target {
            let tile_id = state.world.get_tile_id(tx, ty);
            let player_ty = (py / TILE_SIZE as f32).floor() as i32;
            (tile_id == 1, ty > player_ty)
        } else {
            (false, false)
        };
        anim_key = if is_dirt || is_down {
            DWARF_SHOVEL_KEY
        } else {
            DWARF_SWING_KEY
        };
    }

    let sx = (px - cam_x) as i32;
    let sy = (py - cam_y) as i32;

    graphics::push_matrix();
    // Translate to the center of the player's collision box
    graphics::translate(sx as f32 + PLAYER_W * 0.5, sy as f32 + PLAYER_H * 0.5, 0.0);
    if facing < 0 {
        graphics::scale(-1.0, 1.0, 1.0);
    }
    // Draw the dwarf sprite centered relative to the pivot.
    // Assuming a standard 32x32 Aseprite frame, we offset by -16 to center it.
    graphics::aseprite_play_key(anim_key, -16, -16, DWARF_DEFAULT_TAG);
    graphics::pop_matrix();
}

fn draw_hud(state: &GameState) {
    graphics::set_color(240, 240, 255, 255);
    graphics::text_key(10, 10, FONT_KEY_SPLEEN_16, "Dwarf Mining");

    let mut y = 30;
    for (i, name) in ITEM_NAMES.iter().enumerate() {
        let count = state.inventory[i];
        if count == 0 {
            continue;
        }
        let line = format!("{name}: {count}");
        graphics::text_key(10, y, FONT_KEY_SPLEEN_16, &line);
        y += 16;
    }

    let help = "D-Pad: move | B: jump | A/Mouse: mine";
    graphics::text_key(10, SCREEN_H as i32 - 14, FONT_KEY_SPLEEN_16, help);
}

fn draw_mouse_crosshair(cam_x: f32, cam_y: f32) {
    let mx = input::get_mouse_x();
    let my = input::get_mouse_y();
    let wx = cam_x + mx as f32;
    let wy = cam_y + my as f32;

    let tx = (wx / TILE_SIZE as f32).floor() as i32;
    let ty = (wy / TILE_SIZE as f32).floor() as i32;

    let sx = tx * TILE_SIZE - cam_x as i32;
    let sy = ty * TILE_SIZE - cam_y as i32;

    graphics::set_color(255, 255, 255, 120);
    graphics::rect_outline(sx, sy, TILE_SIZE as u32, TILE_SIZE as u32);
}

fn collides(world: &mut World, x: f32, y: f32, w: f32, h: f32) -> bool {
    let left = (x / TILE_SIZE as f32).floor() as i32;
    let right = ((x + w - 1.0) / TILE_SIZE as f32).floor() as i32;
    let top = (y / TILE_SIZE as f32).floor() as i32;
    let bottom = ((y + h - 1.0) / TILE_SIZE as f32).floor() as i32;

    for ty in top..=bottom {
        for tx in left..=right {
            if world.get_tile_id(tx, ty) != 0 {
                return true;
            }
        }
    }

    false
}

fn item_for_tile(tile_id: u16) -> Option<usize> {
    if tile_id >= ORE_BASE {
        let ore_idx = (tile_id - ORE_BASE) as usize;
        if ore_idx < ITEM_COUNT {
            return Some(ore_idx);
        }
    }
    None
}

fn chunk_coords(tx: i32, ty: i32) -> (i32, i32, i32, i32) {
    let cx = div_floor(tx, CHUNK_SIZE);
    let cy = div_floor(ty, CHUNK_SIZE);
    let lx = mod_floor(tx, CHUNK_SIZE);
    let ly = mod_floor(ty, CHUNK_SIZE);
    (cx, cy, lx, ly)
}

fn div_floor(a: i32, b: i32) -> i32 {
    let mut r = a / b;
    let m = a % b;
    if m != 0 && ((m > 0) != (b > 0)) {
        r -= 1;
    }
    r
}

fn mod_floor(a: i32, n: i32) -> i32 {
    ((a % n) + n) % n
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn noise2d(seed: u32, x: f32, y: f32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let sx = fade(x - x0 as f32);
    let sy = fade(y - y0 as f32);

    let n00 = rand_unit(seed, x0, y0);
    let n10 = rand_unit(seed, x1, y0);
    let n01 = rand_unit(seed, x0, y1);
    let n11 = rand_unit(seed, x1, y1);

    let nx0 = lerp(n00, n10, sx);
    let nx1 = lerp(n01, n11, sx);

    lerp(nx0, nx1, sy)
}

fn generate_tile(seed: u32, tx: i32, ty: i32) -> u16 {
    // Surface height: noise between 10 and 22
    let h_noise = noise2d(seed ^ 0x1234, tx as f32 * 0.05, 0.0);
    let surface_ty = 10 + (h_noise * 12.0) as i32;

    if ty < surface_ty {
        return 0; // Air
    }

    // Dirt layer
    if ty < surface_ty + 6 {
        return 1; // Dirt
    }

    // Stone and Caves
    let cave_noise = noise2d(seed ^ 0x5678, tx as f32 * 0.15, ty as f32 * 0.15);
    if cave_noise < 0.25 {
        return 0; // Cave
    }

    // Ores
    let ore_noise = rand_unit(seed ^ 0xBEEF, tx, ty);
    let depth = ty - surface_ty;
    let ore_chance = (depth as f32 / 100.0).min(0.15);
    if depth > 10 && ore_noise < ore_chance {
        let ore_index = (rand_u32(seed ^ 0xCAFE, tx, ty) % ITEM_COUNT as u32) as u16;
        return ORE_BASE + ore_index;
    }

    2 // Stone
}

fn rand_unit(seed: u32, x: i32, y: i32) -> f32 {
    let v = rand_u32(seed, x, y);
    (v as f32) / (u32::MAX as f32)
}

fn rand_u32(seed: u32, x: i32, y: i32) -> u32 {
    let mut h = seed as u64;
    h ^= (x as i64 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87);
    h ^= (y as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h & 0xFFFF_FFFF) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_origin_centers_player() {
        let player = Player {
            x: (SCREEN_W as f32 - PLAYER_W) * 0.5,
            y: (SCREEN_H as f32 - PLAYER_H) * 0.5,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            on_ground: false,
        };
        let (cam_x, cam_y) = camera_origin(&player);
        assert!((cam_x).abs() < 0.001);
        assert!((cam_y).abs() < 0.001);
    }
}
