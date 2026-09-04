//! Ready-made render pipelines for the two things almost every app needs: drawing a textured
//! quad and drawing lines.
//!
//! Both need [`wgpu::Features::IMMEDIATES`], which [`GlassContext`](crate::GlassContext) requests
//! for you.

mod line;
mod quad;
mod vertex;

pub use line::{Line, LinePipeline, LinePushConstants};
pub use quad::{QuadPipeline, QuadPushConstants};
pub use vertex::{
    ColoredVertex, SimpleTexturedVertex, TexturedVertex, FULL_SCREEN_TRIANGLE_VERTICES,
    QUAD_INDICES, TEXTURED_QUAD_VERTICES,
};
