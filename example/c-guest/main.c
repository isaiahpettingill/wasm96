#include "wasm96.h"
#include <stdint.h>
#include <stdbool.h>

// Playable Snake for wasm96 C guest (freestanding-friendly).
//
// Controls (gamepad, port 0):
//   - D-Pad: change direction
//   - Start: pause/unpause
//   - Select: restart
//
// Notes:
//   - No dynamic allocation.
//   - Uses a grid and draws filled rects.
//   - Text rendering: core now falls back to Spleen 16 when no font registered,
//     so we can safely call wasm96_graphics_text_key_str with any font key.

#define SCREEN_W 640
#define SCREEN_H 480

// Grid / board layout
#define CELL_SIZE 16
#define COLS 30
#define ROWS 24
#define BOARD_X 80
#define BOARD_Y 48

// Derived
#define MAX_CELLS (COLS * ROWS)

// Timing (in frames; assumes ~60fps)
#define STEP_FRAMES_START 10
#define STEP_FRAMES_MIN 4

typedef struct {
    int16_t x;
    int16_t y;
} Point;

typedef struct {
    uint32_t state;
} Rng;

static inline void rng_seed(Rng* r, uint32_t seed) {
    r->state = seed ? seed : 0x12345678u;
}

static inline uint32_t rng_u32(Rng* r) {
    // xorshift32
    uint32_t x = r->state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    r->state = x;
    return x;
}

static inline int rng_range(Rng* r, int lo, int hi) {
    uint32_t span = (uint32_t)(hi - lo + 1);
    return lo + (int)(rng_u32(r) % span);
}

typedef enum {
    DIR_UP = 0,
    DIR_RIGHT = 1,
    DIR_DOWN = 2,
    DIR_LEFT = 3
} Dir;

typedef struct {
    // Snake body as ring buffer of points.
    Point body[MAX_CELLS];
    int head;      // index of head in ring buffer
    int len;       // current length
    Dir dir;
    Dir next_dir;

    // Food position
    Point food;

    // Game state
    bool paused;
    bool game_over;

    // Score
    int score;
    int best;

    // Step timing
    int step_frames;
    int step_counter;

    // Input debouncing
    bool prev_btn[16];

    // Random
    Rng rng;

    // Occupancy grid for fast food placement
    uint8_t occ[MAX_CELLS]; // 0 empty, 1 snake
} Game;

static Game g;

static inline int idx_of(int x, int y) {
    return y * COLS + x;
}

static inline bool in_bounds(int x, int y) {
    return (x >= 0 && x < COLS && y >= 0 && y < ROWS);
}

static inline bool btn_down(uint32_t btn) {
    return wasm96_input_is_button_down(0, btn) != 0;
}

static inline bool btn_pressed(uint32_t btn) {
    bool now = btn_down(btn);
    bool pressed = now && !g.prev_btn[btn];
    g.prev_btn[btn] = now;
    return pressed;
}

static inline void sync_buttons(void) {
    for (int i = 0; i < 16; i++) {
        g.prev_btn[i] = btn_down((uint32_t)i);
    }
}

static void occ_clear(void) {
    for (int i = 0; i < MAX_CELLS; i++) g.occ[i] = 0;
}

static void occ_set_point(Point p, uint8_t v) {
    if (!in_bounds(p.x, p.y)) return;
    g.occ[idx_of(p.x, p.y)] = v;
}

static uint8_t occ_get(int x, int y) {
    if (!in_bounds(x, y)) return 0;
    return g.occ[idx_of(x, y)];
}

static void snake_reset(void) {
    g.head = 0;
    g.len = 3;
    g.dir = DIR_RIGHT;
    g.next_dir = DIR_RIGHT;
    g.paused = false;
    g.game_over = false;
    g.score = 0;
    g.step_frames = STEP_FRAMES_START;
    g.step_counter = 0;

    occ_clear();

    // Start centered
    int sx = COLS / 2;
    int sy = ROWS / 2;

    // Body: tail to head in consecutive cells
    // ring buffer stores points; we keep head index pointing to newest.
    // We'll initialize body[0..len-1] such that head is at index len-1.
    for (int i = 0; i < g.len; i++) {
        Point p;
        p.x = (int16_t)(sx - (g.len - 1 - i));
        p.y = (int16_t)sy;
        g.body[i] = p;
        occ_set_point(p, 1);
    }
    g.head = g.len - 1;
}

static Point snake_head(void) {
    return g.body[g.head];
}

__attribute__((unused)) static Point snake_tail(void) {
    int tail = g.head - (g.len - 1);
    while (tail < 0) tail += MAX_CELLS;
    return g.body[tail];
}

static bool snake_contains(int x, int y) {
    return occ_get(x, y) != 0;
}

