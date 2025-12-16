// wasm96/wasm96-go-sdk/wasm96.go
package wasm96

import (
	"unsafe"
)

// Button represents joypad button ids.
type Button uint32

const (
	ButtonB      Button = 0
	ButtonY      Button = 1
	ButtonSelect Button = 2
	ButtonStart  Button = 3
	ButtonUp     Button = 4
	ButtonDown   Button = 5
	ButtonLeft   Button = 6
	ButtonRight  Button = 7
	ButtonA      Button = 8
	ButtonX      Button = 9
	ButtonL1     Button = 10
	ButtonR1     Button = 11
	ButtonL2     Button = 12
	ButtonR2     Button = 13
	ButtonL3     Button = 14
	ButtonR3     Button = 15
)

// TextSize represents text dimensions.
type TextSize struct {
	Width  uint32
	Height uint32
}

// Low-level raw ABI imports.

//go:wasmimport env wasm96_graphics_set_size
func wasm96_graphics_set_size(width uint32, height uint32)

//go:wasmimport env wasm96_graphics_set_color
func wasm96_graphics_set_color(r uint32, g uint32, b uint32, a uint32)

//go:wasmimport env wasm96_graphics_background
func wasm96_graphics_background(r uint32, g uint32, b uint32)

//go:wasmimport env wasm96_graphics_point
func wasm96_graphics_point(x int32, y int32)

//go:wasmimport env wasm96_graphics_line
func wasm96_graphics_line(x1 int32, y1 int32, x2 int32, y2 int32)

//go:wasmimport env wasm96_graphics_rect
func wasm96_graphics_rect(x int32, y int32, w uint32, h uint32)

//go:wasmimport env wasm96_graphics_rect_outline
func wasm96_graphics_rect_outline(x int32, y int32, w uint32, h uint32)

//go:wasmimport env wasm96_graphics_circle
func wasm96_graphics_circle(x int32, y int32, r uint32)

//go:wasmimport env wasm96_graphics_circle_outline
func wasm96_graphics_circle_outline(x int32, y int32, r uint32)

//go:wasmimport env wasm96_graphics_image
func wasm96_graphics_image(x int32, y int32, w uint32, h uint32, dataPtr uintptr, dataLen uintptr)

//go:wasmimport env wasm96_graphics_triangle
func wasm96_graphics_triangle(x1 int32, y1 int32, x2 int32, y2 int32, x3 int32, y3 int32)

//go:wasmimport env wasm96_graphics_triangle_outline
func wasm96_graphics_triangle_outline(x1 int32, y1 int32, x2 int32, y2 int32, x3 int32, y3 int32)

//go:wasmimport env wasm96_graphics_bezier_quadratic
func wasm96_graphics_bezier_quadratic(x1 int32, y1 int32, cx int32, cy int32, x2 int32, y2 int32, segments uint32)

//go:wasmimport env wasm96_graphics_bezier_cubic
func wasm96_graphics_bezier_cubic(x1 int32, y1 int32, cx1 int32, cy1 int32, cx2 int32, cy2 int32, x2 int32, y2 int32, segments uint32)

//go:wasmimport env wasm96_graphics_pill
func wasm96_graphics_pill(x int32, y int32, w uint32, h uint32)

//go:wasmimport env wasm96_graphics_pill_outline
func wasm96_graphics_pill_outline(x int32, y int32, w uint32, h uint32)

//go:wasmimport env wasm96_graphics_svg_create
func wasm96_graphics_svg_create(dataPtr uintptr, dataLen uintptr) uint32

//go:wasmimport env wasm96_graphics_svg_draw
func wasm96_graphics_svg_draw(id uint32, x int32, y int32, w uint32, h uint32)

//go:wasmimport env wasm96_graphics_svg_destroy
func wasm96_graphics_svg_destroy(id uint32)

//go:wasmimport env wasm96_graphics_gif_create
func wasm96_graphics_gif_create(dataPtr uintptr, dataLen uintptr) uint32

//go:wasmimport env wasm96_graphics_gif_draw
func wasm96_graphics_gif_draw(id uint32, x int32, y int32)

//go:wasmimport env wasm96_graphics_gif_draw_scaled
func wasm96_graphics_gif_draw_scaled(id uint32, x int32, y int32, w uint32, h uint32)

//go:wasmimport env wasm96_graphics_gif_destroy
func wasm96_graphics_gif_destroy(id uint32)

//go:wasmimport env wasm96_graphics_font_upload_ttf
func wasm96_graphics_font_upload_ttf(dataPtr uintptr, dataLen uintptr) uint32

