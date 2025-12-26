#include "wasm96.hpp"

#include <stdint.h>

// Minimal, self-contained Tetris implemented on top of the wasm96 C++ SDK.
// Controls (gamepad):
// - Left/Right: move
// - Down: soft drop
// - A: rotate clockwise
// - B: rotate counter-clockwise
// - Start: pause
// - Select: restart
//
// Notes:
// - Uses a simple frame-based timer (no delta time API in the C++ header).
// - Uses a 10x20 playfield with a hidden 2-row spawn area.
// - Uses a Spleen font for the HUD and saves high score to storage ("SRAM").

namespace {

constexpr int kScreenW = 640;
constexpr int kScreenH = 480;

constexpr int kCols = 10;
constexpr int kRowsVisible = 20;
constexpr int kRowsHidden = 2;
constexpr int kRows = kRowsVisible + kRowsHidden; // includes hidden spawn rows

// Storage/HUD
//
// Core semantics: fonts are keyed by u64. The C++ SDK hashes string keys for you.
// The core also documents a special built-in font key "spleen".
// We'll register a sized Spleen font under that key in setup and always render HUD
// text using it.
constexpr const char* kHudFont = "spleen";
constexpr uint32_t kHudFontSize = 16;
constexpr const char* kHighScoreKey = "tetris_high_score_v1";

// Layout
constexpr int kCell = 20;
constexpr int kFieldX = 80;
constexpr int kFieldY = 40;
constexpr int kBorder = 2;

constexpr int kNextX = kFieldX + kCols * kCell + 40;
constexpr int kNextY = kFieldY + 40;

constexpr int kHudX = kNextX;

struct Color {
    uint8_t r, g, b, a;
};

constexpr Color kBg = {0, 0, 50, 255};
constexpr Color kGrid = {30, 30, 80, 255};
constexpr Color kBorderC = {180, 180, 220, 255};
constexpr Color kText = {240, 240, 255, 255};
constexpr Color kShadow = {0, 0, 0, 100};

// Standard tetromino colors (I, O, T, S, Z, J, L)
constexpr Color kPieceColors[7] = {
    {  0, 240, 240, 255}, // I
    {240, 240,   0, 255}, // O
    {160,   0, 240, 255}, // T
    {  0, 240,   0, 255}, // S
    {240,   0,   0, 255}, // Z
    {  0,  80, 240, 255}, // J
    {240, 160,   0, 255}, // L
};

enum PieceType : int {
    I = 0, O = 1, T = 2, S = 3, Z = 4, J = 5, L = 6
};

// 4x4 bitmasks per rotation state (row-major), using 16-bit where bit (r*4+c) means filled.
// Convention: top-left is bit 0 (r=0,c=0); increase c then r.
constexpr uint16_t kShapes[7][4] = {
    // I
    {
        0b0000'1111'0000'0000,
        0b0010'0010'0010'0010,
        0b0000'0000'1111'0000,
        0b0100'0100'0100'0100,
    },
    // O
    {
        0b0000'0110'0110'0000,
        0b0000'0110'0110'0000,
        0b0000'0110'0110'0000,
        0b0000'0110'0110'0000,
    },
    // T
    {
        0b0000'0100'1110'0000,
        0b0000'0100'0110'0100,
        0b0000'0000'1110'0100,
        0b0000'0100'1100'0100,
    },
    // S
    {
        0b0000'0110'1100'0000,
        0b0000'0100'0110'0010,
        0b0000'0000'0110'1100,
        0b0000'1000'1100'0100,
    },
    // Z
    {
        0b0000'1100'0110'0000,
        0b0000'0010'0110'0100,
        0b0000'0000'1100'0110,
        0b0000'0100'1100'1000,
    },
    // J
    {
        0b0000'1000'1110'0000,
        0b0000'0110'0100'0100,
        0b0000'0000'1110'0010,
        0b0000'0100'0100'1100,
    },
    // L
    {
        0b0000'0010'1110'0000,
        0b0000'0100'0100'0110,
        0b0000'0000'1110'1000,
        0b0000'1100'0100'0100,
    },
};

inline bool shapeCell(uint16_t mask, int r, int c) {
    const int bit = r * 4 + c;
    return (mask >> (15 - bit)) & 1; // because we wrote masks visually MSB-first in 4-bit groups
}

// The masks above were written in nibble groups for readability (MSB-first),
// but our bit addressing wants consistent mapping.
// Using (15 - bit) aligns bit0 to the highest bit. This matches the literals as written.

struct RNG {
    uint32_t state = 0x12345678u;
    void seed(uint32_t s) { state = (s ? s : 0x12345678u); }
    uint32_t nextU32() {
        // xorshift32
        uint32_t x = state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        state = x;
        return x;
    }
    int nextInt(int loInclusive, int hiInclusive) {
        const uint32_t span = (uint32_t)(hiInclusive - loInclusive + 1);
        return loInclusive + (int)(nextU32() % span);
    }
};

struct InputEdge {
    bool prev[16] = {};
    bool pressed(int btn) {
        bool now = wasm96::Input::isButtonDown(0, (wasm96_button_t)btn);
        bool p = now && !prev[btn];
        prev[btn] = now;
        return p;
    }
    bool down(int btn) const {
        return wasm96::Input::isButtonDown(0, (wasm96_button_t)btn);
    }
    void sync() {
        for (int i = 0; i < 16; i++) {
            prev[i] = wasm96::Input::isButtonDown(0, (wasm96_button_t)i);
        }
    }
};

struct Game {
    // Field cells: -1 empty, otherwise 0..6 piece type index
    int8_t field[kRows][kCols] = {};

