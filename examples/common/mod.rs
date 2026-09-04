//! Bits shared between the examples.
//!
//! Not part of the `glass` API. Examples pull this in with
//! `#[path = "../common/mod.rs"] mod common;`, so anything unused by a given example would warn;
//! `#![allow(dead_code)]` keeps each example free to use only what it needs.

#![allow(dead_code)]

/// Maps OpenGL's `-1..1` depth range onto the `0..1` range wgpu expects.
///
/// `glam`'s orthographic projections follow the OpenGL convention, so every projection built here
/// is pre-multiplied by this.
pub const OPENGL_TO_WGPU: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.5, 0.0, //
    0.0, 0.0, 0.5, 1.0,
]);

/// An orthographic projection covering `screen_size` pixels, with the origin at the centre of the
/// screen and y pointing up.
pub fn camera_projection(screen_size: [f32; 2]) -> glam::Mat4 {
    let half_width = screen_size[0] / 2.0;
    let half_height = screen_size[1] / 2.0;
    OPENGL_TO_WGPU
        * glam::camera::rh::proj::directx::orthographic(
            -half_width,
            half_width,
            -half_height,
            half_height,
            0.0,
            1000.0,
        )
}
