//! 3D Graphics implementation using raw OpenGL (via `gl` crate).
//!
//! This module handles:
//! - OpenGL context initialization (loading function pointers).
//! - Managing 3D resources (meshes, shaders, textures).
//! - Drawing 3D scenes.
//! - Compositing the 2D host framebuffer (overlay) onto the 3D scene.
//!
//! NOTE: Some paths in this module create temporary GL textures during `graphics_mesh_draw`.
//! Those textures must be deleted after drawing to avoid leaking GL texture IDs.

use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::io::Cursor;
use std::path::Path;

use std::sync::{Mutex, OnceLock};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use crate::state::global;

use super::resources::RESOURCES;
use super::utils::read_guest_bytes;

// --- Data Structures ---

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

pub struct Mesh {
    pub vao: u32,
    #[allow(dead_code)]
    pub vbo: u32,
    #[allow(dead_code)]
    pub ebo: u32,
    pub index_count: i32,

    /// Optional bound texture for this mesh (keyed image id).
    /// If `None`, the 3D shader will render using the uniform `color`.
    pub texture_key: Option<u64>,
}

#[derive(Default, Clone, Copy)]
pub struct State3d {
    pub enabled: bool,
    pub view: Mat4,
    pub projection: Mat4,
}

struct GlState {
    // 3D Shader
    program_3d: u32,
    uniform_mvp: i32,
    uniform_normal_mat: i32,
    uniform_color: i32,
    uniform_tex3d: i32,
    uniform_use_tex: i32,

    // Overlay Shader (2D)
    program_overlay: u32,
    #[allow(dead_code)]
    uniform_tex: i32,

    // Overlay Resources
    overlay_vao: u32, // Empty VAO for attribute-less rendering
    overlay_texture: u32,
    overlay_texture_size: (u32, u32),

    output_fbo: u32,
}

// --- Global State ---

static STATE_3D: Mutex<State3d> = Mutex::new(State3d {
    enabled: false,
    view: Mat4::IDENTITY,
    projection: Mat4::IDENTITY,
});

lazy_static::lazy_static! {
    static ref MESH_STORE: Mutex<HashMap<u64, Mesh>> = Mutex::new(HashMap::new());
}
static GL_STATE: OnceLock<Mutex<GlState>> = OnceLock::new();

// --- Shaders ---

const VS_3D_SRC: &str = r#"
#version 330 core
layout(location = 0) in vec3 position;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec3 normal;

uniform mat4 mvp;
uniform mat4 normal_mat;

out vec3 v_normal;
out vec2 v_uv;

void main() {
    gl_Position = mvp * vec4(position, 1.0);
    v_normal = mat3(normal_mat) * normal;
    v_uv = uv;
}
"#;

const FS_3D_SRC: &str = r#"
#version 330 core
in vec3 v_normal;
in vec2 v_uv;

uniform vec3 color;
uniform sampler2D tex;
uniform int use_tex;

out vec4 FragColor;

void main() {
    // Simple directional lighting
    vec3 light_dir = normalize(vec3(0.5, 1.0, 0.5));
    float diff = max(dot(normalize(v_normal), light_dir), 0.2);

    vec3 base = color;
    float alpha = 1.0;

    if (use_tex != 0) {
        vec4 t = texture(tex, v_uv);
        base = t.rgb;
        alpha = t.a;
    }

    FragColor = vec4(base * diff, alpha);
}
"#;

const VS_OVERLAY_SRC: &str = r#"
#version 330 core
// Fullscreen triangle strip generated in shader
const vec2 verts[4] = vec2[](vec2(-1,-1), vec2(1,-1), vec2(-1,1), vec2(1,1));
const vec2 uvs[4] = vec2[](vec2(0,1), vec2(1,1), vec2(0,0), vec2(1,0));

out vec2 v_uv;

void main() {
    gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
    v_uv = uvs[gl_VertexID];
}
"#;

const FS_OVERLAY_SRC: &str = r#"
#version 330 core
in vec2 v_uv;
uniform sampler2D tex;
out vec4 FragColor;

