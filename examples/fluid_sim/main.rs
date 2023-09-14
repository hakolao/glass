mod app;
mod camera;
mod circle_pipeline;
mod color;
mod fluid_sim;
mod post_processing;
mod rectangle_pipeline;
mod simple_vertex;
mod timer;

use glass::{
    device_context::DeviceConfig,
    wgpu,
    wgpu::{Backends, Limits, PowerPreference, PresentMode},
    window::WindowConfig,
    Glass, GlassConfig, GlassError,
};

use crate::app::{FluidSimApp, HEIGHT, WIDTH};

fn config() -> GlassConfig {
    GlassConfig {
        device_config: DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            limits: Limits {
                max_push_constant_size: 128,
                ..Limits::default()
            },
            backends: Backends::all(),
        },
        window_configs: vec![WindowConfig {
            width: WIDTH,
            height: HEIGHT,
            exit_on_esc: true,
            present_mode: PresentMode::AutoVsync,
            ..WindowConfig::default()
        }],
    }
}

fn main() -> Result<(), GlassError> {
    Glass::new(FluidSimApp::new(), config()).run()
}
