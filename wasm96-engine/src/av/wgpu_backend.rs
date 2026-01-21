use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::resources::RESOURCES;
use crate::state::global;

fn srgb_to_linear(f: f32) -> f32 {
    if f <= 0.04045 {
        f / 12.92
    } else {
        ((f + 0.055) / 1.055).powf(2.4)
    }
}

// --- Data Structures ---

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgpuVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

impl WgpuVertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct WgpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub texture_key: Option<u64>,
}

#[derive(Clone, Copy)]
struct DrawCall {
    mesh_key: u64,
    mvp: Mat4,
    normal_mat: Mat4,
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct WgpuUniforms {
    mvp: [f32; 16],
    normal_mat: [f32; 16],
    color: [f32; 3],
    use_tex: f32,
}

struct WgpuContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    pipeline_3d: wgpu::RenderPipeline,
    pipeline_2d: wgpu::RenderPipeline,

    bind_group_layout_3d: wgpu::BindGroupLayout,
    bind_group_layout_tex: wgpu::BindGroupLayout,

    overlay_texture: Option<wgpu::Texture>,
    overlay_bind_group: Option<wgpu::BindGroup>,
    sampler: wgpu::Sampler,

    meshes: HashMap<u64, WgpuMesh>,
    draw_calls: Vec<DrawCall>,
    textures: HashMap<u64, Arc<wgpu::BindGroup>>,
    dummy_bind_group_3d: wgpu::BindGroup,
    dummy_bind_group_tex: wgpu::BindGroup,
    clear_color: wgpu::Color,
    depth_texture: Option<wgpu::Texture>,
}

// --- Global State ---

static WGPU_CTX: OnceLock<Mutex<Option<WgpuContext>>> = OnceLock::new();

// --- Shaders ---

const SHADER_SOURCE: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

struct Uniforms {
    mvp: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
    color: vec3<f32>,
    use_tex: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.mvp * vec4<f32>(model.position, 1.0);
    out.uv = model.uv;
    out.normal = (uniforms.normal_mat * vec4<f32>(model.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.2));
    let diffuse = max(dot(normalize(in.normal), light_dir), 0.2);

    var color = vec4<f32>(uniforms.color, 1.0);
    if (uniforms.use_tex > 0.5) {
        color = color * textureSample(t_diffuse, s_diffuse, in.uv);
    }

    return vec4<f32>(color.rgb * diffuse, color.a);
}

struct OverlayOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_overlay(@builtin(vertex_index) vertex_index: u32) -> OverlayOutput {
    let x = f32(i32(vertex_index & 1u) * 2 - 1);
    let y = f32(i32(vertex_index & 2u) - 1);
    var out: OverlayOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_overlay(in: OverlayOutput) -> @location(0) vec4<f32> {
    let tex_uv = vec2<f32>(in.uv.x * 0.5 + 0.5, 1.0 - (in.uv.y * 0.5 + 0.5));
    return textureSample(t_diffuse, s_diffuse, tex_uv);
}
"#;

// --- Public API ---

pub fn init_wgpu(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, format: wgpu::TextureFormat) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wasm96_shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });

    let bind_group_layout_3d = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bind_group_layout_3d"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bind_group_layout_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bind_group_layout_tex"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout_3d = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pipeline_layout_3d"),
        bind_group_layouts: &[&bind_group_layout_3d, &bind_group_layout_tex],
        push_constant_ranges: &[],
    });

    let pipeline_3d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pipeline_3d"),
        layout: Some(&pipeline_layout_3d),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[WgpuVertex::layout()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let pipeline_layout_2d = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pipeline_layout_2d"),
        bind_group_layouts: &[&bind_group_layout_3d, &bind_group_layout_tex],
        push_constant_ranges: &[],
    });

    let pipeline_2d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pipeline_2d"),
        layout: Some(&pipeline_layout_2d),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_overlay",
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_overlay",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let dummy_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dummy_uniforms"),
        size: std::mem::size_of::<WgpuUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });

    let dummy_bind_group_3d = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dummy_bind_group_3d"),
        layout: &bind_group_layout_3d,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: dummy_buffer.as_entire_binding(),
        }],
    });

    // Create a dummy texture/sampler to satisfy the texture bind group layout
    let dummy_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("dummy_tex"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let dummy_view = dummy_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let dummy_bind_group_tex = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dummy_bind_group_tex"),
        layout: &bind_group_layout_tex,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dummy_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let ctx = WgpuContext {
        device,
        queue,
        pipeline_3d,
        pipeline_2d,
        bind_group_layout_3d,
        bind_group_layout_tex,
        overlay_texture: None,
        overlay_bind_group: None,
        sampler,
        meshes: HashMap::new(),
        draw_calls: Vec::new(),
        textures: HashMap::new(),
        dummy_bind_group_3d,
        dummy_bind_group_tex,
        clear_color: wgpu::Color::BLACK,
        depth_texture: None,
    };

    let mut lock = WGPU_CTX.get_or_init(|| Mutex::new(None)).lock().unwrap();
    *lock = Some(ctx);
}