//go:wasmimport env wasm96_graphics_font_use_spleen
func wasm96_graphics_font_use_spleen(size uint32) uint32

//go:wasmimport env wasm96_graphics_text
func wasm96_graphics_text(x int32, y int32, font uint32, textPtr uintptr, textLen uintptr)

//go:wasmimport env wasm96_graphics_text_measure
func wasm96_graphics_text_measure(font uint32, textPtr uintptr, textLen uintptr) uint64

//go:wasmimport env wasm96_input_is_button_down
func wasm96_input_is_button_down(port uint32, btn uint32) uint32

//go:wasmimport env wasm96_input_is_key_down
func wasm96_input_is_key_down(key uint32) uint32

//go:wasmimport env wasm96_input_get_mouse_x
func wasm96_input_get_mouse_x() int32

//go:wasmimport env wasm96_input_get_mouse_y
func wasm96_input_get_mouse_y() int32

//go:wasmimport env wasm96_input_is_mouse_down
func wasm96_input_is_mouse_down(button uint32) uint32

//go:wasmimport env wasm96_audio_init
func wasm96_audio_init(sampleRate uint32) uint32

//go:wasmimport env wasm96_audio_push_samples
func wasm96_audio_push_samples(samplesPtr uintptr, samplesLen uintptr)

//go:wasmimport env wasm96_audio_play_wav
func wasm96_audio_play_wav(dataPtr uintptr, dataLen uintptr)

//go:wasmimport env wasm96_audio_play_qoa
func wasm96_audio_play_qoa(dataPtr uintptr, dataLen uintptr)

//go:wasmimport env wasm96_audio_play_xm
func wasm96_audio_play_xm(dataPtr uintptr, dataLen uintptr)

//go:wasmimport env wasm96_system_log
func wasm96_system_log(messagePtr uintptr, messageLen uintptr)

//go:wasmimport env wasm96_system_millis
func wasm96_system_millis() uint64

// Graphics API.
var Graphics = graphics{}

type graphics struct{}

// SetSize sets the screen dimensions.
func (g graphics) SetSize(width, height uint32) {
	wasm96_graphics_set_size(width, height)
}

// SetColor sets the current drawing color (RGBA).
func (g graphics) SetColor(r, green, b, a uint8) {
	wasm96_graphics_set_color(uint32(r), uint32(green), uint32(b), uint32(a))
}

// Background clears the screen with a specific color (RGB).
func (g graphics) Background(r, green, b uint8) {
	wasm96_graphics_background(uint32(r), uint32(green), uint32(b))
}

// Point draws a single pixel at (x, y).
func (g graphics) Point(x, y int32) {
	wasm96_graphics_point(x, y)
}

// Line draws a line from (x1, y1) to (x2, y2).
func (g graphics) Line(x1, y1, x2, y2 int32) {
	wasm96_graphics_line(x1, y1, x2, y2)
}

// Rect draws a filled rectangle.
func (g graphics) Rect(x, y int32, w, h uint32) {
	wasm96_graphics_rect(x, y, w, h)
}

// RectOutline draws a rectangle outline.
func (g graphics) RectOutline(x, y int32, w, h uint32) {
	wasm96_graphics_rect_outline(x, y, w, h)
}

// Circle draws a filled circle.
func (g graphics) Circle(x, y int32, r uint32) {
	wasm96_graphics_circle(x, y, r)
}

// CircleOutline draws a circle outline.
func (g graphics) CircleOutline(x, y int32, r uint32) {
	wasm96_graphics_circle_outline(x, y, r)
}

