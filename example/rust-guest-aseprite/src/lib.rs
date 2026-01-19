use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{LazyLock, Mutex};

use wasm96_sdk::prelude::*;

const SCREEN_W: u32 = 320;
const SCREEN_H: u32 = 240;
const TILE_SIZE: i32 = 8;
const CHUNK_SIZE: i32 = 32;

const PLAYER_W: f32 = 10.0;
const PLAYER_H: f32 = 14.0;

const FONT_KEY_SPLEEN_16: &str = "font/spleen/16";

const ORES_PNG: &[u8] = include_bytes!("../assets/rocks/ores.png");

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

const DIRT_TILE_INDEX: usize = 0;
const STONE_TILE_INDEX: usize = 1;
const ORE_BASE: u16 = 3;

const ITEM_COUNT: usize = 6;
const ITEM_NAMES: [&str; ITEM_COUNT] = ["Copper", "Iron", "Gold", "Ruby", "Emerald", "Sapphire"];
const ORE_TILE_INDICES: [usize; ITEM_COUNT] = [2, 3, 4, 5, 6, 7];

static GAME_STATE: LazyLock<Mutex<GameState>> = LazyLock::new(|| Mutex::new(GameState::new()));

#[derive(Clone)]
struct TileImage {
    w: u32,
    h: u32,
    pixels: Vec<u8>,
}

#[derive(Clone)]
struct OreDef {
    item_index: usize,
    tile_index: usize,
}

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
    tiles: Vec<TileImage>,
    ores: Vec<OreDef>,
    player: Player,
    inventory: [u32; ITEM_COUNT],
    mining_timer_ms: u32,
    last_ms: u64,
}