static void place_food(void) {
    // Try a bounded number of random attempts, then fall back to scan.
    for (int tries = 0; tries < 200; tries++) {
        int fx = rng_range(&g.rng, 0, COLS - 1);
        int fy = rng_range(&g.rng, 0, ROWS - 1);
        if (!snake_contains(fx, fy)) {
            g.food.x = (int16_t)fx;
            g.food.y = (int16_t)fy;
            return;
        }
    }

    for (int y = 0; y < ROWS; y++) {
        for (int x = 0; x < COLS; x++) {
            if (!snake_contains(x, y)) {
                g.food.x = (int16_t)x;
                g.food.y = (int16_t)y;
                return;
            }
        }
    }

    // No space left: player wins; keep food where it is.
}

static void game_reset(uint32_t seed) {
    rng_seed(&g.rng, seed);
    snake_reset();
    place_food();
    sync_buttons();
}

static inline bool dir_is_opposite(Dir a, Dir b) {
    return ((a == DIR_UP && b == DIR_DOWN) ||
            (a == DIR_DOWN && b == DIR_UP) ||
            (a == DIR_LEFT && b == DIR_RIGHT) ||
            (a == DIR_RIGHT && b == DIR_LEFT));
}

static void handle_input(void) {
    if (btn_pressed(WASM96_BUTTON_START)) {
        g.paused = !g.paused;
    }
    if (btn_pressed(WASM96_BUTTON_SELECT)) {
        game_reset((uint32_t)wasm96_system_millis());
        return;
    }

    // Direction changes (debounced by edge)
    // Note: do not allow immediate reversal.
    Dir desired = g.next_dir;

    if (btn_pressed(WASM96_BUTTON_UP)) desired = DIR_UP;
    else if (btn_pressed(WASM96_BUTTON_RIGHT)) desired = DIR_RIGHT;
    else if (btn_pressed(WASM96_BUTTON_DOWN)) desired = DIR_DOWN;
    else if (btn_pressed(WASM96_BUTTON_LEFT)) desired = DIR_LEFT;

    if (!dir_is_opposite(desired, g.dir)) {
        g.next_dir = desired;
    }
}

static void step_snake(void) {
    if (g.game_over || g.paused) return;

    // Apply direction choice at step boundary
    g.dir = g.next_dir;

    Point h = snake_head();
    Point nh = h;

    switch (g.dir) {
        case DIR_UP:    nh.y -= 1; break;
        case DIR_RIGHT: nh.x += 1; break;
        case DIR_DOWN:  nh.y += 1; break;
        case DIR_LEFT:  nh.x -= 1; break;
        default: break;
    }

    // Wall collision
    if (!in_bounds(nh.x, nh.y)) {
        g.game_over = true;
        return;
    }

    bool eating = (nh.x == g.food.x && nh.y == g.food.y);

    // Compute tail index before move
    int tail_idx = g.head - (g.len - 1);
    while (tail_idx < 0) tail_idx += MAX_CELLS;

    // Self collision:
    // Moving into the current tail cell is allowed if we're not eating (tail moves away).
    uint8_t hit = occ_get(nh.x, nh.y);
    if (hit) {
        Point tail = g.body[tail_idx];
        bool into_tail = (nh.x == tail.x && nh.y == tail.y);
        if (!(into_tail && !eating)) {
            g.game_over = true;
            return;
        }
    }

    // Advance head index
    int new_head = g.head + 1;
    if (new_head >= MAX_CELLS) new_head = 0;

    g.body[new_head] = nh;
    g.head = new_head;
    occ_set_point(nh, 1);

    if (eating) {
        g.len += 1;
        g.score += 10;

        // Speed up a bit over time, clamped
        if ((g.score % 50) == 0 && g.step_frames > STEP_FRAMES_MIN) {
            g.step_frames -= 1;
        }

        if (g.score > g.best) g.best = g.score;

        place_food();
    } else {
        // Remove tail
        Point tail = g.body[tail_idx];
        occ_set_point(tail, 0);
        // Length unchanged; tail index implicitly moves forward next time.
    }

    // If filled board, player wins; treat as game over for now.
    if (g.len >= MAX_CELLS) {
        g.game_over = true;
    }
}

static void update_timing_and_step(void) {
    g.step_counter++;
    if (g.step_counter >= g.step_frames) {
        g.step_counter = 0;
        step_snake();
    }
}

static void draw_cell(int x, int y, uint8_t r, uint8_t g_, uint8_t b, uint8_t a) {
    int px = BOARD_X + x * CELL_SIZE;
    int py = BOARD_Y + y * CELL_SIZE;
    wasm96_graphics_set_color_rgba(r, g_, b, a);
    wasm96_graphics_rect(px, py, (uint32_t)CELL_SIZE, (uint32_t)CELL_SIZE);
}