void main() {
    vec4 c = texture(tex, v_uv);
    // Assume texture is BGRA (uploaded from XRGB/ARGB host buffer).
    // If alpha is 0, discard to show 3D scene behind.
    if (c.a == 0.0) discard;
    FragColor = c;
}
"#;

// --- Initialization ---

pub fn init_gl_context<F>(loader: F)
where
    F: Fn(&str) -> *const c_void,
{
    // Load GL functions
    gl::load_with(loader);

    // Clear mesh store as GL context is new
    MESH_STORE.lock().unwrap().clear();

    // Initialize GL state
    let program_3d = create_program(VS_3D_SRC, FS_3D_SRC);
    check_gl_error("create_program 3d");
    let program_overlay = create_program(VS_OVERLAY_SRC, FS_OVERLAY_SRC);
    check_gl_error("create_program overlay");

    let uniform_mvp = unsafe {
        let name = CString::new("mvp").unwrap();
        gl::GetUniformLocation(program_3d, name.as_ptr())
    };
    let uniform_normal_mat = unsafe {
        let name = CString::new("normal_mat").unwrap();
        gl::GetUniformLocation(program_3d, name.as_ptr())
    };
    let uniform_color = unsafe {
        let name = CString::new("color").unwrap();
        gl::GetUniformLocation(program_3d, name.as_ptr())
    };
    let uniform_tex3d = unsafe {
        let name = CString::new("tex").unwrap();
        gl::GetUniformLocation(program_3d, name.as_ptr())
    };
    let uniform_use_tex = unsafe {
        let name = CString::new("use_tex").unwrap();
        gl::GetUniformLocation(program_3d, name.as_ptr())
    };

    let uniform_tex = unsafe {
        let name = CString::new("tex").unwrap();
        gl::GetUniformLocation(program_overlay, name.as_ptr())
    };
    check_gl_error("get uniforms");

    let mut overlay_vao = 0;
    let mut overlay_texture = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut overlay_vao);
        gl::GenTextures(1, &mut overlay_texture);

        // Setup default texture params
        gl::BindTexture(gl::TEXTURE_2D, overlay_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    }
    check_gl_error("overlay setup");

    let state = GlState {
        program_3d,
        uniform_mvp,
        uniform_normal_mat,
        uniform_color,
        uniform_tex3d,
        uniform_use_tex,
        program_overlay,
        uniform_tex,
        overlay_vao,
        overlay_texture,
        overlay_texture_size: (0, 0),
        output_fbo: 0,
    };

    GL_STATE.get_or_init(|| Mutex::new(state));

    // Initial GL setup
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
    }

    check_gl_error("init_gl_context");
}

fn check_gl_error(label: &str) {
    unsafe {
        let mut err = gl::GetError();
        while err != gl::NO_ERROR {
            eprintln!("GL Error at {}: 0x{:X}", label, err);
            err = gl::GetError();
        }
    }
}

fn create_program(vs_src: &str, fs_src: &str) -> u32 {
    unsafe {
        let vs = compile_shader(gl::VERTEX_SHADER, vs_src);
        let fs = compile_shader(gl::FRAGMENT_SHADER, fs_src);
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        // Check link status
        let mut success = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success == 0 {
            let mut len = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = Vec::<u8>::with_capacity(len as usize);
            buffer.set_len((len as usize) - 1);
            gl::GetProgramInfoLog(
                program,
                len,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut _,
            );
            eprintln!("Program link error: {}", String::from_utf8_lossy(&buffer));
        }

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);
        program
    }
}

fn compile_shader(type_: u32, src: &str) -> u32 {
    unsafe {
        let shader = gl::CreateShader(type_);
        let c_str = CString::new(src).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        // Check compile status
        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = Vec::<u8>::with_capacity(len as usize);
            buffer.set_len((len as usize) - 1);
            gl::GetShaderInfoLog(
                shader,
                len,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut _,
            );
            eprintln!("Shader compile error: {}", String::from_utf8_lossy(&buffer));
        }
        shader
    }
}