pub fn wgpu_mesh_create(key: u64, vertices: &[WgpuVertex], indices: &[u16]) {
    let Some(lock) = WGPU_CTX.get() else { return };
    let mut lock = lock.lock().unwrap();
    if let Some(ctx) = lock.as_mut() {
        let v_buf = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh_v_buf"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let i_buf = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh_i_buf"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        ctx.meshes.insert(
            key,
            WgpuMesh {
                vertex_buffer: v_buf,
                index_buffer: i_buf,
                index_count: indices.len() as u32,
                texture_key: None,
            },
        );
    }
}

pub fn wgpu_mesh_set_texture(mesh_key: u64, tex_key: u64) {
    let Some(lock) = WGPU_CTX.get() else { return };
    let mut lock = lock.lock().unwrap();
    if let Some(ctx) = lock.as_mut() {
        if let Some(mesh) = ctx.meshes.get_mut(&mesh_key) {
            mesh.texture_key = Some(tex_key);
        }
    }
}

pub fn wgpu_mesh_draw(
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
    let Some(lock) = WGPU_CTX.get() else { return };
    let mut lock = lock.lock().unwrap();
    if let Some(ctx) = lock.as_mut() {
        let state_3d = super::graphics3d::STATE_3D.lock().unwrap();
        if !state_3d.enabled {
            return;
        }

        let model = Mat4::from_translation(Vec3::new(x, y, z))
            * Mat4::from_rotation_z(rz)
            * Mat4::from_rotation_y(ry)
            * Mat4::from_rotation_x(rx)
            * Mat4::from_scale(Vec3::new(sx, sy, sz));

        let mvp = state_3d.projection * state_3d.view * model;
        let normal_mat = model.inverse().transpose();

        let color_u32 = global().lock().unwrap().video.draw_color;
        let r = srgb_to_linear(((color_u32 >> 16) & 0xFF) as f32 / 255.0);
        let g = srgb_to_linear(((color_u32 >> 8) & 0xFF) as f32 / 255.0);
        let b = srgb_to_linear((color_u32 & 0xFF) as f32 / 255.0);

        ctx.draw_calls.push(DrawCall {
            mesh_key: key,
            mvp,
            normal_mat,
            color: [r, g, b],
        });
    }
}

pub fn wgpu_clear_framebuffer(r: f32, g: f32, b: f32, a: f32) -> bool {
    if let Some(lock) = WGPU_CTX.get() {
        if let Some(ctx) = lock.lock().unwrap().as_mut() {
            ctx.clear_color = wgpu::Color {
                r: srgb_to_linear(r) as f64,
                g: srgb_to_linear(g) as f64,
                b: srgb_to_linear(b) as f64,
                a: a as f64,
            };
            return true;
        }
    }
    false
}

