struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
};

// https://github.com/bevyengine/bevy/blob/09df19bcadb52d2f4dbbc899aef74cafa9091538/crates/bevy_core_pipeline/src/fullscreen_vertex_shader/fullscreen.wgsl
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return VertexOutput(clip_position, uv);
}

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;

struct PushConstants {
    off: u32,
    exposure: f32,
    gamma: f32,
    pre_saturation: f32,
    post_saturation: f32,
}
var<push_constant> pc: PushConstants;

// pow() but safe for NaNs/negatives
fn powsafe(color: vec3<f32>, power: f32) -> vec3<f32> {
    return pow(abs(color), vec3(power)) * sign(color);
}

/*
    Increase color saturation of the given color data.
    :param color: expected sRGB primaries input
    :param saturationAmount: expected 0-1 range with 1=neutral, 0=no saturation.
    -- ref[2] [4]
*/
fn saturation(color: vec3<f32>, saturationAmount: f32) -> vec3<f32> {
    let luma = tonemapping_luminance(color);
    return mix(vec3(luma), color, vec3(saturationAmount));
}

// luminance coefficients from Rec. 709.
// https://en.wikipedia.org/wiki/Rec._709
fn tonemapping_luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn tonemapping_change_luminance(c_in: vec3<f32>, l_out: f32) -> vec3<f32> {
    let l_in = tonemapping_luminance(c_in);
    return c_in * (l_out / l_in);
}

fn tonemapping_reinhard_luminance(color: vec3<f32>) -> vec3<f32> {
    let l_old = tonemapping_luminance(color);
    let l_new = l_old / (1.0 + l_old);
    return tonemapping_change_luminance(color, l_new);
}

// Source: Advanced VR Rendering, GDC 2015, Alex Vlachos, Valve, Slide 49
// https://media.steampowered.com/apps/valve/2015/Alex_Vlachos_Advanced_VR_Rendering_GDC2015.pdf
fn screen_space_dither(frag_coord: vec2<f32>) -> vec3<f32> {
    var dither = vec3<f32>(dot(vec2<f32>(171.0, 231.0), frag_coord)).xxx;
    dither = fract(dither.rgb / vec3<f32>(103.0, 71.0, 97.0));
    return (dither - 0.5) / 255.0;
}

fn tone_mapping(in: vec4<f32>) -> vec4<f32> {
    var color = max(in.rgb, vec3(0.0));
    // Linear pre tonemapping grading
    color = saturation(color, pc.pre_saturation);
    color = powsafe(color, pc.gamma);
    color = color * powsafe(vec3(2.0), pc.exposure);
    color = max(color, vec3(0.0));
    // tone_mapping
    color = tonemapping_reinhard_luminance(color.rgb);
    // Perceptual post tonemapping grading
    color = saturation(color, pc.post_saturation);

    return vec4(color, in.a);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);
    if (pc.off == u32(1)) {
        return hdr_color;
    }

    var output_rgb = tone_mapping(hdr_color).rgb;

    // Deband dither
    output_rgb = powsafe(output_rgb.rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb.rgb, 2.2);

    return vec4<f32>(output_rgb, hdr_color.a);
}
