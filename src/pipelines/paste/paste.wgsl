struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
};

struct PushConstants {
    scale: vec2<f32>,
    offset: vec2<f32>,
}
var<push_constant> pc: PushConstants;

// https://github.com/bevyengine/bevy/blob/09df19bcadb52d2f4dbbc899aef74cafa9091538/crates/bevy_core_pipeline/src/fullscreen_vertex_shader/fullscreen.wgsl
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let pos_2d = (uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0)) *
        pc.scale + pc.offset;
    let clip_position = vec4<f32>(pos_2d, 0.0, 1.0);
    return VertexOutput(clip_position, uv);
}

@group(0) @binding(0)
var in_texture: texture_2d<f32>;
@group(0) @binding(1)
var in_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(in_texture, in_sampler, in.uv);
}