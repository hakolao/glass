struct PushConstants {
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
    var world_position = vec4<f32>(0.0);
    // if drawing buffer, position defines where to draw the line. pc.start = 1.0,1.0
    // If drawing single line, push constant defines where to draw the line.
    if ((model.index % 2u) == 0u) {
        world_position = vec4<f32>(pc.start.xy * model.position.xy, 0.0, 1.0);
    } else {
        world_position = vec4<f32>(pc.end.xy * model.position.xy, 0.0, 1.0);
    }
    out.clip_position = pc.view_proj * world_position;
    out.color = pc.color * model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}