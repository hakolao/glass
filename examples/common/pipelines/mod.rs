//! Render pipelines for a textured quad and for lines.
//!
//! These used to live in `glass` itself, but drawing is the application's job, not the
//! windowing layer's. They are kept here as worked examples to copy from.
//!
//! Both need [`wgpu::Features::IMMEDIATES`], which an app must request through
//! [`DeviceConfig::features`](glass::DeviceConfig::features).

mod line;
mod quad;
mod vertex;

pub use line::{Line, LinePipeline, LinePushConstants};
pub use quad::{QuadPipeline, QuadPushConstants};
pub use vertex::{
    ColoredVertex, SimpleTexturedVertex, TexturedVertex, FULL_SCREEN_TRIANGLE_VERTICES,
    QUAD_INDICES, TEXTURED_QUAD_VERTICES,
};
