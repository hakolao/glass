use glam::IVec2;
use wgpu::{
    CompositeAlphaMode, CreateSurfaceError, Device, PresentMode, Surface, SurfaceConfiguration,
    TextureFormat,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    monitor::MonitorHandle,
    window::{Fullscreen, Window},
};

use crate::device_context::DeviceContext;

#[derive(Debug, Copy, Clone)]
pub struct WindowConfig {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub pos: WindowPos,
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
            pos: WindowPos::Centered,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
            exit_on_esc: false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WindowPos {
    Centered,
    FullScreen,
    SizedFullScreen,
    FullScreenBorderless,
    Maximized,
    Pos(PhysicalPosition<u32>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SurfaceError {
    /// A timeout was encountered while trying to acquire the next frame.
    Timeout,
    /// The underlying surface has changed, and therefore the swap chain must be updated.
    Outdated,
    /// The swap chain has been lost and needs to be recreated.
    Lost,
    /// There is no more memory left to allocate a new frame.
    OutOfMemory,
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
    pub fn new(
        context: &DeviceContext,
        config: WindowConfig,
        window: Window,
    ) -> Result<GlassWindow, CreateSurfaceError> {
        let size = [window.inner_size().width, window.inner_size().height];
        let surface = unsafe {
            match context.instance().create_surface(&window) {
                Ok(surface) => surface,
                Err(e) => return Err(e),
            }
        };
        Ok(GlassWindow {
            window,
            surface,
            present_mode: config.present_mode,
            alpha_mode: config.alpha_mode,
            exit_on_esc: config.exit_on_esc,
            has_focus: false,
            last_surface_size: size,
        })
    }

    /// Configure surface after resize events
    pub(crate) fn configure_surface_with_size(&mut self, device: &Device, size: PhysicalSize<u32>) {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: Self::surface_format(),
            width: size.width,
            height: size.height,
            present_mode: self.present_mode,
            alpha_mode: self.alpha_mode,
            view_formats: vec![],
        };
        self.configure_surface(device, &config);
        self.last_surface_size = [size.width, size.height];
    }

    /// Configure surface after window has changed. Use this to reconfigure the surface
    pub(crate) fn configure_surface(&mut self, device: &Device, config: &SurfaceConfiguration) {
        self.surface.configure(device, config);
        self.present_mode = config.present_mode;
        self.alpha_mode = config.alpha_mode;
        self.last_surface_size = [config.width, config.height];
    }

    pub fn set_position(&self, window_position: WindowPos) {
        match window_position {
            WindowPos::Maximized => {
                self.window.set_fullscreen(None);
                self.window.set_maximized(true)
            }
            WindowPos::FullScreen => {
                if let Some(monitor) = self.window.current_monitor() {
                    self.window
                        .set_fullscreen(Some(Fullscreen::Exclusive(get_best_videomode(&monitor))));
                }
            }
            WindowPos::SizedFullScreen => {
                if let Some(monitor) = self.window.current_monitor() {
                    let size = self.window.inner_size();
                    self.window
                        .set_fullscreen(Some(Fullscreen::Exclusive(get_fitting_videomode(
                            &monitor,
                            size.width,
                            size.height,
                        ))));
                }
            }
            WindowPos::FullScreenBorderless => self
                .window
                .set_fullscreen(Some(Fullscreen::Borderless(self.window.current_monitor()))),
            WindowPos::Pos(pos) => {
                self.window.set_fullscreen(None);
                self.window.set_outer_position(pos)
            }
            WindowPos::Centered => {
                if let Some(monitor) = self.window.current_monitor() {
                    self.window.set_fullscreen(None);
                    let size = self.window.inner_size();
                    self.window.set_outer_position(get_centered_window_position(
                        &monitor,
                        size.width,
                        size.height,
                    ));
                }
            }
        };
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

pub fn get_centered_window_position(
    monitor: &MonitorHandle,
    window_width: u32,
    window_height: u32,
) -> PhysicalPosition<i32> {
    let size = monitor.size();
    let center = IVec2::new(size.width as i32, size.height as i32) / 2;
    let window_size = PhysicalSize::new(window_width, window_height);
    let left_top = center - IVec2::new(window_size.width as i32, window_size.height as i32) / 2;
    PhysicalPosition::new(left_top.x, left_top.y)
}

pub fn get_fitting_videomode(
    monitor: &winit::monitor::MonitorHandle,
    width: u32,
    height: u32,
) -> winit::monitor::VideoMode {
    let mut modes = monitor.video_modes().collect::<Vec<_>>();

    fn abs_diff(a: u32, b: u32) -> u32 {
        if a > b {
            return a - b;
        }
        b - a
    }

    modes.sort_by(|a, b| {
        use std::cmp::Ordering::*;
        match abs_diff(a.size().width, width).cmp(&abs_diff(b.size().width, width)) {
            Equal => {
                match abs_diff(a.size().height, height).cmp(&abs_diff(b.size().height, height)) {
                    Equal => b
                        .refresh_rate_millihertz()
                        .cmp(&a.refresh_rate_millihertz()),
                    default => default,
                }
            }
            default => default,
        }
    });

    modes.first().unwrap().clone()
}

pub fn get_best_videomode(monitor: &winit::monitor::MonitorHandle) -> winit::monitor::VideoMode {
    let mut modes = monitor.video_modes().collect::<Vec<_>>();
    modes.sort_by(|a, b| {
        use std::cmp::Ordering::*;
        match b.size().width.cmp(&a.size().width) {
            Equal => match b.size().height.cmp(&a.size().height) {
                Equal => b
                    .refresh_rate_millihertz()
                    .cmp(&a.refresh_rate_millihertz()),
                default => default,
            },
            default => default,
        }
    });

    modes.first().unwrap().clone()
}