// --- API ---

pub fn graphics_set_3d(enabled: bool) {
    let mut s = STATE_3D.lock().unwrap();
    s.enabled = enabled;

    // If enabling 3D, we might want to clear the framebuffer here?
    // For now, we rely on the frame loop.
}

pub fn graphics_camera_look_at(
    eye_x: f32,
    eye_y: f32,
    eye_z: f32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
    up_x: f32,
    up_y: f32,
    up_z: f32,
) {
    let mut s = STATE_3D.lock().unwrap();
    s.view = Mat4::look_at_rh(
        Vec3::new(eye_x, eye_y, eye_z),
        Vec3::new(center_x, center_y, center_z),
        Vec3::new(up_x, up_y, up_z),
    );
}

pub fn graphics_camera_perspective(fovy: f32, aspect: f32, near: f32, far: f32) {
    let mut s = STATE_3D.lock().unwrap();
    s.projection = Mat4::perspective_rh(fovy, aspect, near, far);
}

pub fn graphics_mesh_create(
    env: &mut wasmtime::Caller<'_, ()>,
    key: u64,
    v_ptr: u32,
    v_len: u32,
    i_ptr: u32,
    i_len: u32,
) -> u32 {
    let memory = match env.get_export("memory") {
        Some(wasmtime::Extern::Memory(m)) => m,
        _ => return 0,
    };

    let (vertices, indices) = {
        let data = memory.data(env);
        let v_size = std::mem::size_of::<Vertex>();
        let v_bytes = v_len as usize * v_size;
        let i_bytes = i_len as usize * 4; // u32 indices

        let v_ptr = v_ptr as usize;
        let i_ptr = i_ptr as usize;

        if v_ptr + v_bytes > data.len() || i_ptr + i_bytes > data.len() {
            return 0;
        }

        let v_slice = &data[v_ptr..v_ptr + v_bytes];
        let i_slice = &data[i_ptr..i_ptr + i_bytes];

        let vertices: &[Vertex] = bytemuck::cast_slice(v_slice);
        let indices: &[u32] = bytemuck::cast_slice(i_slice);

        (vertices.to_vec(), indices.to_vec())
    };

    let mut vao = 0;
    let mut vbo = 0;
    let mut ebo = 0;

    if GL_STATE.get().is_none() {
        return 0;
    }

    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenBuffers(1, &mut ebo);

        gl::BindVertexArray(vao);

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<Vertex>()) as isize,
            vertices.as_ptr() as *const c_void,
            gl::STATIC_DRAW,
        );

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (indices.len() * 4) as isize,
            indices.as_ptr() as *const c_void,
            gl::STATIC_DRAW,
        );

        // Vertex attributes
        // 0: Position (3 floats)
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            std::mem::size_of::<Vertex>() as i32,
            0 as *const c_void,
        );
        gl::EnableVertexAttribArray(0);

        // 1: UV (2 floats)
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            std::mem::size_of::<Vertex>() as i32,
            12 as *const c_void,
        );
        gl::EnableVertexAttribArray(1);

        // 2: Normal (3 floats)
        gl::VertexAttribPointer(
            2,
            3,
            gl::FLOAT,
            gl::FALSE,
            std::mem::size_of::<Vertex>() as i32,
            20 as *const c_void,
        );
        gl::EnableVertexAttribArray(2);

        gl::BindVertexArray(0);

        check_gl_error("graphics_mesh_create");
    }

    let mut store = MESH_STORE.lock().unwrap();
    store.insert(
        key,
        Mesh {
            vao,
            vbo,
            ebo,
            index_count: i_len as i32,
            texture_key: None,
        },
    );
    1
}

