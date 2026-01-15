const std = @import("std");
const wasm96 = @import("wasm96");

const PADDLE_WIDTH = 10;
const PADDLE_HEIGHT = 80;
const BALL_SIZE = 10;
const PADDLE_SPEED = 5;
const BALL_SPEED_X = 3;
const BALL_SPEED_Y = 2;
const PADDLE_ACCEL = 1.5;
const PADDLE_MAX_SPEED = 10.0;
const PADDLE_FRICTION = 0.8;
const AI_REACTION_DELAY = 5; // frames

fn getScoreStr(score: u32) []const u8 {
    return switch (score) {
        0 => "0",
        1 => "15",
        2 => "30",
        3 => "40",
        else => "Game",
    };
}

const GameState = enum {
    menu,
    playing_1p,
    playing_2p,
    game_over,
};

var game_state: GameState = .menu;
var menu_selection: u32 = 0; // 0: 1 player, 1: 2 players
var game_over_selection: u32 = 0; // 0: replay, 1: main menu
var winner_left: bool = false;
var current_mode: GameState = .playing_1p;

var left_paddle_y: i32 = 200;
var right_paddle_y: i32 = 200;
var left_paddle_vy: f32 = 0.0;
var right_paddle_vy: f32 = 0.0;
var ai_target_y: i32 = 200;
var ai_delay_counter: u32 = 0;
var ball_direction: i32 = 1;
var ball_x: i32 = 320 - BALL_SIZE / 2;
var ball_y: i32 = 240 - BALL_SIZE / 2;
var ball_vx: i32 = BALL_SPEED_X;
var ball_vy: i32 = BALL_SPEED_Y;
var left_score: u32 = 0;
var right_score: u32 = 0;
var left_games: u32 = 0;
var right_games: u32 = 0;

export fn setup() void {
    wasm96.graphics.setSize(640, 480);
    _ = wasm96.graphics.fontRegisterSpleen("font/spleen/16", 16);
    _ = wasm96.graphics.fontRegisterSpleen("font/spleen/24", 24);
}

fn resetBall() void {
    ball_x = 320 - BALL_SIZE / 2;
    ball_y = 240 - BALL_SIZE / 2;
    ball_vx = BALL_SPEED_X * ball_direction;
    ball_vy = BALL_SPEED_Y;
    ball_direction = -ball_direction;
}

fn resetGame() void {
    left_paddle_y = 200;
    right_paddle_y = 200;
    left_paddle_vy = 0.0;
    right_paddle_vy = 0.0;
    ai_target_y = 200;
    ai_delay_counter = 0;
    left_score = 0;
    right_score = 0;
    left_games = 0;
    right_games = 0;
    resetBall();
}

