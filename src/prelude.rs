//! Everything a typical app needs, in one import.
//!
//! ```
//! use glass::prelude::*;
//! ```
//!
//! This covers running an app, configuring windows and devices, and the built-in pipelines. Reach
//! into the individual modules for the rest: [`crate::error`] for the boxed error payloads,
//! [`crate::utils`] for the default-format helpers, and [`crate::window`] for the video-mode
//! functions.

pub use crate::{
    context::{GlassConfig, GlassContext},
    device_context::{DeviceConfig, DeviceContext},
    error::GlassError,
    glass::Glass,
    glass_app::GlassApp,
    pipelines::{
        ColoredVertex, Line, LinePipeline, QuadPipeline, TexturedVertex, QUAD_INDICES,
        TEXTURED_QUAD_VERTICES,
    },
    texture::{Texture, TextureDesc},
    window::{GlassWindow, RenderData, WindowConfig, WindowPos},
};