    PieceType curType = T;
    int curRot = 0;
    int curX = 3;  // position in field coordinates (col)
    int curY = 0;  // position in field coordinates (row, includes hidden)
    PieceType nextType = I;

    bool gameOver = false;
    bool paused = false;

    int score = 0;
    int lines = 0;
    int level = 1;

    int highScore = 0;
    bool highScoreDirty = false;

    // HUD font key (hashed by the SDK into a u64 for the core ABI)
    const char* hudFontKey = kHudFont;

    // Timing (in frames)
    int frame = 0;
    int fallCounter = 0;
    int lockDelay = 0;
    bool touchingGround = false;

    RNG rng;
    InputEdge edge;

    static uint32_t readU32LE(const uint8_t* p) {
        return (uint32_t)p[0]
            | ((uint32_t)p[1] << 8)
            | ((uint32_t)p[2] << 16)
            | ((uint32_t)p[3] << 24);
    }

    static void writeU32LE(uint8_t* p, uint32_t v) {
        p[0] = (uint8_t)(v & 0xFFu);
        p[1] = (uint8_t)((v >> 8) & 0xFFu);
        p[2] = (uint8_t)((v >> 16) & 0xFFu);
        p[3] = (uint8_t)((v >> 24) & 0xFFu);
    }

    void clearField() {
        for (int r = 0; r < kRows; r++) {
            for (int c = 0; c < kCols; c++) {
                field[r][c] = -1;
            }
        }
    }

    void loadHighScore() {
        // C++ SDK doesn't provide a helper for storage_load, but the raw import exists.
        // Returned u64 packs (ptr << 32) | len, or 0 if missing.
        uint64_t packed = wasm96_storage_load(wasm96_hash_key(kHighScoreKey));
        if (packed == 0) {
            highScore = 0;
            return;
        }

        uint32_t ptr = (uint32_t)(packed >> 32);
        uint32_t len = (uint32_t)(packed & 0xFFFFFFFFULL);

        int loaded = 0;
        if (ptr != 0 && len >= 4) {
            const uint8_t* p = (const uint8_t*)(uintptr_t)ptr;
            loaded = (int)readU32LE(p);
        }

        if (ptr != 0 && len != 0) {
            wasm96_storage_free((const uint8_t*)(uintptr_t)ptr, len);
        }

        if (loaded < 0) loaded = 0;
        highScore = loaded;
    }

    void maybeCommitHighScore() {
        if (!highScoreDirty) return;
        highScoreDirty = false;

        uint8_t buf[4];
        writeU32LE(buf, (uint32_t)highScore);
        wasm96::Storage::save(kHighScoreKey, buf, 4);
    }

