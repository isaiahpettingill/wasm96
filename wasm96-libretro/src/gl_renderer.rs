//! Minimal GL compositor used by the libretro frontend.
//!
//! This module uploads the engine's 2D software framebuffer into a texture and draws it
//! as a full-screen overlay into the libretro-provided HW framebuffer (FBO).
//!
//! Important: This intentionally matches the compositing approach used by `wasm96-engine`
//! (shader-generated fullscreen triangle strip via `gl_VertexID`) to avoid quad-UV/origin
//! mismatches that can manifest as upside-down or duplicated/mirrored output.

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

const OVERLAY_TEX_UNIT: u32 = 0;

pub struct GlRenderer {
    program_overlay: u32,
    overlay_vao: u32,
    overlay_texture: u32,
    overlay_texture_size: (u32, u32),
    overlay_upload_rgba: Vec<u8>,
    output_fbo: u32,
}

static GL_RENDERER: OnceLock<Mutex<Option<GlRenderer>>> = OnceLock::new();

const VS_OVERLAY_SRC_GL: &str = r#"
#version 150 core
// Fullscreen triangle strip generated in shader
const vec2 verts[4] = vec2[](vec2(-1,-1), vec2(1,-1), vec2(-1,1), vec2(1,1));
const vec2 uvs[4]   = vec2[](vec2(0,1),   vec2(1,1),  vec2(0,0),  vec2(1,0));

out vec2 v_uv;

void main() {
    gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
    v_uv = uvs[gl_VertexID];
}
"#;

const FS_OVERLAY_SRC_GL: &str = r#"
#version 150 core
in vec2 v_uv;
uniform sampler2D tex;
out vec4 fragColor;

void main() {
    fragColor = texture(tex, v_uv);
}
"#;

const VS_OVERLAY_SRC_GLES: &str = r#"
#version 300 es
// Fullscreen triangle strip generated in shader (gl_VertexID is available in ES 3.0)
const vec2 verts[4] = vec2[](vec2(-1.0,-1.0), vec2(1.0,-1.0), vec2(-1.0,1.0), vec2(1.0,1.0));
const vec2 uvs[4]   = vec2[](vec2(0.0,1.0),   vec2(1.0,1.0),   vec2(0.0,0.0),  vec2(1.0,0.0));

out vec2 v_uv;

void main() {
    gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
    v_uv = uvs[gl_VertexID];
}
"#;

const FS_OVERLAY_SRC_GLES: &str = r#"
#version 300 es
precision mediump float;
in vec2 v_uv;
uniform sampler2D tex;
out vec4 fragColor;

void main() {
    fragColor = texture(tex, v_uv);
}
"#;

fn overlay_shader_sources(use_gles: bool) -> (&'static str, &'static str) {
    if use_gles {
        (VS_OVERLAY_SRC_GLES, FS_OVERLAY_SRC_GLES)
    } else {
        (VS_OVERLAY_SRC_GL, FS_OVERLAY_SRC_GL)
    }
}

impl GlRenderer {
    pub fn init(use_gles: bool) -> Result<Self, String> {
        unsafe {
            let (vs_src, fs_src) = overlay_shader_sources(use_gles);
            let program_overlay = create_program(vs_src, fs_src)?;

            // Create overlay texture
            let mut overlay_texture = 0u32;
            gl::GenTextures(1, &mut overlay_texture);
            gl::BindTexture(gl::TEXTURE_2D, overlay_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            // VAO is required on core profiles; we don't need any attributes because the shader
            // generates the fullscreen strip from `gl_VertexID`.
            let mut overlay_vao = 0u32;
            gl::GenVertexArrays(1, &mut overlay_vao);
            gl::BindVertexArray(overlay_vao);
            gl::BindVertexArray(0);

            Ok(Self {
                program_overlay,
                overlay_vao,
                overlay_texture,
                overlay_texture_size: (0, 0),
                overlay_upload_rgba: Vec::new(),
                output_fbo: 0,
            })
        }
    }

    pub fn prepare_frame(&mut self, fbo: u32, width: u32, height: u32) {
        self.output_fbo = fbo;
        if fbo == 0 {
            return;
        }
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::Viewport(0, 0, width as i32, height as i32);
        }
    }