pub fn graphics_mesh_create_obj(
    env: &mut wasmtime::Caller<'_, ()>,
    key: u64,
    ptr: u32,
    len: u32,
) -> u32 {
    // Ensure GL is initialized (we need a live context to create buffers).
    if GL_STATE.get().is_none() {
        return 0;
    }

    // Read OBJ bytes from guest memory.
    let obj_bytes = match read_guest_bytes(env, ptr, len) {
        Ok(b) => b,
        Err(_) => return 0,
    };

    // Parse OBJ using `tobj` (more robust, supports MTL).
    //
    // We load from an in-memory buffer and provide a material loader closure. Since this core
    // currently receives only OBJ bytes (no filesystem), we provide a "no materials" loader.
    // This still correctly loads geometry and supports models that either don't reference MTL,
    // or where materials are optional.
    //
    // Follow-up: we can extend the ABI to allow the guest to provide MTL bytes and texture bytes
    // so `material_loader` can parse MTL and we can register textures automatically.
    let mut reader = Cursor::new(obj_bytes);

    let (models, _materials) = match tobj::load_obj_buf(
        &mut reader,
        &tobj::LoadOptions {
            // Use tobj's standard behavior as much as possible:
            // - triangulate for our renderer
            // - single_index so tobj unifies position/uv/normal into one index stream
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |_p: &Path| -> tobj::MTLLoadResult {
            // No filesystem access / no provided MTL bytes in this call.
            // Return an empty material list (Ok) so model loading proceeds.
            Ok((Vec::new(), ahash::AHashMap::new()))
        },
    ) {
        Ok(r) => r,
        Err(_) => return 0,
    };

    if models.is_empty() {
        return 0;
    }

    // TEMP DEBUG (remove when done):
    // Dump tobj-produced stream sizes to compare against expected unified tuple counts.
    // For the included duck OBJs, expected unified vertex counts (from offline analysis):
    // - 12248_Bird_v1_L2.obj: 9582
    // - 12249_Bird_v1_L2.obj: 9760
    eprintln!("wasm96: OBJ load OK key={} models={}", key, models.len());
    for (mi, model) in models.iter().enumerate() {
        let m = &model.mesh;
        eprintln!(
            "wasm96: OBJ model[{mi}] pos={} uv={} n={} idx={} (single_index=true triangulate=true)",
            m.positions.len() / 3,
            m.texcoords.len() / 2,
            m.normals.len() / 3,
            m.indices.len(),
        );
    }

    // Convert to wasm96-core's `Vertex` and u32 indices by concatenating all models into one mesh.
    // This preserves a single VAO/VBO/EBO per `key` as expected by the current renderer.
    //
    // IMPORTANT:
    // We rely on `tobj`'s unified indexing (`single_index: true`) so positions/UVs/normals stay
    // correctly associated even for OBJs that use separate v/vt/vn indices on faces.
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for model in models.iter() {
        let mesh = &model.mesh;

        // `tobj` mesh data is flat arrays.
        if mesh.positions.len() % 3 != 0 {
            return 0;
        }
        if !mesh.texcoords.is_empty() && mesh.texcoords.len() % 2 != 0 {
            return 0;
        }
        if !mesh.normals.is_empty() && mesh.normals.len() % 3 != 0 {
            return 0;
        }

        // With `single_index: true`, `tobj` has already unified the attribute indices:
        // - `mesh.positions` / `mesh.texcoords` / `mesh.normals` have matching vertex order
        // - `mesh.indices` references that unified vertex stream
        if mesh.indices.is_empty() {
            continue;
        }

        let base_vertex = vertices.len() as u32;
        let vertex_count = mesh.positions.len() / 3;

        for i in 0..vertex_count {
            let px = mesh.positions[i * 3 + 0];
            let py = mesh.positions[i * 3 + 1];
            let pz = mesh.positions[i * 3 + 2];

            let (u, v) = if mesh.texcoords.len() >= (i * 2 + 2) {
                (mesh.texcoords[i * 2 + 0], 1.0 - mesh.texcoords[i * 2 + 1])
            } else {
                (0.0, 0.0)
            };

            let (nx, ny, nz) = if mesh.normals.len() >= (i * 3 + 3) {
                (
                    mesh.normals[i * 3 + 0],
                    mesh.normals[i * 3 + 1],
                    mesh.normals[i * 3 + 2],
                )
            } else {
                (0.0, 0.0, 1.0)
            };

            vertices.push(Vertex {
                position: [px, py, pz],
                uv: [u, v],
                normal: [nx, ny, nz],
            });
        }

        for &idx in mesh.indices.iter() {
            indices.push(base_vertex + (idx as u32));
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return 0;
    }

    // Create GL buffers (same path as `graphics_mesh_create`, but we already own the vectors).
    let mut vao = 0;
    let mut vbo = 0;
    let mut ebo = 0;

    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenBuffers(1, &mut ebo);

        gl::BindVertexArray(vao);

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<Vertex>()) as isize,
            vertices.as_ptr() as *const c_void,
            gl::STATIC_DRAW,
        );

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (indices.len() * 4) as isize,
            indices.as_ptr() as *const c_void,
            gl::STATIC_DRAW,
        );

        // Vertex attributes
        // 0: Position (3 floats)
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            std::mem::size_of::<Vertex>() as i32,
            0 as *const c_void,
        );
        gl::EnableVertexAttribArray(0);

        // 1: UV (2 floats)
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            std::mem::size_of::<Vertex>() as i32,
            12 as *const c_void,
        );
        gl::EnableVertexAttribArray(1);

        // 2: Normal (3 floats)
        gl::VertexAttribPointer(
            2,
            3,
            gl::FLOAT,
            gl::FALSE,
            std::mem::size_of::<Vertex>() as i32,
            20 as *const c_void,
        );
        gl::EnableVertexAttribArray(2);

        gl::BindVertexArray(0);

        check_gl_error("graphics_mesh_create_obj");
    }

    let mut store = MESH_STORE.lock().unwrap();
    store.insert(
        key,
        Mesh {
            vao,
            vbo,
            ebo,
            index_count: indices.len() as i32,
            texture_key: None,
        },
    );

    1
}

