//! Audio/Video implementation for wasm96-core (Immediate Mode).
//!
//! This module implements the host-side drawing commands and audio handling.
//!
//! - Graphics: The host maintains a `Vec<u32>` framebuffer (XRGB8888).
//!   Guest commands modify this buffer.
//!   `video_present_host` sends it to libretro.
//!
//! - Audio: The host maintains a `Vec<i16>` sample queue.
//!   Guest pushes samples; host drains them to libretro.

use crate::state::global;
use wasmer::FunctionEnvMut;

/// Errors from AV operations.
#[derive(Debug)]
pub enum AvError {
    MissingMemory,
    MemoryReadFailed,
}

// --- Graphics ---

/// Set the screen dimensions. Resizes the host framebuffer.
pub fn graphics_set_size(width: u32, height: u32) {
    if width == 0 || height == 0 {
        return;
    }
    let mut s = global().lock().unwrap();
    s.video.width = width;
    s.video.height = height;
    s.video.framebuffer.resize((width * height) as usize, 0);
    // Clear to black on resize
    s.video.framebuffer.fill(0);
}

/// Set the current drawing color.
pub fn graphics_set_color(r: u32, g: u32, b: u32, _a: u32) {
    let mut s = global().lock().unwrap();
    // Pack as 0x00RRGGBB (XRGB8888). We ignore Alpha for the framebuffer format usually,
    // but we might use it for blending later. For now, simple overwrite.
    // Libretro XRGB8888 expects 0x00RRGGBB.
    let color = ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    s.video.draw_color = color;
}

/// Clear the screen to a specific color.
pub fn graphics_background(r: u32, g: u32, b: u32) {
    let mut s = global().lock().unwrap();
    let color = ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    s.video.framebuffer.fill(color);
}

/// Draw a single pixel.
pub fn graphics_point(x: i32, y: i32) {
    let mut s = global().lock().unwrap();
    let w = s.video.width as i32;
    let h = s.video.height as i32;

    if x >= 0 && x < w && y >= 0 && y < h {
        let idx = (y * w + x) as usize;
        s.video.framebuffer[idx] = s.video.draw_color;
    }
}

/// Draw a line using Bresenham's algorithm.
pub fn graphics_line(mut x0: i32, mut y0: i32, x1: i32, y1: i32) {
    let mut s = global().lock().unwrap();
    let w = s.video.width as i32;
    let h = s.video.height as i32;
    let color = s.video.draw_color;
    let fb = &mut s.video.framebuffer;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
            fb[(y0 * w + x0) as usize] = color;
        }

        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Draw a filled rectangle.
pub fn graphics_rect(x: i32, y: i32, w: u32, h: u32) {
    let mut s = global().lock().unwrap();
    let screen_w = s.video.width as i32;
    let screen_h = s.video.height as i32;
    let color = s.video.draw_color;

    let x_start = x.max(0);
    let y_start = y.max(0);
    let x_end = (x + w as i32).min(screen_w);
    let y_end = (y + h as i32).min(screen_h);

    if x_start >= x_end || y_start >= y_end {
        return;
    }

    let fb_w = s.video.width as usize;
    let fb = &mut s.video.framebuffer;

    for curr_y in y_start..y_end {
        let start_idx = (curr_y as usize) * fb_w + (x_start as usize);
        let end_idx = (curr_y as usize) * fb_w + (x_end as usize);
        fb[start_idx..end_idx].fill(color);
    }
}

/// Draw a rectangle outline.
pub fn graphics_rect_outline(x: i32, y: i32, w: u32, h: u32) {
    // Top
    graphics_line_internal(x, y, x + w as i32, y);
    // Bottom
    graphics_line_internal(x, y + h as i32, x + w as i32, y + h as i32);
    // Left
    graphics_line_internal(x, y, x, y + h as i32);
    // Right
    graphics_line_internal(x + w as i32, y, x + w as i32, y + h as i32);
}

/// Helper for internal line drawing without locking every pixel (if we had a way to pass the lock).
/// Since we don't want to complicate locking, we'll just call the public one which locks.
/// It's slightly inefficient but fine for this scale.
/// Actually, `graphics_rect_outline` calls `graphics_line` 4 times, so 4 locks. Acceptable.
fn graphics_line_internal(x1: i32, y1: i32, x2: i32, y2: i32) {
    graphics_line(x1, y1, x2, y2);
}