    /// Upload framebuffer and composite to FBO.
    pub fn composite_frame(
        &mut self,
        framebuffer: &[u32],
        width: u32,
        height: u32,
        stride_pixels: u32,
    ) -> bool {
        if self.output_fbo == 0 {
            return false;
        }
        if width == 0 || height == 0 {
            return true;
        }

        unsafe {
            // Ensure output target + viewport.
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.output_fbo);
            gl::Viewport(0, 0, width as i32, height as i32);

            // Upload framebuffer to texture (repack XRGB/ARGB u32 -> tightly packed RGBA8).
            gl::ActiveTexture(gl::TEXTURE0 + OVERLAY_TEX_UNIT);
            gl::BindTexture(gl::TEXTURE_2D, self.overlay_texture);

            let stride_pixels = stride_pixels as usize;
            let width_usize = width as usize;
            let height_usize = height as usize;

            let needed_len = width_usize * height_usize * 4;
            if self.overlay_upload_rgba.len() < needed_len {
                self.overlay_upload_rgba.resize(needed_len, 0);
            }

            let mut out_i = 0usize;
            for y in 0..height_usize {
                let row = &framebuffer[(y * stride_pixels)..(y * stride_pixels + width_usize)];
                for &px in row {
                    let a = ((px >> 24) & 0xFF) as u8;
                    let r = ((px >> 16) & 0xFF) as u8;
                    let g = ((px >> 8) & 0xFF) as u8;
                    let b = (px & 0xFF) as u8;

                    self.overlay_upload_rgba[out_i] = r;
                    self.overlay_upload_rgba[out_i + 1] = g;
                    self.overlay_upload_rgba[out_i + 2] = b;
                    self.overlay_upload_rgba[out_i + 3] = a;
                    out_i += 4;
                }
            }

            let rgba_ptr = self.overlay_upload_rgba.as_ptr() as *const c_void;

            if self.overlay_texture_size != (width, height) {
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    width as i32,
                    height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    rgba_ptr,
                );
                self.overlay_texture_size = (width, height);
            } else {
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    rgba_ptr,
                );
            }

            // Save state we mutate (keep it minimal and correct).
            let prev_program = {
                let mut v = 0i32;
                gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut v);
                v as u32
            };
            let prev_active_tex = {
                let mut v = 0i32;
                gl::GetIntegerv(gl::ACTIVE_TEXTURE, &mut v);
                v as u32
            };
            let prev_tex2d = {
                let mut v = 0i32;
                gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut v);
                v as u32
            };
            let prev_vao = {
                let mut v = 0i32;
                gl::GetIntegerv(gl::VERTEX_ARRAY_BINDING, &mut v);
                v as u32
            };
            let prev_blend = gl::IsEnabled(gl::BLEND) == gl::TRUE;
            let prev_depth = gl::IsEnabled(gl::DEPTH_TEST) == gl::TRUE;
            let prev_cull = gl::IsEnabled(gl::CULL_FACE) == gl::TRUE;
            let prev_scissor = gl::IsEnabled(gl::SCISSOR_TEST) == gl::TRUE;
            let prev_scissor_box = {
                let mut r = [0i32; 4];
                gl::GetIntegerv(gl::SCISSOR_BOX, r.as_mut_ptr());
                r
            };

            // Local, bounded overlay draw.
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(0, 0, width as i32, height as i32);

            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);

            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            gl::UseProgram(self.program_overlay);

            gl::ActiveTexture(gl::TEXTURE0 + OVERLAY_TEX_UNIT);
            gl::BindTexture(gl::TEXTURE_2D, self.overlay_texture);

            let tex_loc =
                gl::GetUniformLocation(self.program_overlay, b"tex\0".as_ptr() as *const i8);
            if tex_loc >= 0 {
                gl::Uniform1i(tex_loc, OVERLAY_TEX_UNIT as i32);
            }

            gl::BindVertexArray(self.overlay_vao);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            // Restore state.
            gl::BindVertexArray(prev_vao);
            gl::ActiveTexture(prev_active_tex);
            gl::BindTexture(gl::TEXTURE_2D, prev_tex2d);
            gl::UseProgram(prev_program);

            if prev_blend {
                gl::Enable(gl::BLEND);
            } else {
                gl::Disable(gl::BLEND);
            }
            if prev_depth {
                gl::Enable(gl::DEPTH_TEST);
            } else {
                gl::Disable(gl::DEPTH_TEST);
            }
            if prev_cull {
                gl::Enable(gl::CULL_FACE);
            } else {
                gl::Disable(gl::CULL_FACE);
            }

            if prev_scissor {
                gl::Enable(gl::SCISSOR_TEST);
            } else {
                gl::Disable(gl::SCISSOR_TEST);
            }
            gl::Scissor(
                prev_scissor_box[0],
                prev_scissor_box[1],
                prev_scissor_box[2],
                prev_scissor_box[3],
            );
        }

        true
    }

    pub fn clear(&mut self, r: f32, g: f32, b: f32, a: f32) -> bool {
        if self.output_fbo == 0 {
            return false;
        }
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.output_fbo);
            gl::ClearColor(r, g, b, a);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        true
    }
}

