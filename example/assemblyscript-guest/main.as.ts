// wasm96 AssemblyScript guest example: "Generated Flappy" (Flappy Bird-style)
//
// This file intentionally does "manual imports" of the wasm96 core ABI functions
// (i.e. no SDK wrapper / no WIT bindings).
//
// Host calls:
// - setup() once
// - update() once per frame
// - draw() once per frame
//
// Controls (gamepad port 0):
// - A / B / Up: flap
// - Start: restart
//
// Notes:
// - Text APIs in wasm96 take a raw pointer + length (bytes). AssemblyScript `string` is
//   UTF-16 internally, so we use a tiny ASCII/UTF-8 scratch buffer and pass that instead.
// - We also implement a tiny FNV-1a hash for font keys, matching wasm96 SDK behavior.
// - Flap strength has mild diminishing returns when you flap repeatedly in quick succession.

//// ---------------------------
//// Manual wasm imports (env.*)
//// ---------------------------

@external("env", "wasm96_graphics_set_size")
declare function wasm96_graphics_set_size(width: u32, height: u32): void;

@external("env", "wasm96_graphics_set_color")
declare function wasm96_graphics_set_color(r: u32, g: u32, b: u32, a: u32): void;

@external("env", "wasm96_graphics_background")
declare function wasm96_graphics_background(r: u32, g: u32, b: u32): void;

@external("env", "wasm96_graphics_rect")
declare function wasm96_graphics_rect(x: i32, y: i32, w: u32, h: u32): void;

@external("env", "wasm96_graphics_rect_outline")
declare function wasm96_graphics_rect_outline(x: i32, y: i32, w: u32, h: u32): void;

@external("env", "wasm96_graphics_circle")
declare function wasm96_graphics_circle(x: i32, y: i32, r: u32): void;

@external("env", "wasm96_graphics_font_register_spleen")
declare function wasm96_graphics_font_register_spleen(key: u64, size: u32): u32;

@external("env", "wasm96_graphics_text_key")
declare function wasm96_graphics_text_key(x: i32, y: i32, font_key: u64, text_ptr: usize, text_len: u32): void;

@external("env", "wasm96_input_is_button_down")
declare function wasm96_input_is_button_down(port: u32, btn: u32): u32;

@external("env", "wasm96_system_millis")
declare function wasm96_system_millis(): u64;

@external("env", "wasm96_system_log")
declare function wasm96_system_log(ptr: usize, len: u32): void;

//// ---------------------------
//// Constants / config
//// ---------------------------

const W: i32 = 320;
const H: i32 = 240;

const PORT0: u32 = 0;

// wasm96 button ids (from wasm96-c-sdk/wasm96.h)
const BTN_B: u32 = 0;
const BTN_Y: u32 = 1;
const BTN_SELECT: u32 = 2;
const BTN_START: u32 = 3;
const BTN_UP: u32 = 4;
const BTN_DOWN: u32 = 5;
const BTN_LEFT: u32 = 6;
const BTN_RIGHT: u32 = 7;
const BTN_A: u32 = 8;

const HUD_FONT_SIZE: u32 = 16;

// Physics (tuned for 60fps-ish fixed step)
const GRAVITY: f32 = 0.40;
const FLAP_VY: f32 = -6.5;

// Diminishing flap tuning:
// If you flap repeatedly without "cooling down", each new flap is slightly weaker.
const FLAP_CHAIN_WINDOW_FRAMES: i32 = 18; // flaps within this window count as a chain
const FLAP_CHAIN_DECAY: f32 = 0.25;       // how much power is lost per chain step
const FLAP_CHAIN_MAX: i32 = 8;            // clamp so it never gets ridiculous

const BIRD_X: f32 = 80.0;
const BIRD_R: f32 = 6.0;

const PIPE_W: f32 = 26.0;
const PIPE_SPEED: f32 = 2.2;

const GAP_H: f32 = 70.0;
const PIPE_MIN_Y: f32 = 40.0;  // min center
const PIPE_MAX_Y: f32 = 190.0; // max center

