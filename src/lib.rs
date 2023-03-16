pub mod device_context;
mod glass;
mod glass_app;
#[cfg(feature = "iced_gui")]
pub mod iced_utils;
pub mod pipelines;
pub mod texture;
pub mod utils;
pub mod window;

// For convenience, export egui libs when that feature is enabled
#[cfg(all(feature = "egui_gui", not(feature = "iced_gui")))]
pub use egui;
#[cfg(all(feature = "egui_gui", not(feature = "iced_gui")))]
pub use egui_wgpu;
#[cfg(all(feature = "egui_gui", not(feature = "iced_gui")))]
pub use egui_winit;
#[cfg(all(feature = "iced_gui", not(feature = "egui_gui")))]
pub use iced_graphics;
// For convenience, export iced when that feature is enabled
#[cfg(all(feature = "iced_gui", not(feature = "egui_gui")))]
pub use iced_native;
#[cfg(all(feature = "iced_gui", not(feature = "egui_gui")))]
pub use iced_wgpu;
#[cfg(all(feature = "iced_gui", not(feature = "egui_gui")))]
pub use iced_winit;
// --
pub use image;
pub use wgpu;
pub use winit;

pub use crate::{glass::*, glass_app::*};
