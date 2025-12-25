(module
  (import "env" "wasm96_graphics_set_size" (func $set_size (param i32 i32)))
  (import "env" "wasm96_graphics_background" (func $background (param i32 i32 i32)))
  (import "env" "wasm96_graphics_set_color" (func $set_color (param i32 i32 i32 i32)))
  (import "env" "wasm96_graphics_rect" (func $rect (param i32 i32 i32 i32)))
  (import "env" "wasm96_audio_init" (func $audio_init (param i32) (result i32)))
  (import "env" "wasm96_input_is_button_down" (func $is_button_down (param i32 i32) (result i32)))

  (global $rect_x (mut i32)
    (i32.const 10))
  (global $rect_y (mut i32)
    (i32.const 10))
  (global $color_r (mut i32)
    (i32.const 255))
  (global $color_g (mut i32)
    (i32.const 100))
  (global $color_b (mut i32)
    (i32.const 100))

  (func $setup
    i32.const 320
    i32.const 240
    call $set_size
    i32.const 44100
    call $audio_init
    drop)

  (func $update
    ;; move up
    i32.const 0
    i32.const 4
    call $is_button_down
    if
      global.get $rect_y
      i32.const 3
      i32.sub
      global.set $rect_y
    end

    ;; move down
    i32.const 0
    i32.const 5
    call $is_button_down
    if
      global.get $rect_y
      i32.const 3
      i32.add
      global.set $rect_y
    end

    ;; move left
    i32.const 0
    i32.const 6
    call $is_button_down
    if
      global.get $rect_x
      i32.const 3
      i32.sub
      global.set $rect_x
    end

    ;; move right
    i32.const 0
    i32.const 7
    call $is_button_down
    if
      global.get $rect_x
      i32.const 3
      i32.add
      global.set $rect_x
    end

    ;; clamp x
    global.get $rect_x
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      global.set $rect_x
    end
    global.get $rect_x
    i32.const 290
    i32.gt_s
    if
      i32.const 290
      global.set $rect_x
    end

    ;; clamp y
    global.get $rect_y
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      global.set $rect_y
    end
    global.get $rect_y
    i32.const 210
    i32.gt_s
    if
      i32.const 210
      global.set $rect_y
    end

    ;; change color with A (green)
    i32.const 0
    i32.const 8
    call $is_button_down
    if
      i32.const 100
      global.set $color_r
      i32.const 255
      global.set $color_g
      i32.const 100
      global.set $color_b
    else
      ;; change color with B (blue)
      i32.const 0
      i32.const 0
      call $is_button_down
      if
        i32.const 100
        global.set $color_r
        i32.const 100
        global.set $color_g
        i32.const 255
        global.set $color_b
      else
        ;; default red
        i32.const 255
        global.set $color_r
        i32.const 100
        global.set $color_g
        i32.const 100
        global.set $color_b
      end
    end)

  (func $draw
    ;; background
    i32.const 20
    i32.const 20
    i32.const 40
    call $background

    ;; set color
    global.get $color_r
    global.get $color_g
    global.get $color_b
    i32.const 255
    call $set_color

    ;; draw rect
    global.get $rect_x
    global.get $rect_y
    i32.const 30
    i32.const 30
    call $rect)

  (export "setup" (func $setup))
  (export "update" (func $update))
  (export "draw" (func $draw)))
