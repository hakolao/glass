pub mod device_context;
mod glass;
mod glass_app;

pub mod pipelines;
pub mod texture;
pub mod utils;
pub mod window;

// For convenience, export egui libs when that feature is enabled
#[cfg(feature = "egui_gui")]
pub use egui;
#[cfg(all(feature = "egui_demo_lib", feature = "egui_gui"))]
pub use egui_demo_lib;
#[cfg(all(feature = "egui_extra", feature = "egui_gui"))]
pub use egui_extras;
#[cfg(all(feature = "egui_extra", feature = "egui_gui"))]
pub use egui_plot;
#[cfg(feature = "egui_gui")]
pub use egui_wgpu;
#[cfg(feature = "egui_gui")]
pub use egui_winit;
// --
pub use image;
pub use wgpu;
pub use winit;

pub use crate::{glass::*, glass_app::*};
