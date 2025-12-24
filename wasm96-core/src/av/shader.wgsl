struct Uniforms {
    mvp: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

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

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.mvp * vec4<f32>(model.position, 1.0);
    out.uv = model.uv;
    // Transform normal to world space
    out.normal = (uniforms.normal_mat * vec4<f32>(model.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Fixed directional light
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.5));
    let normal = normalize(in.normal);

    // Diffuse lighting
    let diffuse = max(dot(normal, light_dir), 0.2);

    // White material
    let color = vec3<f32>(1.0, 1.0, 1.0) * diffuse;

    return vec4<f32>(color, 1.0);
}
