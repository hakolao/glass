pub mod device_context;
mod glass;
mod glass_app;
#[cfg(feature = "iced_gui")]
pub mod iced_utils;
pub mod texture;
pub mod utils;
pub mod window;

// For convenience, export egui libs when that feature is enabled
#[cfg(feature = "egui_gui")]
pub use egui;
#[cfg(feature = "egui_gui")]
pub use egui_wgpu;
#[cfg(feature = "egui_gui")]
pub use egui_winit;
#[cfg(feature = "iced_gui")]
pub use iced_graphics;
// For convenience, export iced when that feature is enabled
#[cfg(feature = "iced_gui")]
pub use iced_native;
#[cfg(feature = "iced_gui")]
pub use iced_wgpu;
#[cfg(feature = "iced_gui")]
pub use iced_winit;
// --
// For convenience export winit and wgpu
pub use wgpu;
pub use winit;

pub use crate::{glass::*, glass_app::*};
