struct PushConstants {
    draw_start: vec2<f32>,
    draw_end: vec2<f32>,
    draw_radius: f32,
}
var<push_constant> pc: PushConstants;

@group(0) @binding(0)
var data_in: texture_storage_2d<rgba16float, read_write>;

// Line v->w, point p
// https://stackoverflow.com/questions/849211/shortest-distance-between-a-point-and-a-line-segment
fn closest_point_on_line(v: vec2<f32>, w: vec2<f32>, p: vec2<f32>) -> vec2<f32> {
    let c = v - w;
    // length squared
    let l2 = dot(c, c);
    if (l2 == 0.0) {
        return v;
    }
    let t = max(0.0, min(1.0, dot(p - v, w - v) / l2));
    let projection = v + t * (w - v);
    return projection;
}

fn draw_particle_circle(pos: vec2<f32>, draw_pos: vec2<f32>, radius: f32) {
    let y_start = draw_pos.y - radius;
    let y_end = draw_pos.y + radius;
    let x_start = draw_pos.x - radius;
    let x_end = draw_pos.x + radius;
    if (pos.x >= x_start && pos.x <= x_end && pos.y >= y_start && pos.y <= y_end) {
        let diff = pos - draw_pos;
        let dist = length(diff);
        if (round(dist) <= radius) {
            textureStore(data_in, vec2<i32>(pos), vec4<f32>(1.0, 0.0, 0.0, 1.0));
        }
    }
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>)
{
    let pixel = vec2<u32>(invocation_id.xy);
    let size = vec2<u32>(textureDimensions(data_in));
    if (pixel.x >= size.x && pixel.y >= size.y) {
        return ;
    }
    // Draw circle
    if (pc.draw_radius > 0.0) {
        let pos = vec2<f32>(pixel);
        let point_on_line = closest_point_on_line(pc.draw_start, pc.draw_end, pos);
        draw_particle_circle(pos, point_on_line, pc.draw_radius);
    }
}