impl GameState {
    fn new() -> Self {
        Self {
            initialized: false,
            world: World::new(0),
            tiles: Vec::new(),
            ores: Vec::new(),
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

    state.player.x = (SCREEN_W as f32 - PLAYER_W) * 0.5;
    state.player.y = (SCREEN_H as f32 - PLAYER_H) * 0.5;
    state.player.vx = 0.0;
    state.player.vy = 0.0;
    state.player.on_ground = false;

    let center_tx = (state.player.x / TILE_SIZE as f32).floor() as i32;
    let center_ty = (state.player.y / TILE_SIZE as f32).floor() as i32;
    for ty in (center_ty - 2)..=(center_ty + 2) {
        for tx in (center_tx - 2)..=(center_tx + 2) {
            state.world.set_tile_id(tx, ty, 0);
        }
    }

    let (w, h, pixels) = decode_png_rgba(ORES_PNG);
    state.tiles = split_tiles(w, h, TILE_SIZE as u32, TILE_SIZE as u32, &pixels);
    state.ores = build_ores();

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

    let mut move_dir = 0;
    if input::is_button_down(0, Button::Left) {
        move_dir -= 1;
    }
    if input::is_button_down(0, Button::Right) {
        move_dir += 1;
    }

    if move_dir != 0 {
        state.player.facing = move_dir;
    }

    let speed = 1.4;
    state.player.vx = move_dir as f32 * speed;

    let gravity = 0.25;
    state.player.vy += gravity;
    if state.player.vy > 3.8 {
        state.player.vy = 3.8;
    }

    let px = state.player.x;
    let py = state.player.y;

    let new_x = px + state.player.vx;
    if !collides(&mut state.world, new_x, py, PLAYER_W, PLAYER_H) {
        state.player.x = new_x;
    }

    let cur_x = state.player.x;
    let new_y = py + state.player.vy;
    if !collides(&mut state.world, cur_x, new_y, PLAYER_W, PLAYER_H) {
        state.player.y = new_y;
    } else {
        state.player.vy = 0.0;
    }

    let cur_x = state.player.x;
    let cur_y = state.player.y;
    let on_ground = collides(&mut state.world, cur_x, cur_y + 1.0, PLAYER_W, PLAYER_H);
    state.player.on_ground = on_ground;

    if on_ground && input::is_button_down(0, Button::B) {
        state.player.vy = -3.8;
    }

    if state.mining_timer_ms > 0 {
        state.mining_timer_ms = state.mining_timer_ms.saturating_sub(dt_ms);
    }

    let mining_input = input::is_button_down(0, Button::A) || input::is_mouse_down(0);
    if mining_input && state.mining_timer_ms == 0 {
        state.mining_timer_ms = 180;

        let target_tx;
        let target_ty;

        if input::is_mouse_down(0) {
            let (cam_x, cam_y) = camera_origin(&state.player);
            let mx = input::get_mouse_x() as f32 + cam_x;
            let my = input::get_mouse_y() as f32 + cam_y;
            target_tx = (mx / TILE_SIZE as f32).floor() as i32;
            target_ty = (my / TILE_SIZE as f32).floor() as i32;
        } else {
            let offset_x = state.player.facing as f32 * (PLAYER_W * 0.6 + TILE_SIZE as f32);
            let offset_y = PLAYER_H * 0.4;
            let wx = state.player.x + offset_x;
            let wy = state.player.y + offset_y;
            target_tx = (wx / TILE_SIZE as f32).floor() as i32;
            target_ty = (wy / TILE_SIZE as f32).floor() as i32;
        }

        let dx = target_tx * TILE_SIZE - state.player.x as i32;
        let dy = target_ty * TILE_SIZE - state.player.y as i32;
        if dx.abs() <= TILE_SIZE * 4 && dy.abs() <= TILE_SIZE * 4 {
            let mined = state.world.mine_tile(target_tx, target_ty);
            if let Some(item_index) = item_for_tile(&state.ores, mined) {
                state.inventory[item_index] = state.inventory[item_index].saturating_add(1);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn draw() {
    let mut state = GAME_STATE.lock().unwrap();
    if !state.initialized {
        return;
    }

    graphics::background(10, 12, 18);

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
    if state.tiles.is_empty() {
        return;
    }

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
            let tile_index = tile_image_index(tile_id, state.tiles.len(), &state.ores);
            let tile = &state.tiles[tile_index];
            let sx = tx * TILE_SIZE - cam_x as i32;
            let sy = ty * TILE_SIZE - cam_y as i32;
            graphics::image(sx, sy, tile.w, tile.h, &tile.pixels);
        }
    }
}

fn draw_player(state: &mut GameState, cam_x: f32, cam_y: f32) {
    let moving = state.player.vx.abs() > 0.01;
    let mining = state.mining_timer_ms > 0;
    let on_ground = state.player.on_ground;

    let anim_key = if mining && on_ground {
        DWARF_SHOVEL_KEY
    } else if mining {
        DWARF_SWING_KEY
    } else if moving {
        DWARF_MOVE_KEY
    } else {
        DWARF_IDLE_KEY
    };

    let sx = (state.player.x - cam_x) as i32;
    let sy = (state.player.y - cam_y) as i32;

    graphics::aseprite_play_key(anim_key, sx, sy, DWARF_DEFAULT_TAG);
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

fn build_ores() -> Vec<OreDef> {
    let mut ores = Vec::new();
    for (i, tile_index) in ORE_TILE_INDICES.iter().enumerate() {
        ores.push(OreDef {
            item_index: i,
            tile_index: *tile_index,
        });
    }
    ores
}

fn item_for_tile(ores: &[OreDef], tile_id: u16) -> Option<usize> {
    if tile_id >= ORE_BASE {
        let idx = (tile_id - ORE_BASE) as usize;
        if idx < ores.len() {
            return Some(ores[idx].item_index);
        }
    }
    None
}

fn tile_image_index(tile_id: u16, tile_len: usize, ores: &[OreDef]) -> usize {
    if tile_len == 0 {
        return 0;
    }
    if tile_id == 1 {
        return DIRT_TILE_INDEX.min(tile_len - 1);
    }
    if tile_id == 2 {
        return STONE_TILE_INDEX.min(tile_len - 1);
    }
    if tile_id >= ORE_BASE {
        let idx = (tile_id - ORE_BASE) as usize;
        if idx < ores.len() {
            return ores[idx].tile_index % tile_len;
        }
    }
    0
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

fn mod_floor(a: i32, b: i32) -> i32 {
    let m = a % b;
    if m < 0 { m + b } else { m }
}

fn generate_tile(seed: u32, tx: i32, ty: i32) -> u16 {
    if ty < 0 {
        return 0;
    }

    let depth = ty as i32;
    let base = if depth < 4 {
        1
    } else if depth < 12 {
        1
    } else {
        2
    };

    let cave_noise = rand_unit(seed ^ 0xA53C_F00D, tx, ty);
    if depth > 2 && cave_noise < 0.08 {
        return 0;
    }

    let ore_noise = rand_unit(seed ^ 0xBEEF_CAFE, tx, ty);
    let ore_chance = (depth as f32 / 80.0).min(0.28);
    if depth > 4 && ore_noise < ore_chance {
        let ore_index = (rand_u32(seed ^ 0xCAFE_BABE, tx, ty) % ITEM_COUNT as u32) as u16;
        return ORE_BASE + ore_index;
    }

    base as u16
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

fn decode_png_rgba(data: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_transformations(
        png::Transformations::ALPHA | png::Transformations::EXPAND | png::Transformations::STRIP_16,
    );
    let mut reader = decoder.read_info().expect("png read_info failed");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png next_frame failed");
    let bytes = buf[..info.buffer_size()].to_vec();

    let expected_rgba = (info.width * info.height * 4) as usize;
    if bytes.len() == expected_rgba {
        return (info.width, info.height, bytes);
    }

    let expected_rgb = (info.width * info.height * 3) as usize;
    if bytes.len() == expected_rgb {
        let mut rgba = Vec::with_capacity(expected_rgba);
        for chunk in bytes.chunks_exact(3) {
            rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
        }
        return (info.width, info.height, rgba);
    }

    let expected_gray = (info.width * info.height) as usize;
    if bytes.len() == expected_gray {
        let mut rgba = Vec::with_capacity(expected_rgba);
        for value in bytes.iter() {
            rgba.extend_from_slice(&[*value, *value, *value, 255]);
        }
        return (info.width, info.height, rgba);
    }

    let expected_gray_alpha = (info.width * info.height * 2) as usize;
    if bytes.len() == expected_gray_alpha {
        let mut rgba = Vec::with_capacity(expected_rgba);
        for chunk in bytes.chunks_exact(2) {
            rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
        }
        return (info.width, info.height, rgba);
    }

    (info.width, info.height, bytes)
}

fn split_tiles(width: u32, height: u32, tile_w: u32, tile_h: u32, pixels: &[u8]) -> Vec<TileImage> {
    let mut tiles = Vec::new();
    let tiles_x = width / tile_w;
    let tiles_y = height / tile_h;

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let mut tile_pixels = Vec::with_capacity((tile_w * tile_h * 4) as usize);
            for y in 0..tile_h {
                for x in 0..tile_w {
                    let src_x = tx * tile_w + x;
                    let src_y = ty * tile_h + y;
                    let index = ((src_y * width + src_x) * 4) as usize;
                    tile_pixels.extend_from_slice(&pixels[index..index + 4]);
                }
            }
            tiles.push(TileImage {
                w: tile_w,
                h: tile_h,
                pixels: tile_pixels,
            });
        }
    }

    tiles
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