const GROUND_Y: f32 = 220.0;

//// ---------------------------
//// Tiny utilities (hash, rng, ascii buffer)
//// ---------------------------

function fnv1a64Ascii(s: string): u64 {
  // FNV-1a 64-bit, matches wasm96 C SDK helper (byte-wise)
  let hash: u64 = 0xcbf29ce484222325;
  for (let i = 0; i < s.length; i++) {
    // We assume ASCII keys ("spleen") so charCodeAt <= 0x7F.
    hash ^= <u64>(s.charCodeAt(i) & 0xff);
    hash *= 0x100000001b3;
  }
  return hash;
}

class Rng {
  state: u32;
  constructor(seed: u32) {
    this.state = seed != 0 ? seed : 0x12345678;
  }
  nextU32(): u32 {
    // xorshift32
    let x = this.state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    this.state = x;
    return x;
  }
  rangeF32(lo: f32, hi: f32): f32 {
    // use top 24 bits for a pseudo-float in [0,1)
    const u = (this.nextU32() >>> 8) & 0x00ffffff;
    const t = <f32>u / 16777216.0;
    return lo + (hi - lo) * t;
  }
}

// A small ASCII scratch buffer for logging/text.
// We keep it global so its pointer stays stable.
const TEXT_BUF_CAP: i32 = 128;

let textBuf: Uint8Array = new Uint8Array(TEXT_BUF_CAP);

function textBufPtr(): usize {
  // IMPORTANT:
  // wasm96 expects a pointer to the *bytes*, not a pointer to the ArrayBuffer object.
  // For a Uint8Array, `dataStart` is the linear-memory address of element 0.
  return textBuf.dataStart;
}

function writeAsciiToBuf(s: string): u32 {
  // Writes ASCII (or best-effort low 7-bit) into textBuf, returns byte length.
  let n: i32 = s.length;
  if (n > TEXT_BUF_CAP - 1) n = TEXT_BUF_CAP - 1;
  for (let i: i32 = 0; i < n; i++) {
    let c = s.charCodeAt(i);
    if (c < 0) c = 0;
    if (c > 127) c = 63; // '?'
    textBuf[i] = <u8>c;
  }
  return <u32>n;
}

function logAscii(s: string): void {
  const len = writeAsciiToBuf(s);
  wasm96_system_log(textBufPtr(), len);
}

function drawText(x: i32, y: i32, fontKey: u64, s: string): void {
  const len = writeAsciiToBuf(s);
  wasm96_graphics_text_key(x, y, fontKey, textBufPtr(), len);
}

function clampf(v: f32, lo: f32, hi: f32): f32 {
  return v < lo ? lo : (v > hi ? hi : v);
}

function rectIntersect(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32): bool {
  if (ax + aw <= bx) return false;
  if (bx + bw <= ax) return false;
  if (ay + ah <= by) return false;
  if (by + bh <= ay) return false;
  return true;
}

//// ---------------------------
//// Game state
//// ---------------------------

let hudFontKey: u64 = 0;

let rng: Rng | null = null;

let started: bool = false;
let dead: bool = false;
let deathLockFrames: i32 = 0; // small cooldown so restart/flap edges don't get eaten

let birdY: f32 = 120.0;
let birdVy: f32 = 0.0;

let pipeX: f32 = 260.0;
let pipeGapCenterY: f32 = 120.0;
let pipePassed: bool = false;

let score: i32 = 0;
let best: i32 = 0;

let prevFlapDown: bool = false;
let prevStartDown: bool = false;

// Flap diminishing returns state
let flapChain: i32 = 0;
let lastFlapFrame: i32 = -1000000;
let frameCounter: i32 = 0;

