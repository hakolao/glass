use std::sync::Arc;

use wgpu::{
    CompositeAlphaMode, CreateSurfaceError, Device, PresentMode, Surface, SurfaceConfiguration,
    TextureFormat,
};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
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
    pub surface_format: TextureFormat,
    pub desired_maximum_frame_latency: u32,
    pub max_size: Option<LogicalSize<u32>>,
    pub min_size: Option<LogicalSize<u32>>,
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
            surface_format: GlassWindow::default_surface_format(),
            desired_maximum_frame_latency: 2,
            exit_on_esc: false,
            max_size: None,
            min_size: None,
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
    window: Arc<Window>,
    surface: Surface<'static>,
    present_mode: PresentMode,
    alpha_mode: CompositeAlphaMode,
    surface_format: TextureFormat,
    desired_maximum_frame_latency: u32,
    exit_on_esc: bool,
    has_focus: bool,
    last_surface_size: [u32; 2],
}

impl GlassWindow {
    /// Creates a new [`GlassWindow`] that owns the winit [`Window`](winit::window::Window).
    pub fn new(
        context: &DeviceContext,
        config: WindowConfig,
        window: Arc<Window>,
    ) -> Result<GlassWindow, CreateSurfaceError> {
        let size = [window.inner_size().width, window.inner_size().height];
        let surface = context.instance().create_surface(window.clone())?;
        let allowed_formats = GlassWindow::allowed_surface_formats();
        if !(config.surface_format == allowed_formats[0]
            || config.surface_format == allowed_formats[1])
        {
            panic!(
                "{:?} not allowed. Surface should be created with either: {:?} or {:?}",
                config.surface_format, allowed_formats[0], allowed_formats[1]
            );
        }
        Ok(GlassWindow {
            window,
            surface,
            present_mode: config.present_mode,
            alpha_mode: config.alpha_mode,
            surface_format: config.surface_format,
            exit_on_esc: config.exit_on_esc,
            desired_maximum_frame_latency: config.desired_maximum_frame_latency,
            has_focus: false,
            last_surface_size: size,
        })
    }

    /// Configure surface after resize events
    pub(crate) fn configure_surface_with_size(&mut self, device: &Device, size: PhysicalSize<u32>) {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            width: size.width,
            height: size.height,
            present_mode: self.present_mode,
            alpha_mode: self.alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: self.desired_maximum_frame_latency,
        };
        self.configure_surface(device, &config);
        self.last_surface_size = [size.width, size.height];
    }

    /// Configure surface after window has changed. Use this to reconfigure the surface
    pub(crate) fn configure_surface(&mut self, device: &Device, config: &SurfaceConfiguration) {
        self.surface.configure(device, config);
        self.present_mode = config.present_mode;
        self.alpha_mode = config.alpha_mode;
        self.surface_format = config.format;
        self.desired_maximum_frame_latency = config.desired_maximum_frame_latency;
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
                    let size = self.window.inner_size().to_logical(monitor.scale_factor());
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

    /// Return allowed [`TextureFormat`](wgpu::TextureFormat)s for the surface.
    /// These are `Bgra8UnormSrgb` and `Bgra8Unorm`
    pub fn allowed_surface_formats() -> [TextureFormat; 2] {
        [TextureFormat::Bgra8UnormSrgb, TextureFormat::Bgra8Unorm]
    }

    /// Return default [`TextureFormat`](wgpu::TextureFormat)s
    pub fn default_surface_format() -> TextureFormat {
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
) -> LogicalPosition<i32> {
    let size: LogicalSize<i32> = monitor.size().to_logical(monitor.scale_factor());
    let lt_x = size.width / 2 - window_width as i32 / 2;
    let lt_y = size.height / 2 - window_height as i32 / 2;
    LogicalPosition::new(lt_x, lt_y)
}

pub fn get_fitting_videomode(
    monitor: &winit::monitor::MonitorHandle,
    width: u32,
    height: u32,
) -> winit::monitor::VideoModeHandle {
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

pub fn get_best_videomode(
    monitor: &winit::monitor::MonitorHandle,
) -> winit::monitor::VideoModeHandle {
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