/// Draw a filled circle.
pub fn graphics_circle(cx: i32, cy: i32, r: u32) {
    let mut s = global().lock().unwrap();
    let w = s.video.width as i32;
    let h = s.video.height as i32;
    let color = s.video.draw_color;
    let fb = &mut s.video.framebuffer;

    let r_sq = (r * r) as i32;
    let r_i32 = r as i32;

    let x_min = (cx - r_i32).max(0);
    let x_max = (cx + r_i32).min(w);
    let y_min = (cy - r_i32).max(0);
    let y_max = (cy + r_i32).min(h);

    for y in y_min..y_max {
        for x in x_min..x_max {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r_sq {
                fb[(y * w + x) as usize] = color;
            }
        }
    }
}

/// Draw a circle outline (Bresenham's circle algorithm).
pub fn graphics_circle_outline(cx: i32, cy: i32, r: u32) {
    let mut s = global().lock().unwrap();
    let w = s.video.width as i32;
    let h = s.video.height as i32;
    let color = s.video.draw_color;
    let fb = &mut s.video.framebuffer;

    let mut x = 0;
    let mut y = r as i32;
    let mut d = 3 - 2 * r as i32;

    let mut plot = |x: i32, y: i32| {
        if x >= 0 && x < w && y >= 0 && y < h {
            fb[(y * w + x) as usize] = color;
        }
    };

    while y >= x {
        plot(cx + x, cy + y);
        plot(cx - x, cy + y);
        plot(cx + x, cy - y);
        plot(cx - x, cy - y);
        plot(cx + y, cy + x);
        plot(cx - y, cy + x);
        plot(cx + y, cy - x);
        plot(cx - y, cy - x);

        x += 1;
        if d > 0 {
            y -= 1;
            d = d + 4 * (x - y) + 10;
        } else {
            d = d + 4 * x + 6;
        }
    }
}

