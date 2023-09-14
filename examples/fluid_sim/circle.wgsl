struct PushConstants {
    view_proj: mat4x4<f32>,
    color: vec4<f32>,
    pos: vec2<f32>,
    radius: f32,
    thickness: f32,
    smoothness: f32,
}
var<push_constant> pc: PushConstants;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let vertex_pos = model.position;
    let pos = pc.pos;
    let size = pc.radius * 2.0;
    let world_position = vec4<f32>(size, size, 0.0, 1.0) *
        vec4<f32>(vertex_pos, 0.0, 1.0) +
        // Offset
        vec4<f32>(pos, 0.0, 0.0);
    out.clip_position = pc.view_proj * world_position;
    out.color = pc.color;
    out.uv = model.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = 2.0 * pc.radius;
    let uv = in.uv - vec2<f32>(0.5, 0.5);
    let dist = length(uv);
    let half_thickness = pc.thickness / 2.0;
    let outer_val = smoothstep(0.5, 0.5 - pc.smoothness, dist);
    let inner_val = smoothstep(0.5 - pc.thickness, 0.5 - pc.thickness - pc.smoothness, dist);
    let val = outer_val - inner_val;
    return in.color * val;
}