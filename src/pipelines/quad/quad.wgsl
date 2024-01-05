struct PushConstants {
    quad_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    dims: vec2<f32>,
    uv_offset: vec2<f32>,
    uv_scale: vec2<f32>,
    aa_strength: f32,
}
var<push_constant> pc: PushConstants;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = (model.tex_coords + pc.uv_offset) / pc.uv_scale;
    let world_position = vec4<f32>(pc.dims, 0.0, 1.0) *
        // Scale vertices
        model.position +
        // Offset by pos
        pc.quad_pos;
    out.clip_position = pc.view_proj * world_position;
    out.color = model.color;
    return out;
}

@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0)@binding(1)
var s: sampler;

// https://www.shadertoy.com/view/MllBWf
fn get_coords_aa(uv: vec2<f32>) -> vec2<f32> {
    let fl = floor(uv + 0.5);
    var fr = fract(uv + 0.5);
    let aa = fwidth(uv) * pc.aa_strength * 0.5;
    fr = smoothstep(0.5 - aa, 0.5 + aa, fr);
    return fl + fr - 0.5;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(input_texture));
    return in.color * textureSample(input_texture, s, get_coords_aa(in.tex_coords * size) / size);
}