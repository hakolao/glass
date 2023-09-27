struct PushConstants {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    start: vec4<f32>,
    end: vec4<f32>,
    color: vec4<f32>,
}
var<push_constant> pc: PushConstants;

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    var pos = vec4<f32>(0.0);
    if (model.index == u32(0)) {
        pos = pc.start;
    } else {
        pos = pc.end;
    }
    let world_position = pos + pc.view_pos;
    out.clip_position = pc.view_proj * world_position;
    out.color = pc.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}