export fn update() void {
    switch (game_state) {
        .menu => {
            // Menu navigation
            if (wasm96.input.isButtonDown(0, .up) and menu_selection > 0) {
                menu_selection -= 1;
            }
            if (wasm96.input.isButtonDown(0, .down) and menu_selection < 1) {
                menu_selection += 1;
            }
            // Select
            if (wasm96.input.isButtonDown(0, .a)) {
                if (menu_selection == 0) {
                    game_state = .playing_1p;
                    current_mode = .playing_1p;
                } else {
                    game_state = .playing_2p;
                    current_mode = .playing_2p;
                }
                resetGame();
            }
        },
        .playing_1p, .playing_2p => {
            // Move left paddle with acceleration
            if (wasm96.input.isButtonDown(0, .up)) {
                left_paddle_vy -= PADDLE_ACCEL;
            }
            if (wasm96.input.isButtonDown(0, .down)) {
                left_paddle_vy += PADDLE_ACCEL;
            }
            left_paddle_vy *= PADDLE_FRICTION;
            if (left_paddle_vy > PADDLE_MAX_SPEED) left_paddle_vy = PADDLE_MAX_SPEED;
            if (left_paddle_vy < -PADDLE_MAX_SPEED) left_paddle_vy = -PADDLE_MAX_SPEED;
            left_paddle_y += @as(i32, @intFromFloat(left_paddle_vy));
            // Clamp left paddle
            if (left_paddle_y < 0) {
                left_paddle_y = 0;
                left_paddle_vy = 0.0;
            }
            if (left_paddle_y > 480 - PADDLE_HEIGHT) {
                left_paddle_y = 480 - PADDLE_HEIGHT;
                left_paddle_vy = 0.0;
            }

            // Move right paddle
            if (game_state == .playing_2p) {
                if (wasm96.input.isButtonDown(1, .up)) {
                    right_paddle_vy -= PADDLE_ACCEL;
                }
                if (wasm96.input.isButtonDown(1, .down)) {
                    right_paddle_vy += PADDLE_ACCEL;
                }
                right_paddle_vy *= PADDLE_FRICTION;
                if (right_paddle_vy > PADDLE_MAX_SPEED) right_paddle_vy = PADDLE_MAX_SPEED;
                if (right_paddle_vy < -PADDLE_MAX_SPEED) right_paddle_vy = -PADDLE_MAX_SPEED;
                right_paddle_y += @as(i32, @intFromFloat(right_paddle_vy));
            } else {
                // AI for right paddle with delay and smoothing
                ai_delay_counter += 1;
                if (ai_delay_counter >= AI_REACTION_DELAY) {
                    ai_target_y = ball_y + BALL_SIZE / 2 - PADDLE_HEIGHT / 2;
                    ai_delay_counter = 0;
                }
                const diff = ai_target_y - right_paddle_y;
                if (diff > 0) {
                    right_paddle_vy += PADDLE_ACCEL;
                } else if (diff < 0) {
                    right_paddle_vy -= PADDLE_ACCEL;
                }
                right_paddle_vy *= PADDLE_FRICTION;
                if (right_paddle_vy > PADDLE_MAX_SPEED) right_paddle_vy = PADDLE_MAX_SPEED;
                if (right_paddle_vy < -PADDLE_MAX_SPEED) right_paddle_vy = -PADDLE_MAX_SPEED;
                right_paddle_y += @as(i32, @intFromFloat(right_paddle_vy));
            }
            // Clamp right paddle
            if (right_paddle_y < 0) {
                right_paddle_y = 0;
                right_paddle_vy = 0.0;
            }
            if (right_paddle_y > 480 - PADDLE_HEIGHT) {
                right_paddle_y = 480 - PADDLE_HEIGHT;
                right_paddle_vy = 0.0;
            }

            // Move ball
            ball_x += ball_vx;
            ball_y += ball_vy;

            // Bounce off top and bottom
            if (ball_y <= 0 or ball_y >= 480 - BALL_SIZE) {
                ball_vy = -ball_vy;
            }

            // Check collision with left paddle
            if (ball_x <= 20 + PADDLE_WIDTH and ball_x + BALL_SIZE > 20 and
                ball_y + BALL_SIZE > left_paddle_y and ball_y < left_paddle_y + PADDLE_HEIGHT)
            {
                ball_vx = -ball_vx;
                // Adjust vertical velocity based on hit position
                const hit_pos = ball_y + BALL_SIZE / 2 - left_paddle_y - PADDLE_HEIGHT / 2;
                ball_vy += @as(i32, @intFromFloat(@as(f32, @floatFromInt(hit_pos)) / 10.0));
                // Clamp ball vy
                if (ball_vy > 8) ball_vy = 8;
                if (ball_vy < -8) ball_vy = -8;
            }

            // Check collision with right paddle
            if (ball_x + BALL_SIZE >= 610 and ball_x < 610 + PADDLE_WIDTH and
                ball_y + BALL_SIZE > right_paddle_y and ball_y < right_paddle_y + PADDLE_HEIGHT)
            {
                ball_vx = -ball_vx;
                // Adjust vertical velocity based on hit position
                const hit_pos = ball_y + BALL_SIZE / 2 - right_paddle_y - PADDLE_HEIGHT / 2;
                ball_vy += @as(i32, @intFromFloat(@as(f32, @floatFromInt(hit_pos)) / 10.0));
                // Clamp ball vy
                if (ball_vy > 8) ball_vy = 8;
                if (ball_vy < -8) ball_vy = -8;
            }

            // Score
            if (ball_x < 0) {
                right_score += 1;
                if (right_score >= 4) {
                    right_games += 1;
                    if (right_games >= 3) {
                        winner_left = false;
                        game_state = .game_over;
                    } else {
                        left_score = 0;
                        right_score = 0;
                    }
                }
                resetBall();
            }
            if (ball_x > 640) {
                left_score += 1;
                if (left_score >= 4) {
                    left_games += 1;
                    if (left_games >= 3) {
                        winner_left = true;
                        game_state = .game_over;
                    } else {
                        left_score = 0;
                        right_score = 0;
                    }
                }
                resetBall();
            }

            // Return to menu on start press
            if (wasm96.input.isButtonDown(0, .start)) {
                game_state = .menu;
            }
        },
        .game_over => {
            // Menu navigation
            if (wasm96.input.isButtonDown(0, .up) and game_over_selection > 0) {
                game_over_selection -= 1;
            }
            if (wasm96.input.isButtonDown(0, .down) and game_over_selection < 1) {
                game_over_selection += 1;
            }
            // Select
            if (wasm96.input.isButtonDown(0, .a)) {
                if (game_over_selection == 0) {
                    // Replay
                    resetGame();
                    game_state = current_mode;
                } else {
                    // Main menu
                    game_state = .menu;
                }
            }
        },
    }
}

