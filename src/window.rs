use wgpu::{CompositeAlphaMode, PresentMode, Surface, SurfaceConfiguration, TextureFormat};
use winit::{dpi::PhysicalSize, window::Window};

use crate::device_context::DeviceContext;

// ToDo: Add more options
#[derive(Debug, Copy, Clone)]
pub struct WindowConfig {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub present_mode: PresentMode,
    pub alpha_mode: CompositeAlphaMode,
    pub exit_on_esc: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "App",
            width: 1920,
            height: 1080,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
            exit_on_esc: false,
        }
    }
}

pub struct GlassWindow {
    window: Window,
    surface: Surface,
    present_mode: PresentMode,
    alpha_mode: CompositeAlphaMode,
    exit_on_esc: bool,
    has_focus: bool,
    last_surface_size: [u32; 2],
}

impl GlassWindow {
    /// Creates a new [`GlassWindow`] that owns the winit [`Window`](winit::window::Window).
    pub fn new(context: &DeviceContext, config: WindowConfig, window: Window) -> GlassWindow {
        let size = [window.inner_size().width, window.inner_size().height];
        let surface = unsafe { context.instance().create_surface(&window) };
        GlassWindow {
            window,
            surface,
            present_mode: config.present_mode,
            alpha_mode: config.alpha_mode,
            exit_on_esc: config.exit_on_esc,
            has_focus: false,
            last_surface_size: size,
        }
    }

    /// Configure surface after resize events
    pub(crate) fn configure_surface_with_size(
        &mut self,
        context: &DeviceContext,
        size: PhysicalSize<u32>,
    ) {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: Self::surface_format(),
            width: size.width,
            height: size.height,
            present_mode: self.present_mode,
            alpha_mode: self.alpha_mode,
        };
        self.configure_surface(context, &config);
        self.last_surface_size = [size.width, size.height];
    }

    /// Configure surface after window has changed. Use this to reconfigure the surface
    pub fn configure_surface(&mut self, context: &DeviceContext, config: &SurfaceConfiguration) {
        self.surface.configure(context.device(), config);
        self.present_mode = config.present_mode;
        self.alpha_mode = config.alpha_mode;
        self.last_surface_size = [config.width, config.height];
    }

    /// Return [`Surface`](wgpu::Surface) belonging to the window
    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    /// Return [`Window`](winit::window::Window)
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Return [`PresentMode`](wgpu::PresentMode) belonging to the window
    pub fn present_mode(&self) -> PresentMode {
        self.present_mode
    }

    /// Return [`TextureFormat`](wgpu::TextureFormat) belonging to the window surface
    pub fn surface_format() -> TextureFormat {
        TextureFormat::Bgra8UnormSrgb
    }

    pub(crate) fn exit_on_esc(&self) -> bool {
        self.exit_on_esc
    }

    pub fn is_focused(&self) -> bool {
        self.has_focus
    }

    pub(crate) fn set_focus(&mut self, has_focus: bool) {
        self.has_focus = has_focus;
    }

    pub fn surface_size(&self) -> [u32; 2] {
        self.last_surface_size
    }
}
