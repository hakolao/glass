#include examples/shader_with_includes/consts.wgsl
#include examples/triangle/triangle.wgsl

fn test_function(some_vec: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(some_vec, 1.0);
}