function resetGame(): void {
  started = false;
  dead = false;
  deathLockFrames = 8;

  birdY = 120.0;
  birdVy = 0.0;

  pipeX = 260.0;
  pipeGapCenterY = randomGapCenter();
  pipePassed = false;

  score = 0;

  // Reset edge tracking so the first press after restart is always seen.
  prevFlapDown = false;
  prevStartDown = false;

  // Reset diminishing flap state
  flapChain = 0;
  lastFlapFrame = frameCounter - 1000000;
}

function randomGapCenter(): f32 {
  const r = rng;
  if (r == null) return 120.0;
  return r.rangeF32(PIPE_MIN_Y, PIPE_MAX_Y);
}

function flapPressedNow(): bool {
  const downA = wasm96_input_is_button_down(PORT0, BTN_A) != 0;
  const downB = wasm96_input_is_button_down(PORT0, BTN_B) != 0;
  const downUp = wasm96_input_is_button_down(PORT0, BTN_UP) != 0;
  const down = downA || downB || downUp;

  // Rising edge
  const pressed = down && !prevFlapDown;
  prevFlapDown = down;
  return pressed;
}

function startPressedNow(): bool {
  const down = wasm96_input_is_button_down(PORT0, BTN_START) != 0;
  const pressed = down && !prevStartDown;
  prevStartDown = down;
  return pressed;
}

function spawnPipe(): void {
  pipeX = <f32>(W + 20);
  pipeGapCenterY = randomGapCenter();
  pipePassed = false;
}

function stepGame(): void {
  frameCounter++;

  // Always allow restart, even while dead.
  if (startPressedNow()) {
    resetGame();
    return;
  }

  // After a death/restart, give a few frames so edge tracking doesn't get stuck.
  if (deathLockFrames > 0) {
    deathLockFrames--;
    // Still update flap edge state so first press after lock is detected properly.
    flapPressedNow();
    return;
  }

  // If dead, allow flap to instantly restart and jump (more forgiving).
  if (dead) {
    if (flapPressedNow()) {
      resetGame();
      started = true;

      // Treat this as a fresh first flap (no decay).
      flapChain = 0;
      lastFlapFrame = frameCounter;

      birdVy = FLAP_VY;
    }
    return;
  }

  const flap = flapPressedNow();

  if (!started) {
    if (flap) {
      started = true;

      // First flap starts the chain.
      flapChain = 0;
      lastFlapFrame = frameCounter;

      birdVy = FLAP_VY; // don't start by falling into guaranteed death
    } else {
      // idle bob
      birdVy = 0.0;
    }
    return;
  }

  if (flap) {
    // Diminishing returns: flaps close together get progressively weaker.
    if (frameCounter - lastFlapFrame <= FLAP_CHAIN_WINDOW_FRAMES) {
      flapChain = flapChain + 1;
      if (flapChain > FLAP_CHAIN_MAX) flapChain = FLAP_CHAIN_MAX;
    } else {
      flapChain = 0;
    }
    lastFlapFrame = frameCounter;

    // Less negative = less upward impulse.
    const vy = FLAP_VY + <f32>flapChain * FLAP_CHAIN_DECAY;
    birdVy = vy;
  }

  // bird physics
  birdVy += GRAVITY;
  birdY += birdVy;

  // pipe movement
  pipeX -= PIPE_SPEED;
  if (pipeX + PIPE_W < 0.0) {
    spawnPipe();
  }

  // scoring: when pipe crosses bird x
  if (!pipePassed && pipeX + PIPE_W < BIRD_X) {
    pipePassed = true;
    score++;
    if (score > best) best = score;
  }

  // collisions
  if (birdY - BIRD_R < 0.0) {
    birdY = BIRD_R;
    birdVy = 0.0;
  }

  if (birdY + BIRD_R >= GROUND_Y) {
    dead = true;
    started = false;
    deathLockFrames = 8;
    return;
  }

  const gapTop = pipeGapCenterY - GAP_H * 0.5;
  const gapBot = pipeGapCenterY + GAP_H * 0.5;

  const birdAabbX = BIRD_X - BIRD_R;
  const birdAabbY = birdY - BIRD_R;
  const birdAabbW = BIRD_R * 2.0;
  const birdAabbH = BIRD_R * 2.0;

  // upper pipe rect
  if (rectIntersect(birdAabbX, birdAabbY, birdAabbW, birdAabbH, pipeX, 0.0, PIPE_W, gapTop)) {
    dead = true;
    started = false;
    deathLockFrames = 8;
    return;
  }

  // lower pipe rect
  if (rectIntersect(birdAabbX, birdAabbY, birdAabbW, birdAabbH, pipeX, gapBot, PIPE_W, GROUND_Y - gapBot)) {
    dead = true;
    started = false;
    deathLockFrames = 8;
    return;
  }
}