// Image draws an image/sprite.
func (g graphics) Image(x, y int32, w, h uint32, data []uint8) {
	wasm96_graphics_image(x, y, w, h, uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// Triangle draws a filled triangle.
func (g graphics) Triangle(x1, y1, x2, y2, x3, y3 int32) {
	wasm96_graphics_triangle(x1, y1, x2, y2, x3, y3)
}

// TriangleOutline draws a triangle outline.
func (g graphics) TriangleOutline(x1, y1, x2, y2, x3, y3 int32) {
	wasm96_graphics_triangle_outline(x1, y1, x2, y2, x3, y3)
}

// BezierQuadratic draws a quadratic Bezier curve.
func (g graphics) BezierQuadratic(x1, y1, cx, cy, x2, y2 int32, segments uint32) {
	wasm96_graphics_bezier_quadratic(x1, y1, cx, cy, x2, y2, segments)
}

// BezierCubic draws a cubic Bezier curve.
func (g graphics) BezierCubic(x1, y1, cx1, cy1, cx2, cy2, x2, y2 int32, segments uint32) {
	wasm96_graphics_bezier_cubic(x1, y1, cx1, cy1, cx2, cy2, x2, y2, segments)
}

// Pill draws a filled pill.
func (g graphics) Pill(x, y int32, w, h uint32) {
	wasm96_graphics_pill(x, y, w, h)
}

// PillOutline draws a pill outline.
func (g graphics) PillOutline(x, y int32, w, h uint32) {
	wasm96_graphics_pill_outline(x, y, w, h)
}

// SvgCreate creates an SVG resource.
func (g graphics) SvgCreate(data []uint8) uint32 {
	return wasm96_graphics_svg_create(uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// SvgDraw draws an SVG resource.
func (g graphics) SvgDraw(id uint32, x, y int32, w, h uint32) {
	wasm96_graphics_svg_draw(id, x, y, w, h)
}

// SvgDestroy destroys an SVG resource.
func (g graphics) SvgDestroy(id uint32) {
	wasm96_graphics_svg_destroy(id)
}

// GifCreate creates a GIF resource.
func (g graphics) GifCreate(data []uint8) uint32 {
	return wasm96_graphics_gif_create(uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// GifDraw draws a GIF resource at natural size.
func (g graphics) GifDraw(id uint32, x, y int32) {
	wasm96_graphics_gif_draw(id, x, y)
}

// GifDrawScaled draws a GIF resource scaled.
func (g graphics) GifDrawScaled(id uint32, x, y int32, w, h uint32) {
	wasm96_graphics_gif_draw_scaled(id, x, y, w, h)
}

// GifDestroy destroys a GIF resource.
func (g graphics) GifDestroy(id uint32) {
	wasm96_graphics_gif_destroy(id)
}

// FontUploadTtf uploads a TTF font.
func (g graphics) FontUploadTtf(data []uint8) uint32 {
	return wasm96_graphics_font_upload_ttf(uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// FontUseSpleen uses a built-in Spleen font.
func (g graphics) FontUseSpleen(size uint32) uint32 {
	return wasm96_graphics_font_use_spleen(size)
}

// Text draws text.
func (g graphics) Text(x, y int32, font uint32, text string) {
	data := []byte(text)
	wasm96_graphics_text(x, y, font, uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// TextMeasure measures text.
func (g graphics) TextMeasure(font uint32, text string) TextSize {
	data := []byte(text)
	packed := wasm96_graphics_text_measure(font, uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
	return TextSize{
		Width:  uint32(packed >> 32),
		Height: uint32(packed & 0xFFFFFFFF),
	}
}

// Input API.
var Input = input{}

type input struct{}

// IsButtonDown returns true if the specified button is currently held down.
func (i input) IsButtonDown(port uint32, btn Button) bool {
	return wasm96_input_is_button_down(port, uint32(btn)) != 0
}

// IsKeyDown returns true if the specified key is currently held down.
func (i input) IsKeyDown(key uint32) bool {
	return wasm96_input_is_key_down(key) != 0
}

// GetMouseX gets current mouse X position.
func (i input) GetMouseX() int32 {
	return wasm96_input_get_mouse_x()
}

// GetMouseY gets current mouse Y position.
func (i input) GetMouseY() int32 {
	return wasm96_input_get_mouse_y()
}

// IsMouseDown returns true if the specified mouse button is held down.
func (i input) IsMouseDown(button uint32) bool {
	return wasm96_input_is_mouse_down(button) != 0
}

// Audio API.
var Audio = audio{}

type audio struct{}

// Init initializes audio system.
func (a audio) Init(sampleRate uint32) uint32 {
	return wasm96_audio_init(sampleRate)
}

// PushSamples pushes a chunk of audio samples.
func (a audio) PushSamples(samples []int16) {
	wasm96_audio_push_samples(uintptr(unsafe.Pointer(&samples[0])), uintptr(len(samples)))
}

// PlayWav plays a WAV file.
func (a audio) PlayWav(data []uint8) {
	wasm96_audio_play_wav(uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// PlayQoa plays a QOA file.
func (a audio) PlayQoa(data []uint8) {
	wasm96_audio_play_qoa(uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// PlayXm plays an XM file.
func (a audio) PlayXm(data []uint8) {
	wasm96_audio_play_xm(uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// System API.
var System = system{}

type system struct{}

// Log logs a message to the host console.
func (s system) Log(message string) {
	data := []byte(message)
	wasm96_system_log(uintptr(unsafe.Pointer(&data[0])), uintptr(len(data)))
}

// Millis gets the number of milliseconds since the app started.
func (s system) Millis() uint64 {
	return wasm96_system_millis()
}
