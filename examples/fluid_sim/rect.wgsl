struct PushConstants {
    view_proj: mat4x4<f32>,
    color: vec4<f32>,
    pos: vec2<f32>,
    width: f32,
    height: f32,
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
    let world_position = vec4<f32>(pc.width, pc.height, 0.0, 1.0) *
        vec4<f32>(vertex_pos, 0.0, 1.0) +
        // Offset
        vec4<f32>(pos, 0.0, 0.0);
    out.clip_position = pc.view_proj * world_position;
    out.color = pc.color;
    out.uv = model.tex_coords;
    return out;
}

fn box_sdf(position: vec2<f32>, half_size: vec2<f32>, corner_radius: f32) -> f32 {
    let q = abs(position) - half_size + corner_radius;
    return min(max(q.x, q.y), 0.0) + length(vec2<f32>(max(q.x, 0.0), max(q.y, 0.0))) - corner_radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv - vec2<f32>(0.5, 0.5);
    let half_size = vec2(0.5, 0.5);
    let ar = pc.width / pc.height;
	let d = box_sdf(uv, half_size, 0.0);
	let d2 = box_sdf(uv, half_size - vec2<f32>(pc.thickness) * vec2<f32>(1.0, ar), 0.0);
    let val1 = 1.0 - sign(d);
    let val2 = 1.0 - sign(d2);
    let val = val1 - val2;
    return in.color * val;
}