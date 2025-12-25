use crate::state;
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use wasmtime::Caller;
use wgpu::util::DeviceExt;

lazy_static::lazy_static! {
    static ref STATE_3D: Mutex<State3d> = Mutex::new(State3d::default());
    static ref MESH_STORE: Mutex<HashMap<u64, Mesh>> = Mutex::new(HashMap::new());
}

static WGPU_STATE: OnceLock<Option<Mutex<WgpuState>>> = OnceLock::new();

struct WgpuState {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    target_texture: Option<wgpu::Texture>,
    depth_texture: Option<wgpu::Texture>,
    output_buffer: Option<wgpu::Buffer>,
    texture_size: (u32, u32),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    normal_mat: [[f32; 4]; 4],
}

struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    gpu_buffers: Option<(wgpu::Buffer, wgpu::Buffer)>,
}

struct State3d {
    enabled: bool,
    view: Mat4,
    projection: Mat4,
}

impl Default for State3d {
    fn default() -> Self {
        Self {
            enabled: false,
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
        }
    }
}

// Minimal block_on for wgpu async init
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    unsafe fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    let mut pinned = Box::pin(future);

    loop {
        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(val) => return val,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

pub fn init_gl_context() {
    // Initialize WGPU on the first context reset
    WGPU_STATE.get_or_init(|| {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }));

        let adapter = match adapter {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Failed to find an appropriate wgpu adapter: {:?}", e);
                return None;
            }
        };

        let (device, queue): (wgpu::Device, wgpu::Queue) =
            match block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Wasm96 Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to create wgpu device: {:?}", e);
                    return None;
                }
            };

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shader.wgsl"
            ))),
        });

        // Uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            multiview_mask: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 20,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),

            cache: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Some(Mutex::new(WgpuState {
            instance,
            device,
            queue,
            pipeline,
            uniform_buffer,
            bind_group,
            target_texture: None,
            depth_texture: None,
            output_buffer: None,
            texture_size: (0, 0),
        }))
    });
}

pub fn graphics_set_3d(enable: bool) {
    let mut state = STATE_3D.lock().unwrap();
    state.enabled = enable;
    if enable {
        init_gl_context();
    }
}

pub fn graphics_camera_look_at(
    eye_x: f32,
    eye_y: f32,
    eye_z: f32,
    target_x: f32,
    target_y: f32,
    target_z: f32,
    up_x: f32,
    up_y: f32,
    up_z: f32,
) {
    let mut state = STATE_3D.lock().unwrap();
    state.view = Mat4::look_at_rh(
        Vec3::new(eye_x, eye_y, eye_z),
        Vec3::new(target_x, target_y, target_z),
        Vec3::new(up_x, up_y, up_z),
    );
}

pub fn graphics_camera_perspective(fovy: f32, aspect: f32, near: f32, far: f32) {
    let mut state = STATE_3D.lock().unwrap();
    state.projection = Mat4::perspective_rh(fovy, aspect, near, far);
}

pub fn graphics_mesh_create(
    caller: &mut Caller<'_, ()>,
    key: u64,
    v_ptr: u32,
    v_len: u32,
    i_ptr: u32,
    i_len: u32,
) -> u32 {
    let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
        Some(m) => m,
        None => return 0,
    };

    let v_byte_len = (v_len as usize) * 8 * 4;
    let mut v_bytes = vec![0u8; v_byte_len];
    if memory
        .read(&mut *caller, v_ptr as usize, &mut v_bytes)
        .is_err()
    {
        return 0;
    }

    let mut vertices = Vec::with_capacity(v_len as usize);
    for i in 0..v_len as usize {
        let offset = i * 8 * 4;
        let floats: &[f32] = bytemuck::cast_slice(&v_bytes[offset..offset + 32]);
        vertices.push(Vertex {
            position: [floats[0], floats[1], floats[2]],
            uv: [floats[3], floats[4]],
            normal: [floats[5], floats[6], floats[7]],
        });
    }

    let i_byte_len = (i_len as usize) * 4;
    let mut i_bytes = vec![0u8; i_byte_len];
    if memory
        .read(&mut *caller, i_ptr as usize, &mut i_bytes)
        .is_err()
    {
        return 0;
    }
    let indices: Vec<u32> = bytemuck::cast_slice(&i_bytes).to_vec();

    let mut meshes = MESH_STORE.lock().unwrap();
    meshes.insert(
        key,
        Mesh {
            vertices,
            indices,
            gpu_buffers: None,
        },
    );
    1
}