//// ---------------------------
//// Guest entrypoints
//// ---------------------------

export function setup(): void {
  wasm96_graphics_set_size(<u32>W, <u32>H);

  hudFontKey = fnv1a64Ascii("spleen");
  // Register built-in Spleen font at desired size for text rendering.
  // Even if core provides fallback, making this explicit is nice.
  wasm96_graphics_font_register_spleen(hudFontKey, HUD_FONT_SIZE);

  // Seed RNG
  const ms = wasm96_system_millis();
  rng = new Rng(<u32>(ms as u32));

  resetGame();
  logAscii("asm-guest: generated flappy loaded");
}

export function update(): void {
  // We still need to update input edge-tracking even when paused/dead,
  // so flapPressedNow/startPressedNow are called from stepGame().
  stepGame();
}

export function draw(): void {
  // Background
  wasm96_graphics_background(20, 20, 40);

  // Ground
  wasm96_graphics_set_color(40, 120, 60, 255);
  wasm96_graphics_rect(0, <i32>GROUND_Y, <u32>W, <u32>(H - <i32>GROUND_Y));

  // Pipes
  const gapTop = pipeGapCenterY - GAP_H * 0.5;
  const gapBot = pipeGapCenterY + GAP_H * 0.5;

  wasm96_graphics_set_color(40, 200, 90, 255);
  // upper
  wasm96_graphics_rect(<i32>pipeX, 0, <u32>PIPE_W, <u32>clampf(gapTop, 0.0, GROUND_Y));
  // lower
  wasm96_graphics_rect(<i32>pipeX, <i32>gapBot, <u32>PIPE_W, <u32>clampf(GROUND_Y - gapBot, 0.0, GROUND_Y));

  // Pipe outlines
  wasm96_graphics_set_color(10, 80, 30, 255);
  wasm96_graphics_rect_outline(<i32>pipeX, 0, <u32>PIPE_W, <u32>clampf(gapTop, 0.0, GROUND_Y));
  wasm96_graphics_rect_outline(<i32>pipeX, <i32>gapBot, <u32>PIPE_W, <u32>clampf(GROUND_Y - gapBot, 0.0, GROUND_Y));

  // Bird
  wasm96_graphics_set_color(255, 220, 80, 255);
  wasm96_graphics_circle(<i32>BIRD_X, <i32>birdY, <u32>BIRD_R);

  wasm96_graphics_set_color(40, 30, 10, 255);
  wasm96_graphics_circle(<i32>(BIRD_X + 2.0), <i32>(birdY - 1.0), 1);

  // HUD text
  wasm96_graphics_set_color(255, 255, 255, 255);

  drawText(8, 8, hudFontKey, "Generated Flappy");
  drawText(8, 24, hudFontKey, "Score: " + score.toString());
  drawText(8, 40, hudFontKey, "Best: " + best.toString());

  if (!started && !dead) {
    drawText(8, 64, hudFontKey, "Press A/B/Up to flap");
    drawText(8, 80, hudFontKey, "Start: restart");
  }

  if (dead) {
    wasm96_graphics_set_color(255, 90, 90, 255);
    drawText(92, 110, hudFontKey, "CRASH!");
    wasm96_graphics_set_color(255, 255, 255, 255);
    drawText(62, 130, hudFontKey, "Press Start to retry");
  }
}
