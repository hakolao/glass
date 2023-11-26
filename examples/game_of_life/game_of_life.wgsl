struct PushConstants {
    draw_start: vec2<f32>,
    draw_end: vec2<f32>,
    draw_radius: f32,
}
var<push_constant> pc: PushConstants;

@group(0) @binding(0)
var image: texture_storage_2d<rgba16float, read_write>;
@group(0) @binding(1)
var data_in: texture_storage_2d<rgba16float, read_write>;

// https://stackoverflow.com/questions/4200224/random-noise-functions-for-glsl
fn randomFloat(xy: vec2<f32>, seed: f32) -> f32 {
    let offset = vec2<f32>(0.12345, 0.54321);
    let xy_new = xy + offset;
    // Golden ratio
    var PHI = 1.61803398874989484820459;
    return abs(fract(tan(distance(xy * PHI, xy_new) * (seed + 0.765831)) * xy_new.x));
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let randomNumber = randomFloat(vec2<f32>(invocation_id.xy), 0.5);
    let alive = randomNumber > 0.9;
    let color = vec4<f32>(f32(alive), 0.0, 0.0, 1.0);

    textureStore(data_in, location, color);
}

fn is_alive(location: vec2<i32>, offset_x: i32, offset_y: i32) -> i32 {
    let value: vec4<f32> = textureLoad(data_in, location + vec2<i32>(offset_x, offset_y));
    return i32(value.x);
}

fn count_alive(location: vec2<i32>) -> i32 {
    return is_alive(location, -1, -1) +
           is_alive(location, -1,  0) +
           is_alive(location, -1,  1) +
           is_alive(location,  0, -1) +
           is_alive(location,  0,  1) +
           is_alive(location,  1, -1) +
           is_alive(location,  1,  0) +
           is_alive(location,  1,  1);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let n_alive = count_alive(location);

    var alive: bool;
    if (n_alive == 3) {
        alive = true;
    } else if (n_alive == 2) {
        let currently_alive = is_alive(location, 0, 0);
        alive = bool(currently_alive);
    } else {
        alive = false;
    }
    let color = vec4<f32>(f32(alive), 0.0, 0.0, 1.0);

//    storageBarrier();

    textureStore(image, location, color);
}