    void reset(uint32_t seed) {
        clearField();
        rng.seed(seed);
        curType = (PieceType)rng.nextInt(0, 6);
        nextType = (PieceType)rng.nextInt(0, 6);
        curRot = 0;
        curX = 3;
        curY = 0;
        gameOver = false;
        paused = false;
        score = 0;
        lines = 0;
        level = 1;
        frame = 0;
        fallCounter = 0;
        lockDelay = 0;
        touchingGround = false;
        highScoreDirty = false;
        edge.sync();
    }

    int fallIntervalFrames() const {
        // Simple level curve: faster as level increases.
        // Clamp to avoid zero/negative.
        int base = 30;                 // ~0.5s at 60fps
        int dec = (level - 1) * 2;     // speed up
        int v = base - dec;
        if (v < 5) v = 5;
        return v;
    }

    bool collides(PieceType t, int rot, int px, int py) const {
        uint16_t m = kShapes[(int)t][rot & 3];
        for (int r = 0; r < 4; r++) {
            for (int c = 0; c < 4; c++) {
                if (!shapeCell(m, r, c)) continue;
                int fx = px + c;
                int fy = py + r;
                if (fx < 0 || fx >= kCols) return true;
                if (fy >= kRows) return true;
                if (fy >= 0) {
                    if (field[fy][fx] != -1) return true;
                }
            }
        }
        return false;
    }

    void placePieceToField() {
        uint16_t m = kShapes[(int)curType][curRot & 3];
        for (int r = 0; r < 4; r++) {
            for (int c = 0; c < 4; c++) {
                if (!shapeCell(m, r, c)) continue;
                int fx = curX + c;
                int fy = curY + r;
                if (fy >= 0 && fy < kRows && fx >= 0 && fx < kCols) {
                    field[fy][fx] = (int8_t)curType;
                }
            }
        }
    }

    int clearLines() {
        int cleared = 0;
        for (int r = 0; r < kRows; r++) {
            bool full = true;
            for (int c = 0; c < kCols; c++) {
                if (field[r][c] == -1) { full = false; break; }
            }
            if (!full) continue;

            // shift down everything above r
            for (int rr = r; rr > 0; rr--) {
                for (int c = 0; c < kCols; c++) field[rr][c] = field[rr - 1][c];
            }
            for (int c = 0; c < kCols; c++) field[0][c] = -1;
            cleared++;
        }
        return cleared;
    }

    void updateLevel() {
        level = 1 + lines / 10;
        if (level < 1) level = 1;
    }

    void awardForClears(int cleared) {
        if (cleared <= 0) return;
        // Classic-ish scoring
        static const int table[5] = {0, 100, 300, 500, 800};
        int add = table[cleared];
        score += add * level;
        lines += cleared;
        updateLevel();

        if (score > highScore) {
            highScore = score;
            highScoreDirty = true;
            maybeCommitHighScore();
        }
    }

    void spawnNext() {
        curType = nextType;
        nextType = (PieceType)rng.nextInt(0, 6);
        curRot = 0;
        curX = 3;
        curY = -1; // start slightly in hidden area
        lockDelay = 0;

        if (collides(curType, curRot, curX, curY)) {
            gameOver = true;
        }
    }

    bool tryMove(int dx, int dy) {
        if (!collides(curType, curRot, curX + dx, curY + dy)) {
            curX += dx;
            curY += dy;
            return true;
        }
        return false;
    }

    bool tryRotate(int dir) {
        // SRS-lite wall kicks (very small set).
        int newRot = (curRot + dir) & 3;
        const int kicks[6][2] = {
            {0, 0}, { -1, 0 }, { 1, 0 }, { 0, -1 }, { -2, 0 }, { 2, 0 }
        };
        for (const auto& k : kicks) {
            int nx = curX + k[0];
            int ny = curY + k[1];
            if (!collides(curType, newRot, nx, ny)) {
                curRot = newRot;
                curX = nx;
                curY = ny;
                return true;
            }
        }
        return false;
    }

    int hardDropDistance() const {
        int d = 0;
        while (!collides(curType, curRot, curX, curY + d + 1)) d++;
        return d;
    }

