use std::sync::Arc;

use wgpu::{
    CommandBuffer, CommandEncoder, CompositeAlphaMode, CreateSurfaceError, Device, PresentMode,
    Queue, Surface, SurfaceConfiguration, SurfaceTexture, TextureFormat,
};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    monitor::MonitorHandle,
    window::{Fullscreen, Window},
};

use crate::{device_context::DeviceContext, GlassApp};

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
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
            title: "App".to_string(),
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
    allowed_formats: Vec<TextureFormat>,
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
        let formats = surface.get_capabilities(&context.adapter()).formats;
        if !formats.contains(&config.surface_format) {
            panic!(
                "{:?} not allowed. Allowed formats: {:?}",
                config.surface_format, formats
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
            allowed_formats: formats,
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
        if !self.allowed_formats.contains(&config.format) {
            panic!(
                "{:?} not allowed. Allowed formats: {:?}",
                config.format, self.allowed_formats
            );
        }
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

    /// Return allowed texture formats for this window [`Surface`](wgpu::Surface)
    pub fn allowed_formats(&self) -> &Vec<TextureFormat> {
        &self.allowed_formats
    }

    /// Return [`Surface`](wgpu::Surface) belonging to the window
    pub fn surface(&self) -> &Surface<'_> {
        &self.surface
    }

    /// Return [`Window`](winit::window::Window)
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Return [`Window`](winit::window::Window) arc
    pub fn window_arc(&self) -> &Arc<Window> {
        &self.window
    }

    /// Return [`PresentMode`](wgpu::PresentMode) belonging to the window
    pub fn present_mode(&self) -> PresentMode {
        self.present_mode
    }

    /// Return default [`TextureFormat`](wgpu::TextureFormat)s
    pub fn default_surface_format() -> TextureFormat {
        #[cfg(target_os = "linux")]
        {
            let is_wayland = std::env::var("XDG_SESSION_TYPE")
                .map(|s| s == "wayland")
                .unwrap_or_else(|_| {
                    std::env::var("WAYLAND_DISPLAY")
                        .map(|s| !s.is_empty())
                        .unwrap_or(false)
                });

            if is_wayland {
                return TextureFormat::Bgra8Unorm;
            }
        }

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

    pub fn render_default<T: GlassApp>(
        &self,
        device: &Device,
        queue: &Queue,
        app: &mut T,
        mut render_function: impl FnMut(&mut T, RenderData) -> Option<Vec<CommandBuffer>>,
    ) {
        match self.surface().get_current_texture() {
            Ok(frame) => {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Commands"),
                });
                let mut commands = render_function(app, RenderData {
                    encoder: &mut encoder,
                    window: self,
                    frame: &frame,
                })
                .unwrap_or_default();
                commands.push(encoder.finish());
                queue.submit(commands);
                frame.present();
            }
            Err(error) => {
                if error == wgpu::SurfaceError::OutOfMemory {
                    panic!("Swapchain error: {error}. Rendering cannot continue.")
                }
            }
        }
        self.window().request_redraw();
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

/// Just a util struct to pass data required for rendering
pub struct RenderData<'a> {
    pub encoder: &'a mut CommandEncoder,
    pub window: &'a GlassWindow,
    pub frame: &'a SurfaceTexture,
}
