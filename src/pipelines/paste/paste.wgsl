struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct PushConstants {
    tint: vec4<f32>,
    scale: vec2<f32>,
    offset: vec2<f32>,
}
var<push_constant> pc: PushConstants;

// https://github.com/bevyengine/bevy/blob/09df19bcadb52d2f4dbbc899aef74cafa9091538/crates/bevy_core_pipeline/src/fullscreen_vertex_shader/fullscreen.wgsl
@vertex
fn vs_main(
   quad: VertexInput,
) -> VertexOutput {
    let world_position = vec4<f32>(2.0 * quad.position.xy * pc.scale + pc.offset, 0.0, 1.0);
    return VertexOutput(world_position, quad.tex_coords);
}

@group(0) @binding(0)
var in_texture: texture_2d<f32>;
@group(0) @binding(1)
var in_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(clamp(in.uv.x, 0.0, 1.0), clamp(in.uv.y, 0.0, 1.0));
    return textureSample(in_texture, in_sampler, uv) * pc.tint;
}