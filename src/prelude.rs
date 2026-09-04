//! Everything a typical app needs, in one import.
//!
//! ```
//! use glass::prelude::*;
//! ```
//!
//! This covers running an app and configuring windows and devices. Reach into the
//! individual modules for the rest: [`crate::error`] for the boxed error payloads,
//! [`crate::utils`] for the default-format helpers, and [`crate::window`] for the
//! video-mode functions.

pub use crate::{
    context::{GlassConfig, GlassContext},
    device_context::{DeviceConfig, DeviceContext},
    error::GlassError,
    glass::Glass,
    glass_app::GlassApp,
    texture::{Texture, TextureDesc},
    window::{GlassWindow, RenderData, WindowConfig, WindowPos},
};