pub fn graphics_mesh_create_obj(caller: &mut Caller<'_, ()>, key: u64, ptr: u32, len: u32) -> u32 {
    let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
        Some(m) => m,
        None => return 0,
    };

    let mut bytes = vec![0u8; len as usize];
    if memory.read(&mut *caller, ptr as usize, &mut bytes).is_err() {
        return 0;
    }

    let Ok(obj): Result<obj::Obj<obj::TexturedVertex, u32>, _> =
        obj::load_obj(std::io::Cursor::new(&bytes))
    else {
        return 0;
    };

    let mut vertices = Vec::new();
    for v in obj.vertices {
        vertices.push(Vertex {
            position: v.position,
            uv: [v.texture[0], v.texture[1]],
            normal: v.normal,
        });
    }
    let indices = obj.indices;

    let mut meshes = MESH_STORE.lock().unwrap();
    meshes.insert(
        key,
        Mesh {
            vertices,
            indices,
            gpu_buffers: None,
        },
    );
    1
}

pub fn graphics_mesh_create_stl(caller: &mut Caller<'_, ()>, key: u64, ptr: u32, len: u32) -> u32 {
    let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
        Some(m) => m,
        None => return 0,
    };

    let mut bytes = vec![0u8; len as usize];
    if memory.read(&mut *caller, ptr as usize, &mut bytes).is_err() {
        return 0;
    }

    let mut reader = std::io::Cursor::new(&bytes);
    let Ok(mesh) = nom_stl::parse_stl(&mut reader) else {
        return 0;
    };

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for triangle in mesh.triangles() {
        let n = triangle.normal();
        for v in triangle.vertices() {
            vertices.push(Vertex {
                position: v,
                uv: [0.0, 0.0],
                normal: n,
            });
            indices.push((vertices.len() - 1) as u32);
        }
    }

    let mut meshes = MESH_STORE.lock().unwrap();
    meshes.insert(
        key,
        Mesh {
            vertices,
            indices,
            gpu_buffers: None,
        },
    );
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
    let state = STATE_3D.lock().unwrap();
    if !state.enabled {
        return;
    }

    let Some(wgpu_mutex) = WGPU_STATE.get().and_then(|opt| opt.as_ref()) else {
        return;
    };
    let mut wgpu_state = wgpu_mutex.lock().unwrap();

    let mut meshes = MESH_STORE.lock().unwrap();
    let Some(mesh) = meshes.get_mut(&key) else {
        return;
    };

    // Upload mesh if needed
    if mesh.gpu_buffers.is_none() {
        let vb = wgpu_state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh VB"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ib = wgpu_state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh IB"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        mesh.gpu_buffers = Some((vb, ib));
    }

    // Ensure textures match screen size
    let (width, height) = {
        let global = state::global().lock().unwrap();
        (global.video.width, global.video.height)
    };

    if wgpu_state.texture_size != (width, height) {
        let texture = wgpu_state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let depth_texture = wgpu_state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // Buffer for readback
        let _output_buffer_size = (width * height * 4) as wgpu::BufferAddress;
        // Align to 256 bytes
        let aligned_width_bytes = (width * 4 + 255) & !255;
        let aligned_output_buffer_size = (aligned_width_bytes * height) as wgpu::BufferAddress;

        let output_buffer = wgpu_state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: aligned_output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        wgpu_state.target_texture = Some(texture);
        wgpu_state.depth_texture = Some(depth_texture);
        wgpu_state.output_buffer = Some(output_buffer);
        wgpu_state.texture_size = (width, height);
    }

    // Update Uniforms
    let model = Mat4::from_translation(Vec3::new(x, y, z))
        * Mat4::from_euler(glam::EulerRot::XYZ, rx, ry, rz)
        * Mat4::from_scale(Vec3::new(sx, sy, sz));
    let mvp = state.projection * state.view * model;
    let normal_mat = model.inverse().transpose();

    let uniforms = Uniforms {
        mvp: mvp.to_cols_array_2d(),
        normal_mat: normal_mat.to_cols_array_2d(),
    };
    wgpu_state.queue.write_buffer(
        &wgpu_state.uniform_buffer,
        0,
        bytemuck::cast_slice(&[uniforms]),
    );

    // Render
    let mut encoder = wgpu_state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let view = wgpu_state
            .target_texture
            .as_ref()
            .unwrap()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = wgpu_state
            .depth_texture
            .as_ref()
            .unwrap()
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            multiview_mask: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Accumulate
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Accumulate depth
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some((vb, ib)) = &mesh.gpu_buffers {
            rpass.set_pipeline(&wgpu_state.pipeline);
            rpass.set_bind_group(0, &wgpu_state.bind_group, &[]);
            rpass.set_vertex_buffer(0, vb.slice(..));
            rpass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }
    }

    wgpu_state.queue.submit(Some(encoder.finish()));
}

