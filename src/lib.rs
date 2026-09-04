#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links
)]

mod context;
pub mod device_context;
pub mod error;
mod glass;
mod glass_app;
pub mod pipelines;
pub mod prelude;
pub mod texture;
pub mod utils;
pub mod window;

// `wgpu` and `winit` types appear throughout this crate's API, so their versions are part of
// `glass`'s semver contract. Use these re-exports rather than depending on the crates separately,
// to be sure you are using the same versions `glass` was built against.
pub use wgpu;
pub use winit;

pub use crate::{
    context::{GlassConfig, GlassContext},
    device_context::{DeviceConfig, DeviceContext},
    error::GlassError,
    glass::Glass,
    glass_app::GlassApp,
    texture::Texture,
    window::{GlassWindow, RenderData, WindowConfig, WindowPos},
};
