// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) t_value: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// We have a max of 1024 points, since the max uniform buffer size is (as low as) 16KB
@group(1) @binding(0)
var<uniform> point_colors: array<vec4<f32>, 1024>;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    let lower_t = floor(model.t_value);
    let interp_t = model.t_value - lower_t;
    out.color = point_colors[u32(lower_t)] * (1.0 - interp_t) + point_colors[u32(lower_t) + 1u] * interp_t;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
