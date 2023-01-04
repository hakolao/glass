// From https://github.com/sotrh/learn-wgpu/blob/master/code/beginner/tutorial5-textures/src/shader.wgsl

struct PushConstants {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    scale: vec2<f32>,
}
var<push_constant> pc: PushConstants;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;

    let world_position = vec4<f32>(pc.scale, 1.0, 1.0) * vec4<f32>(model.position, 1.0);
    out.clip_position = pc.view_proj * world_position;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}