pub fn wgpu_present(view: &wgpu::TextureView, width: u32, height: u32, sw_framebuffer: &[u32]) {
    let Some(lock) = WGPU_CTX.get() else { return };
    let mut lock = lock.lock().unwrap();
    let Some(ctx) = lock.as_mut() else { return };

    // Update overlay texture
    if ctx.overlay_texture.is_none()
        || ctx.overlay_texture.as_ref().unwrap().width() != width
        || ctx.overlay_texture.as_ref().unwrap().height() != height
    {
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("overlay_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay_bind_group"),
            layout: &ctx.bind_group_layout_tex,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&ctx.sampler),
                },
            ],
        });
        ctx.overlay_texture = Some(texture);
        ctx.overlay_bind_group = Some(bind_group);
    }

    // Update depth texture
    if ctx.depth_texture.is_none()
        || ctx.depth_texture.as_ref().unwrap().width() != width
        || ctx.depth_texture.as_ref().unwrap().height() != height
    {
        let depth_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
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
        ctx.depth_texture = Some(depth_texture);
    }

    // Repack ARGB8888 to RGBA8888 for wgpu
    let mut rgba = Vec::with_capacity(sw_framebuffer.len() * 4);
    for &p in sw_framebuffer {
        rgba.push(((p >> 16) & 0xFF) as u8);
        rgba.push(((p >> 8) & 0xFF) as u8);
        rgba.push((p & 0xFF) as u8);
        rgba.push(((p >> 24) & 0xFF) as u8);
    }

    ctx.queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: ctx.overlay_texture.as_ref().unwrap(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    // Prepare 3D resources before starting render pass to satisfy lifetimes
    let mut prepared_calls = Vec::new();
    for call in &ctx.draw_calls {
        if let Some(mesh) = ctx.meshes.get(&call.mesh_key) {
            // Handle texture if present
            let tex_bg = if let Some(tex_key) = mesh.texture_key {
                if !ctx.textures.contains_key(&tex_key) {
                    let img = {
                        let res = RESOURCES.lock().unwrap();
                        res.keyed_images.get(&tex_key).cloned()
                    };

                    if let Some(img) = img {
                        let size = wgpu::Extent3d {
                            width: img.width,
                            height: img.height,
                            depth_or_array_layers: 1,
                        };
                        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
                            label: None,
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });

                        ctx.queue.write_texture(
                            wgpu::ImageCopyTexture {
                                texture: &texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            &img.rgba,
                            wgpu::ImageDataLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * img.width),
                                rows_per_image: Some(img.height),
                            },
                            size,
                        );

                        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                        let bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: None,
                            layout: &ctx.bind_group_layout_tex,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::TextureView(&view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(&ctx.sampler),
                                },
                            ],
                        });
                        ctx.textures.insert(tex_key, Arc::new(bg));
                    }
                }
                ctx.textures.get(&tex_key).cloned()
            } else {
                None
            };

            let uniforms = WgpuUniforms {
                mvp: call.mvp.to_cols_array(),
                normal_mat: call.normal_mat.to_cols_array(),
                color: call.color,
                use_tex: if tex_bg.is_some() { 1.0 } else { 0.0 },
            };
            let u_buf = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::bytes_of(&uniforms),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &ctx.bind_group_layout_3d,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: u_buf.as_entire_binding(),
                }],
            });
            prepared_calls.push((u_buf, bg, tex_bg, call.mesh_key));
        }
    }

    let depth_view = ctx
        .depth_texture
        .as_ref()
        .unwrap()
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("present_encoder"),
        });
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("present_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(ctx.clear_color),
                    store: wgpu::StoreOp::Store,
                },
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

        // 3D pass
        rpass.set_pipeline(&ctx.pipeline_3d);
        for (_u_buf, bg, tex_bg, mesh_key) in &prepared_calls {
            if let Some(mesh) = ctx.meshes.get(mesh_key) {
                rpass.set_bind_group(0, bg, &[]);
                if let Some(tbg) = tex_bg {
                    rpass.set_bind_group(1, tbg, &[]);
                } else {
                    rpass.set_bind_group(1, &ctx.dummy_bind_group_tex, &[]);
                }
                rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }

        // 2D Overlay pass
        rpass.set_pipeline(&ctx.pipeline_2d);
        if let Some(bg) = &ctx.overlay_bind_group {
            rpass.set_bind_group(0, &ctx.dummy_bind_group_3d, &[]);
            rpass.set_bind_group(1, bg, &[]);
            rpass.draw(0..4, 0..1);
        }
    }

    ctx.queue.submit(std::iter::once(encoder.finish()));
    ctx.draw_calls.clear();
}