    void hardDrop() {
        int d = hardDropDistance();
        curY += d;
        // award a small bonus per hard-drop cell (optional)
        score += d * 2;

        if (score > highScore) {
            highScore = score;
            highScoreDirty = true;
            maybeCommitHighScore();
        }

        lockPiece();
    }

    void lockPiece() {
        placePieceToField();
        int cleared = clearLines();
        awardForClears(cleared);
        spawnNext();
        touchingGround = false;
        fallCounter = 0;
        lockDelay = 0;
    }

    void tickGameplay() {
        // Handle pause/restart
        if (edge.pressed(WASM96_BUTTON_START)) paused = !paused;
        if (edge.pressed(WASM96_BUTTON_SELECT)) {
            reset((uint32_t)wasm96::System::millis());
            loadHighScore();
            return;
        }
        if (paused || gameOver) return;

        // Movement
        if (edge.pressed(WASM96_BUTTON_LEFT)) tryMove(-1, 0);
        if (edge.pressed(WASM96_BUTTON_RIGHT)) tryMove(1, 0);

        // Rotation
        if (edge.pressed(WASM96_BUTTON_A)) tryRotate(+1);
        if (edge.pressed(WASM96_BUTTON_B)) tryRotate(-1);

        // Soft drop
        bool soft = edge.down(WASM96_BUTTON_DOWN);

        // Hard drop mapped to Y/X could be nice, but keep minimal:
        // Map L1 to hard drop if available.
        if (edge.pressed(WASM96_BUTTON_L1)) hardDrop();

        // Gravity
        int interval = fallIntervalFrames();
        if (soft) interval = 2;

        fallCounter++;
        if (fallCounter >= interval) {
            fallCounter = 0;
            if (!tryMove(0, 1)) {
                // can't fall
                if (!touchingGround) {
                    touchingGround = true;
                    lockDelay = 0;
                }
            } else {
                touchingGround = false;
                lockDelay = 0;
            }
        }

        // Lock delay if touching ground
        if (touchingGround) {
            lockDelay++;
            // a small lock delay (~0.4s)
            if (lockDelay > 24) {
                lockPiece();
            }
        }
    }