export fn draw() void {
    wasm96.graphics.background(0, 0, 0); // Black background
    wasm96.graphics.setColor(255, 255, 255, 255); // White

    switch (game_state) {
        .menu => {
            wasm96.graphics.textKey(200, 150, "font/spleen/24", "Pong Menu");
            wasm96.graphics.textKey(200, 200, "font/spleen/16", "1 Player");
            wasm96.graphics.textKey(200, 230, "font/spleen/16", "2 Players");
            if (menu_selection == 0) {
                wasm96.graphics.textKey(170, 200, "font/spleen/16", ">");
            } else {
                wasm96.graphics.textKey(170, 230, "font/spleen/16", ">");
            }
            wasm96.graphics.textKey(150, 300, "font/spleen/16", "Press A to select");
        },
        .playing_1p, .playing_2p => {
            // Draw paddles
            wasm96.graphics.rect(20, left_paddle_y, PADDLE_WIDTH, PADDLE_HEIGHT);
            wasm96.graphics.rect(610, right_paddle_y, PADDLE_WIDTH, PADDLE_HEIGHT);

            // Draw ball
            wasm96.graphics.rect(ball_x, ball_y, BALL_SIZE, BALL_SIZE);

            // Draw scores
            const left_str = getScoreStr(left_score);
            const right_str = getScoreStr(right_score);
            wasm96.graphics.textKey(200, 20, "font/spleen/16", left_str);
            wasm96.graphics.textKey(400, 20, "font/spleen/16", right_str);

            // Draw games
            var left_game_buf: [10]u8 = undefined;
            var right_game_buf: [10]u8 = undefined;
            const left_game_str = std.fmt.bufPrint(&left_game_buf, "{}", .{left_games}) catch "0";
            const right_game_str = std.fmt.bufPrint(&right_game_buf, "{}", .{right_games}) catch "0";
            wasm96.graphics.textKey(180, 40, "font/spleen/16", left_game_str);
            wasm96.graphics.textKey(380, 40, "font/spleen/16", right_game_str);

            // Draw mode
            if (game_state == .playing_1p) {
                wasm96.graphics.textKey(280, 20, "font/spleen/16", "1P");
            } else {
                wasm96.graphics.textKey(280, 20, "font/spleen/16", "2P");
            }

            // Match point
            if (left_score == 3 and right_score < 3) {
                wasm96.graphics.textKey(250, 60, "font/spleen/16", "Match Point Left");
            } else if (right_score == 3 and left_score < 3) {
                wasm96.graphics.textKey(250, 60, "font/spleen/16", "Match Point Right");
            }
        },
        .game_over => {
            const winner_str = if (winner_left) "Left Wins!" else "Right Wins!";
            wasm96.graphics.textKey(250, 150, "font/spleen/24", winner_str);
            wasm96.graphics.textKey(250, 200, "font/spleen/16", "Replay");
            wasm96.graphics.textKey(250, 230, "font/spleen/16", "Main Menu");
            if (game_over_selection == 0) {
                wasm96.graphics.textKey(220, 200, "font/spleen/16", ">");
            } else {
                wasm96.graphics.textKey(220, 230, "font/spleen/16", ">");
            }
            wasm96.graphics.textKey(200, 280, "font/spleen/16", "Press A to select");
        },
    }
}