pub fn clear_depth() {
    if let Some(wgpu_mutex) = WGPU_STATE.get().and_then(|opt| opt.as_ref()) {
        let wgpu_state = wgpu_mutex.lock().unwrap();
        if let Some(target) = &wgpu_state.target_texture {
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let depth_view = wgpu_state
                .depth_texture
                .as_ref()
                .unwrap()
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = wgpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Clear Pass"),
                    multiview_mask: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            wgpu_state.queue.submit(Some(encoder.finish()));
        }
    }
}

pub fn flush_to_host() {
    let Some(wgpu_mutex) = WGPU_STATE.get().and_then(|opt| opt.as_ref()) else {
        return;
    };
    let wgpu_state = wgpu_mutex.lock().unwrap();
    let Some(texture) = &wgpu_state.target_texture else {
        return;
    };
    let Some(output_buffer) = &wgpu_state.output_buffer else {
        return;
    };

    let (width, height) = wgpu_state.texture_size;
    let aligned_width_bytes = (width * 4 + 255) & !255;

    let mut encoder = wgpu_state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(aligned_width_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    let _index = wgpu_state.queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
    wgpu_state.instance.poll_all(true);
    rx.recv().unwrap().unwrap();

    {
        let data = buffer_slice.get_mapped_range();
        let mut global = state::global().lock().unwrap();
        let fb = &mut global.video.framebuffer;

        // Copy row by row to handle padding
        for y in 0..height {
            let src_offset = (y * aligned_width_bytes) as usize;
            let dst_offset = (y * width) as usize;
            let src_row = &data[src_offset..src_offset + (width as usize) * 4];
            let dst_row = &mut fb[dst_offset..dst_offset + (width as usize)];

            // Convert RGBA (wgpu) to ARGB (libretro)
            // WGPU Rgba8Unorm: R, G, B, A
            // Libretro XRGB8888: B, G, R, X (Little Endian u32) -> 0x00RRGGBB
            // Actually, libretro ARGB8888 is 0xAARRGGBB.
            // Let's assume we just need to pack it.
            for (i, pixel) in dst_row.iter_mut().enumerate() {
                let r = src_row[i * 4];
                let g = src_row[i * 4 + 1];
                let b = src_row[i * 4 + 2];
                let a = src_row[i * 4 + 3];

                if a > 0 {
                    // If alpha > 0, we overwrite.
                    *pixel =
                        ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }
            }
        }
    }

    output_buffer.unmap();
}