pub fn graphics_mesh_create_stl(
    _env: &mut wasmtime::Caller<'_, ()>,
    _key: u64,
    _ptr: u32,
    _len: u32,
) -> u32 {
    0
}

/// Bind a keyed image texture to an existing mesh.
///
/// This only stores the association (`mesh_key -> image_key`) inside the mesh store.
/// The actual GL texture object upload/lookup is expected to be handled by the graphics
/// resource system, and the draw path needs to bind the corresponding GL texture.
///
/// Returns 1 on success, 0 on failure (missing mesh).
pub fn graphics_mesh_set_texture(mesh_key: u64, image_key: u64) -> u32 {
    let mut store = MESH_STORE.lock().unwrap();
    let mesh = match store.get_mut(&mesh_key) {
        Some(m) => m,
        None => return 0,
    };

    mesh.texture_key = Some(image_key);
    1
}

pub fn graphics_mesh_draw(
    key: u64,
    x: f32,
    y: f32,
    z: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    sx: f32,
    sy: f32,
    sz: f32,
) {
    let gl_state_lock = GL_STATE.get();
    if gl_state_lock.is_none() {
        return;
    }
    let gl_state = gl_state_lock.unwrap().lock().unwrap();

    let state_3d = STATE_3D.lock().unwrap();
    if !state_3d.enabled {
        return;
    }

    let store = MESH_STORE.lock().unwrap();
    let mesh = match store.get(&key) {
        Some(m) => m,
        None => return,
    };

    // Calculate matrices
    let model = Mat4::from_translation(Vec3::new(x, y, z))
        * Mat4::from_rotation_z(rz)
        * Mat4::from_rotation_y(ry)
        * Mat4::from_rotation_x(rx)
        * Mat4::from_scale(Vec3::new(sx, sy, sz));

    let mvp = state_3d.projection * state_3d.view * model;
    let normal_mat = model.inverse().transpose();

    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, gl_state.output_fbo);
        gl::UseProgram(gl_state.program_3d);

        gl::UniformMatrix4fv(
            gl_state.uniform_mvp,
            1,
            gl::FALSE,
            mvp.to_cols_array().as_ptr(),
        );
        gl::UniformMatrix4fv(
            gl_state.uniform_normal_mat,
            1,
            gl::FALSE,
            normal_mat.to_cols_array().as_ptr(),
        );

        // Get color from global state or use default
        // Previous implementation used a uniform color.
        // We'll use white for now or get it from `VideoState`?
        // `VideoState` has `draw_color`.
        let color_u32 = global().lock().unwrap().video.draw_color;
        let r = ((color_u32 >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color_u32 >> 8) & 0xFF) as f32 / 255.0;
        let b = (color_u32 & 0xFF) as f32 / 255.0;
        gl::Uniform3f(gl_state.uniform_color, r, g, b);

        // Texture binding:
        // - PNG is treated as RGBA (alpha respected)
        // - JPEG is treated as RGB but stored/uploaded as RGBA with A=255
        //
        // Keyed textures are uploaded lazily on demand from `RESOURCES.keyed_images`.
        let mut use_tex = 0i32;
        let mut texture_id = 0u32;
        let mut delete_texture_after_draw = false;

        if let Some(img_key) = mesh.texture_key {
            let img = {
                let res = match RESOURCES.lock() {
                    Ok(r) => r,
                    Err(poisoned) => poisoned.into_inner(),
                };
                res.keyed_images.get(&img_key).cloned()
            };

            if let Some(img) = img {
                gl::GenTextures(1, &mut texture_id);
                gl::BindTexture(gl::TEXTURE_2D, texture_id);

                // Avoid shimmering/aliasing artifacts on textured 3D meshes:
                // - Use mipmaps for minification
                // - Use linear filtering for magnification
                gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    gl::LINEAR_MIPMAP_LINEAR as i32,
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

                // Avoid wrap edge artifacts on UV seams.
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);

                // Ensure tightly packed RGBA upload.
                gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);

                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA8 as i32,
                    img.width as i32,
                    img.height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    img.rgba.as_ptr() as *const c_void,
                );

                // Generate mipmaps after uploading the base level.
                gl::GenerateMipmap(gl::TEXTURE_2D);

                // Improve minification quality when the driver supports anisotropic filtering.
                // If the extension isn't present, this is a no-op.
                //
                // Note: We query via GetStringi to avoid relying on extension loader helpers.
                let mut has_aniso = false;
                let mut ext_count: i32 = 0;
                gl::GetIntegerv(gl::NUM_EXTENSIONS, &mut ext_count);
                let needle = b"GL_EXT_texture_filter_anisotropic";
                let mut i: i32 = 0;
                while i < ext_count {
                    let ext = gl::GetStringi(gl::EXTENSIONS, i as u32);
                    if !ext.is_null() {
                        // SAFETY: OpenGL guarantees NUL-terminated strings for extension names.
                        let s = std::ffi::CStr::from_ptr(ext as *const _).to_bytes();
                        if s == needle {
                            has_aniso = true;
                            break;
                        }
                    }
                    i += 1;
                }
                if has_aniso {
                    // These constants are from GL_EXT_texture_filter_anisotropic.
                    const TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
                    const MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;

                    let mut max_aniso: f32 = 1.0;
                    gl::GetFloatv(MAX_TEXTURE_MAX_ANISOTROPY_EXT, &mut max_aniso);
                    // A reasonable cap; drivers may support very high values.
                    let aniso = if max_aniso > 8.0 { 8.0 } else { max_aniso };
                    gl::TexParameterf(gl::TEXTURE_2D, TEXTURE_MAX_ANISOTROPY_EXT, aniso);
                }

                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, texture_id);

                use_tex = 1;
                delete_texture_after_draw = true;
            }
        }

        gl::Uniform1i(gl_state.uniform_use_tex, use_tex);
        gl::Uniform1i(gl_state.uniform_tex3d, 0);

        // NOTE:
        // This uses per-draw texture creation (simple but not optimal). To avoid leaking GL texture
        // IDs, we delete the texture after the draw call. A follow-up should cache GL texture ids
        // per image key and delete them on unregister/context reset.
        gl::BindVertexArray(mesh.vao);
        gl::DrawElements(
            gl::TRIANGLES,
            mesh.index_count,
            gl::UNSIGNED_INT,
            std::ptr::null(),
        );
        gl::BindVertexArray(0);

        if delete_texture_after_draw && texture_id != 0 {
            // Ensure it is not bound when we delete it.
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::DeleteTextures(1, &texture_id);
        }

        check_gl_error("graphics_mesh_draw");
    }
}