/// Draw an image from guest memory.
/// `ptr` points to RGBA bytes (4 bytes per pixel).
pub fn graphics_image(
    env: &FunctionEnvMut<()>,
    x: i32,
    y: i32,
    img_w: u32,
    img_h: u32,
    ptr: u32,
    len: u32,
) -> Result<(), AvError> {
    // Basic validation
    let expected_len = img_w.checked_mul(img_h).and_then(|s| s.checked_mul(4));
    if let Some(req) = expected_len {
        if len < req {
            // Not enough data provided
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Read guest memory
    let memory_ptr = {
        let s = global().lock().unwrap();
        s.memory
    };
    if memory_ptr.is_null() {
        return Err(AvError::MissingMemory);
    }

    // SAFETY: memory pointer checked.
    let mem = unsafe { &*memory_ptr };
    let view = mem.view(env);

    // We read the whole image into a temp buffer.
    // Optimization: could read row-by-row to avoid large allocation,
    // but for retro resolutions this is fine.
    let mut img_data = vec![0u8; len as usize];
    view.read(ptr as u64, &mut img_data)
        .map_err(|_| AvError::MemoryReadFailed)?;

    // Lock and draw
    let mut s = global().lock().unwrap();
    let screen_w = s.video.width as i32;
    let screen_h = s.video.height as i32;
    let fb = &mut s.video.framebuffer;

    // Clipping
    let x_start = x.max(0);
    let y_start = y.max(0);
    let x_end = (x + img_w as i32).min(screen_w);
    let y_end = (y + img_h as i32).min(screen_h);

    if x_start >= x_end || y_start >= y_end {
        return Ok(());
    }

    for curr_y in y_start..y_end {
        let src_y = curr_y - y; // relative to image
        let src_row_start = (src_y as usize) * (img_w as usize) * 4;

        let dst_row_start = (curr_y as usize) * (screen_w as usize);

        for curr_x in x_start..x_end {
            let src_x = curr_x - x; // relative to image
            let src_idx = src_row_start + (src_x as usize) * 4;

            let r = img_data[src_idx];
            let g = img_data[src_idx + 1];
            let b = img_data[src_idx + 2];
            let a = img_data[src_idx + 3];

            if a > 0 {
                // Simple alpha check (0 = transparent, >0 = opaque).
                // Real blending would be: result = alpha * src + (1-alpha) * dst
                // For now, just overwrite if not fully transparent.
                let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                fb[dst_row_start + (curr_x as usize)] = color;
            }
        }
    }

    Ok(())
}

/// Present the framebuffer to libretro.
pub fn video_present_host() {
    let (handle_ptr, _width, _height, fb) = {
        let s = global().lock().unwrap();
        (
            s.handle,
            s.video.width,
            s.video.height,
            s.video.framebuffer.clone(),
        )
    };

    if handle_ptr.is_null() {
        return;
    }

    // Convert Vec<u32> to &[u8] for libretro.
    // XRGB8888 is 4 bytes per pixel.
    // We can cast the slice safely because the layout is compatible (little endian).
    let data_ptr = fb.as_ptr() as *const u8;
    let data_len = fb.len() * 4;
    let data_slice = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };

    // SAFETY: handle pointer checked.
    let h = unsafe { &mut *handle_ptr };
    h.upload_video_frame(data_slice);
}

// --- Audio ---

pub fn audio_init(sample_rate: u32) -> u32 {
    let mut s = global().lock().unwrap();
    s.audio.sample_rate = sample_rate;
    // Return buffer size hint (e.g. 1 frame worth? or just 0).
    // The guest doesn't strictly need this if it pushes what it wants.
    1024
}

pub fn audio_push_samples(env: &FunctionEnvMut<()>, ptr: u32, count: u32) -> Result<(), AvError> {
    let memory_ptr = {
        let s = global().lock().unwrap();
        s.memory
    };

    if memory_ptr.is_null() {
        return Err(AvError::MissingMemory);
    }

    // SAFETY: memory pointer checked.
    let mem = unsafe { &*memory_ptr };
    let view = mem.view(env);

    // Read i16 samples. count is number of i16 elements.
    let byte_len = count.checked_mul(2).ok_or(AvError::MemoryReadFailed)?;
    let mut tmp_bytes = vec![0u8; byte_len as usize];

    view.read(ptr as u64, &mut tmp_bytes)
        .map_err(|_| AvError::MemoryReadFailed)?;

    // Convert bytes to i16
    let mut samples = Vec::with_capacity(count as usize);
    for chunk in tmp_bytes.chunks_exact(2) {
        let val = i16::from_le_bytes([chunk[0], chunk[1]]);
        samples.push(val);
    }

    // Append to host queue
    let mut s = global().lock().unwrap();
    s.audio.host_queue.extend(samples);

    Ok(())
}

pub fn audio_drain_host(max_frames: u32) -> u32 {
    let (handle_ptr, sample_rate) = {
        let s = global().lock().unwrap();
        (s.handle, s.audio.sample_rate)
    };

    if handle_ptr.is_null() {
        return 0;
    }

    // We must upload a minimum amount of audio each frame to satisfy libretro-backend.
    // The backend requires at least ~1 frame worth of stereo samples; for 44.1kHz @ 60fps:
    // 44100 / 60 = 735 frames (per channel) => 1470 i16 samples interleaved stereo.
    //
    // Even if the guest provides no audio (or too little), we pad with silence so the core
    // never panics due to insufficient audio uploads.
    let min_samples_per_run: usize = ((sample_rate as usize) / 60) * 2;

    // Stereo = 2 i16 samples per audio frame (L, R)
    let samples_per_frame: usize = 2;

    let mut drained: Vec<i16> = {
        let mut s = global().lock().unwrap();

        let available_samples = s.audio.host_queue.len();
        let available_frames = available_samples / samples_per_frame;

        let frames_to_take = if max_frames == 0 {
            available_frames
        } else {
            available_frames.min(max_frames as usize)
        };

        let samples_to_take = frames_to_take * samples_per_frame;

        if samples_to_take == 0 {
            Vec::new()
        } else {
            s.audio.host_queue.drain(0..samples_to_take).collect()
        }
    };

    // Pad with silence if guest produced too few samples this run.
    if drained.len() < min_samples_per_run {
        drained.resize(min_samples_per_run, 0i16);
    }

    // SAFETY: handle pointer checked.
    let h = unsafe { &mut *handle_ptr };
    h.upload_audio_frame(&drained);

    // Report how many *audio frames* we uploaded after padding (stereo frames).
    (drained.len() / samples_per_frame) as u32
}