pub fn init_gl_renderer(use_gles: bool) -> bool {
    let lock = GL_RENDERER.get_or_init(|| Mutex::new(None));
    let mut g = lock.lock().unwrap();
    if g.is_some() {
        return true;
    }
    match GlRenderer::init(use_gles) {
        Ok(r) => {
            *g = Some(r);
            true
        }
        Err(e) => {
            eprintln!("(wasm96-libretro) gl_renderer init failed: {e}");
            false
        }
    }
}

pub fn prepare_frame(fbo: u32, width: u32, height: u32) {
    let Some(lock) = GL_RENDERER.get() else {
        return;
    };
    let mut g = lock.lock().unwrap();
    if let Some(r) = g.as_mut() {
        r.prepare_frame(fbo, width, height);
    }
}

pub fn composite_frame(framebuffer: &[u32], width: u32, height: u32, stride_pixels: u32) -> bool {
    let Some(lock) = GL_RENDERER.get() else {
        return false;
    };
    let mut g = lock.lock().unwrap();
    let Some(r) = g.as_mut() else {
        return false;
    };
    r.composite_frame(framebuffer, width, height, stride_pixels)
}

pub fn clear_framebuffer(r: f32, g: f32, b: f32, a: f32) -> bool {
    let Some(lock) = GL_RENDERER.get() else {
        return false;
    };
    let mut st = lock.lock().unwrap();
    let Some(renderer) = st.as_mut() else {
        return false;
    };
    renderer.clear(r, g, b, a)
}

fn create_program(vs_src: &str, fs_src: &str) -> Result<u32, String> {
    unsafe {
        let vs = compile_shader(gl::VERTEX_SHADER, vs_src)?;
        let fs = compile_shader(gl::FRAGMENT_SHADER, fs_src)?;

        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);

        let mut linked = 0i32;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut linked);
        if linked == 0 {
            let mut len = 0i32;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec![0u8; len.max(1) as usize];
            gl::GetProgramInfoLog(
                program,
                len,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut i8,
            );
            gl::DeleteProgram(program);
            return Err(String::from_utf8_lossy(&buf).trim().to_string());
        }

        Ok(program)
    }
}

fn compile_shader(kind: u32, src: &str) -> Result<u32, String> {
    unsafe {
        let shader = gl::CreateShader(kind);
        let c_str = std::ffi::CString::new(src).map_err(|e| e.to_string())?;
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        let mut ok = 0i32;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut ok);
        if ok == 0 {
            let mut len = 0i32;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec![0u8; len.max(1) as usize];
            gl::GetShaderInfoLog(
                shader,
                len,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut i8,
            );
            gl::DeleteShader(shader);
            return Err(String::from_utf8_lossy(&buf).trim().to_string());
        }

        Ok(shader)
    }
}