    void update() {
        frame++;
        tickGameplay();
    }
};

Game g;

void drawCell(int fx, int fy, Color c) {
    // Only draw visible rows
    const int visibleRow = fy - kRowsHidden;
    if (visibleRow < 0) return;

    int x = kFieldX + fx * kCell;
    int y = kFieldY + visibleRow * kCell;

    // shadow
    wasm96::Graphics::setColor(kShadow.r, kShadow.g, kShadow.b, kShadow.a);
    wasm96::Graphics::rect(x + 2, y + 2, kCell, kCell);

    // fill
    wasm96::Graphics::setColor(c.r, c.g, c.b, c.a);
    wasm96::Graphics::rect(x, y, kCell, kCell);

    // highlight border
    wasm96::Graphics::setColor(255, 255, 255, 60);
    wasm96::Graphics::rectOutline(x, y, kCell, kCell);
}

void drawField() {
    // Field background area
    int w = kCols * kCell;
    int h = kRowsVisible * kCell;

    wasm96::Graphics::setColor(10, 10, 40, 255);
    wasm96::Graphics::rect(kFieldX - kBorder, kFieldY - kBorder, w + 2 * kBorder, h + 2 * kBorder);

    wasm96::Graphics::setColor(kBorderC.r, kBorderC.g, kBorderC.b, kBorderC.a);
    wasm96::Graphics::rectOutline(kFieldX - kBorder, kFieldY - kBorder, w + 2 * kBorder, h + 2 * kBorder);

    // Grid
    wasm96::Graphics::setColor(kGrid.r, kGrid.g, kGrid.b, kGrid.a);
    for (int c = 1; c < kCols; c++) {
        int x = kFieldX + c * kCell;
        wasm96::Graphics::line(x, kFieldY, x, kFieldY + h);
    }
    for (int r = 1; r < kRowsVisible; r++) {
        int y = kFieldY + r * kCell;
        wasm96::Graphics::line(kFieldX, y, kFieldX + w, y);
    }
}

void drawLockedBlocks() {
    for (int r = 0; r < kRows; r++) {
        for (int c = 0; c < kCols; c++) {
            int8_t v = g.field[r][c];
            if (v < 0) continue;
            drawCell(c, r, kPieceColors[(int)v]);
        }
    }
}

void drawPieceGhost() {
    if (g.gameOver) return;

    int d = g.hardDropDistance();
    uint16_t m = kShapes[(int)g.curType][g.curRot & 3];

    Color base = kPieceColors[(int)g.curType];
    Color ghost = { (uint8_t)(base.r / 2), (uint8_t)(base.g / 2), (uint8_t)(base.b / 2), 90 };

    for (int r = 0; r < 4; r++) {
        for (int c = 0; c < 4; c++) {
            if (!shapeCell(m, r, c)) continue;
            int fx = g.curX + c;
            int fy = g.curY + r + d;
            drawCell(fx, fy, ghost);
        }
    }
}

void drawActivePiece() {
    if (g.gameOver) return;

    uint16_t m = kShapes[(int)g.curType][g.curRot & 3];
    Color col = kPieceColors[(int)g.curType];

    for (int r = 0; r < 4; r++) {
        for (int c = 0; c < 4; c++) {
            if (!shapeCell(m, r, c)) continue;
            int fx = g.curX + c;
            int fy = g.curY + r;
            drawCell(fx, fy, col);
        }
    }
}

void drawNextPiece() {
    wasm96::Graphics::setColor(kText.r, kText.g, kText.b, kText.a);
    wasm96::Graphics::textKey(kNextX, kFieldY, kHudFont, "NEXT");

    // Draw a small 4x4 preview box
    int box = 4 * kCell;
    wasm96::Graphics::setColor(10, 10, 40, 255);
    wasm96::Graphics::rect(kNextX - kBorder, kNextY - kBorder, box + 2 * kBorder, box + 2 * kBorder);
    wasm96::Graphics::setColor(kBorderC.r, kBorderC.g, kBorderC.b, kBorderC.a);
    wasm96::Graphics::rectOutline(kNextX - kBorder, kNextY - kBorder, box + 2 * kBorder, box + 2 * kBorder);

    uint16_t m = kShapes[(int)g.nextType][0];
    Color col = kPieceColors[(int)g.nextType];

    // Center-ish preview
    for (int r = 0; r < 4; r++) {
        for (int c = 0; c < 4; c++) {
            if (!shapeCell(m, r, c)) continue;
            int x = kNextX + c * kCell;
            int y = kNextY + r * kCell;
            wasm96::Graphics::setColor(kShadow.r, kShadow.g, kShadow.b, kShadow.a);
            wasm96::Graphics::rect(x + 2, y + 2, kCell, kCell);
            wasm96::Graphics::setColor(col.r, col.g, col.b, col.a);
            wasm96::Graphics::rect(x, y, kCell, kCell);
            wasm96::Graphics::setColor(255, 255, 255, 60);
            wasm96::Graphics::rectOutline(x, y, kCell, kCell);
        }
    }
}

void drawHud() {
    char buf[128];

    wasm96::Graphics::setColor(kText.r, kText.g, kText.b, kText.a);

    // Scoreboard panel background
    wasm96::Graphics::setColor(10, 10, 40, 255);
    wasm96::Graphics::rect(kHudX - 12, kFieldY - 4, 240, 360);
    wasm96::Graphics::setColor(kBorderC.r, kBorderC.g, kBorderC.b, kBorderC.a);
    wasm96::Graphics::rectOutline(kHudX - 12, kFieldY - 4, 240, 360);

    // (No sprintf include to keep minimal; do manual formatting)
    auto writeInt = [](char* dst, int v) -> char* {
        // returns end ptr (null-terminated)
        if (v == 0) {
            *dst++ = '0';
            *dst = '\0';
            return dst;
        }
        if (v < 0) { *dst++ = '-'; v = -v; }
        char tmp[16];
        int n = 0;
        while (v > 0 && n < 15) { tmp[n++] = char('0' + (v % 10)); v /= 10; }
        for (int i = n - 1; i >= 0; i--) *dst++ = tmp[i];
        *dst = '\0';
        return dst;
    };

    // Use the built-in Spleen font key (registered in setup at the desired size).
    const char* font = kHudFont;

    wasm96::Graphics::setColor(kText.r, kText.g, kText.b, kText.a);
    wasm96::Graphics::textKey(kHudX, kFieldY + 8, font, "SCOREBOARD");

    // "SCORE: "
    const char* s1 = "SCORE: ";
    int i = 0;
    for (; s1[i] != '\0'; i++) buf[i] = s1[i];
    char* p = buf + i;
    p = writeInt(p, g.score);
    wasm96::Graphics::textKey(kHudX, kFieldY + 40, font, buf);

    // "HIGH: "
    const char* sH = "HIGH: ";
    i = 0;
    for (; sH[i] != '\0'; i++) buf[i] = sH[i];
    p = buf + i;
    p = writeInt(p, g.highScore);
    wasm96::Graphics::textKey(kHudX, kFieldY + 64, font, buf);

    // "LINES: "
    const char* s2 = "LINES: ";
    i = 0;
    for (; s2[i] != '\0'; i++) buf[i] = s2[i];
    p = buf + i;
    p = writeInt(p, g.lines);
    wasm96::Graphics::textKey(kHudX, kFieldY + 96, font, buf);

    // "LEVEL: "
    const char* s3 = "LEVEL: ";
    i = 0;
    for (; s3[i] != '\0'; i++) buf[i] = s3[i];
    p = buf + i;
    p = writeInt(p, g.level);
    wasm96::Graphics::textKey(kHudX, kFieldY + 120, font, buf);

    // Controls
    wasm96::Graphics::textKey(kHudX, kFieldY + 160, font, "Controls:");
    wasm96::Graphics::textKey(kHudX, kFieldY + 180, font, "Left/Right: Move");
    wasm96::Graphics::textKey(kHudX, kFieldY + 200, font, "Down: Soft drop");
    wasm96::Graphics::textKey(kHudX, kFieldY + 220, font, "A/B: Rotate");
    wasm96::Graphics::textKey(kHudX, kFieldY + 240, font, "L1: Hard drop");
    wasm96::Graphics::textKey(kHudX, kFieldY + 260, font, "Start: Pause");
    wasm96::Graphics::textKey(kHudX, kFieldY + 280, font, "Select: Restart");

    if (g.paused) {
        wasm96::Graphics::setColor(255, 255, 255, 255);
        wasm96::Graphics::textKey(kFieldX, kFieldY + 200, font, "PAUSED");
    }
    if (g.gameOver) {
        wasm96::Graphics::setColor(255, 120, 120, 255);
        wasm96::Graphics::textKey(kFieldX, kFieldY + 180, font, "GAME OVER");
        wasm96::Graphics::setColor(kText.r, kText.g, kText.b, kText.a);
        wasm96::Graphics::textKey(kFieldX, kFieldY + 204, font, "Press Select to restart");
    }
}

} // namespace

extern "C" void setup() {
    wasm96::Graphics::setSize(kScreenW, kScreenH);
    wasm96::Graphics::setColor(255, 255, 255, 255);

    // Register the built-in Spleen font at the HUD size under the special key "spleen".
    // Text rendering depends on a registered font key.
    wasm96::Graphics::fontRegisterSpleen(kHudFont, kHudFontSize);

    // Seed from system millis if available
    g.reset((uint32_t)wasm96::System::millis());
    g.loadHighScore();

    // Ensure we start with a valid spawn
    if (!g.gameOver && g.collides(g.curType, g.curRot, g.curX, g.curY)) {
        g.reset(0xC0FFEEu);
        g.loadHighScore();
    }
}

extern "C" void update() {
    g.update();
}

extern "C" void draw() {
    wasm96::Graphics::background(kBg.r, kBg.g, kBg.b);

    drawField();
    drawLockedBlocks();
    drawPieceGhost();
    drawActivePiece();
    drawNextPiece();
    drawHud();

    // Title
    wasm96::Graphics::setColor(kText.r, kText.g, kText.b, kText.a);
    wasm96::Graphics::textKey(kFieldX, 10, kHudFont, "WASM96 Tetris (C++ guest)");
}