#[allow(dead_code)]
pub fn clear_depth() {
    unsafe {
        gl::Clear(gl::DEPTH_BUFFER_BIT);
    }
}

pub fn prepare_frame(fbo: usize) {
    let gl_state_lock = GL_STATE.get();
    if gl_state_lock.is_none() {
        return;
    }
    let mut gl_state = gl_state_lock.unwrap().lock().unwrap();
    gl_state.output_fbo = fbo as u32;

    let (width, height) = {
        let s = global().lock().unwrap();
        (s.video.width, s.video.height)
    };

    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo as u32);
        gl::Viewport(0, 0, width as i32, height as i32);
    }

    check_gl_error("prepare_frame");
}

pub fn flush_to_host() -> bool {
    let gl_state_lock = GL_STATE.get();
    if gl_state_lock.is_none() {
        return false;
    }
    let mut gl_state = gl_state_lock.unwrap().lock().unwrap();

    let (width, height, fb, video_cb) = {
        let s = global().lock().unwrap();
        (
            s.video.width,
            s.video.height,
            s.video.framebuffer.clone(),
            s.video_refresh_cb,
        )
    };

    if width == 0 || height == 0 {
        return true;
    }

    unsafe {
        // 1. Upload 2D framebuffer to texture
        gl::BindTexture(gl::TEXTURE_2D, gl_state.overlay_texture);

        if gl_state.overlay_texture_size != (width, height) {
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                gl::BGRA,
                gl::UNSIGNED_BYTE,
                fb.as_ptr() as *const c_void,
            );
            gl_state.overlay_texture_size = (width, height);
        } else {
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                width as i32,
                height as i32,
                gl::BGRA,
                gl::UNSIGNED_BYTE,
                fb.as_ptr() as *const c_void,
            );
        }

        // 2. Draw Overlay
        gl::BindFramebuffer(gl::FRAMEBUFFER, gl_state.output_fbo);

        // Enable blending for transparency
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl::UseProgram(gl_state.program_overlay);
        gl::BindVertexArray(gl_state.overlay_vao);
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

        gl::Disable(gl::BLEND);
        gl::BindVertexArray(0);

        // 3. Present
        // In HW render mode, we call video_refresh with RETRO_HW_FRAME_BUFFER_VALID (-1 cast to ptr)
        if let Some(cb) = video_cb {
            cb(
                libretro_sys::HW_FRAME_BUFFER_VALID as *const c_void,
                width,
                height,
                0, // Pitch is ignored for HW render
            );
        }

        check_gl_error("flush_to_host");
    }
    true
}

// Helper to clear the screen at the start of the frame (if needed)
// This should be called by the core loop, but we don't have a hook there yet.
// For now, we can rely on the fact that we draw 3D over whatever was there,
// and if we want a clear, we should add `graphics_clear` API.
// But `graphics_background` in 2D clears the 2D buffer.
// We might want `graphics_clear_3d`?
// I'll add a public function that the core *could* call if I modified `lib.rs`.
pub fn clear_framebuffer(r: f32, g: f32, b: f32, a: f32) -> bool {
    let gl_state_lock = GL_STATE.get();
    if gl_state_lock.is_none() {
        return false;
    }
    let gl_state = gl_state_lock.unwrap().lock().unwrap();

    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, gl_state.output_fbo);
        gl::ClearColor(r, g, b, a);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
    true
}