static void draw_board(void) {
    // Board backdrop
    int w = COLS * CELL_SIZE;
    int h = ROWS * CELL_SIZE;

    wasm96_graphics_set_color_rgba(10, 10, 40, 255);
    wasm96_graphics_rect(BOARD_X - 2, BOARD_Y - 2, (uint32_t)(w + 4), (uint32_t)(h + 4));

    wasm96_graphics_set_color_rgba(180, 180, 220, 255);
    wasm96_graphics_rect_outline(BOARD_X - 2, BOARD_Y - 2, (uint32_t)(w + 4), (uint32_t)(h + 4));

    // Light grid (optional)
    wasm96_graphics_set_color_rgba(30, 30, 80, 255);
    for (int c = 1; c < COLS; c++) {
        int x = BOARD_X + c * CELL_SIZE;
        wasm96_graphics_line(x, BOARD_Y, x, BOARD_Y + h);
    }
    for (int r = 1; r < ROWS; r++) {
        int y = BOARD_Y + r * CELL_SIZE;
        wasm96_graphics_line(BOARD_X, y, BOARD_X + w, y);
    }
}

static void draw_snake_and_food(void) {
    // Food
    draw_cell(g.food.x, g.food.y, 240, 80, 80, 255);

    // Snake (iterate over length)
    // Head brighter
    for (int i = 0; i < g.len; i++) {
        int idx = g.head - i;
        while (idx < 0) idx += MAX_CELLS;

        Point p = g.body[idx];

        if (i == 0) {
            draw_cell(p.x, p.y, 120, 255, 120, 255);
        } else {
            draw_cell(p.x, p.y, 60, 200, 90, 255);
        }
    }
}

static void write_int(char* out, int v) {
    // Minimal int to string (null-terminated)
    char buf[16];
    int n = 0;
    int sign = 0;

    if (v == 0) {
        out[0] = '0';
        out[1] = '\0';
        return;
    }

    if (v < 0) {
        sign = 1;
        v = -v;
    }

    while (v > 0 && n < 15) {
        buf[n++] = (char)('0' + (v % 10));
        v /= 10;
    }

    int pos = 0;
    if (sign) out[pos++] = '-';
    for (int i = n - 1; i >= 0; i--) out[pos++] = buf[i];
    out[pos] = '\0';
}

static void draw_hud(void) {
    // Sidebar
    int hud_x = 16;
    int hud_y = 16;

    wasm96_graphics_set_color_rgba(240, 240, 255, 255);
    wasm96_graphics_text_key_str(hud_x, hud_y, "spleen", "WASM96 Snake (C guest)");

    char line[64];
    char num[16];

    // SCORE
    write_int(num, g.score);
    // "SCORE: " + num
    int k = 0;
    const char* pfx = "SCORE: ";
    for (; pfx[k] != '\0'; k++) line[k] = pfx[k];
    int j = 0;
    for (; num[j] != '\0' && (k + j) < 63; j++) line[k + j] = num[j];
    line[k + j] = '\0';
    wasm96_graphics_text_key_str(hud_x, hud_y + 22, "spleen", line);

    // BEST
    write_int(num, g.best);
    k = 0;
    pfx = "BEST: ";
    for (; pfx[k] != '\0'; k++) line[k] = pfx[k];
    j = 0;
    for (; num[j] != '\0' && (k + j) < 63; j++) line[k + j] = num[j];
    line[k + j] = '\0';
    wasm96_graphics_text_key_str(hud_x, hud_y + 44, "spleen", line);

    if (g.paused) {
        wasm96_graphics_set_color_rgba(255, 255, 0, 255);
        wasm96_graphics_text_key_str(hud_x, hud_y + 76, "spleen", "PAUSED");
    } else if (g.game_over) {
        wasm96_graphics_set_color_rgba(255, 120, 120, 255);
        wasm96_graphics_text_key_str(hud_x, hud_y + 76, "spleen", "GAME OVER");
        wasm96_graphics_set_color_rgba(240, 240, 255, 255);
        wasm96_graphics_text_key_str(hud_x, hud_y + 98, "spleen", "Select: restart");
    } else {
        wasm96_graphics_set_color_rgba(200, 200, 255, 255);
        wasm96_graphics_text_key_str(hud_x, hud_y + 76, "spleen", "D-Pad: move");
        wasm96_graphics_text_key_str(hud_x, hud_y + 98, "spleen", "Start: pause");
        wasm96_graphics_text_key_str(hud_x, hud_y + 120, "spleen", "Select: restart");
    }
}

void setup(void) {
    wasm96_graphics_set_size(SCREEN_W, SCREEN_H);
    wasm96_graphics_set_color_rgba(255, 255, 255, 255);

    // Optional: register spleen under the "spleen" key at size 16.
    // If the guest doesn't register a font, the core falls back to Spleen 16 anyway.
    wasm96_graphics_font_register_spleen(wasm96_hash_key("spleen"), 16);

    game_reset((uint32_t)wasm96_system_millis());
}

void update(void) {
    handle_input();
    update_timing_and_step();
}

void draw(void) {
    wasm96_graphics_background_rgb(0, 0, 50);

    draw_board();
    draw_snake_and_food();
    draw_hud();
}
