//! Window configuration and the [`GlassWindow`] type that pairs a winit window with its wgpu
//! surface.

use std::{cmp::Reverse, sync::Arc};

use wgpu::{
    CommandBuffer, CommandEncoder, CompositeAlphaMode, Device, PresentMode, Surface,
    SurfaceConfiguration, SurfaceTexture, TextureFormat,
};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    monitor::{MonitorHandle, VideoModeHandle},
    window::{Fullscreen, Window, WindowAttributes},
};

use crate::{device_context::DeviceContext, GlassError};

/// How a window is created: its size, position, surface and input behaviour.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Title shown in the window's title bar.
    pub title: String,
    /// Initial width in logical pixels.
    pub width: u32,
    /// Initial height in logical pixels.
    pub height: u32,
    /// Where the window is placed, including the fullscreen variants.
    pub pos: WindowPos,
    /// The surface the window renders through. Its `format` must be one the surface supports;
    /// [`GlassWindow::default_surface_format`] gives a format that works on the current platform.
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Upper bound on the window's inner size, if any.
    pub max_size: Option<LogicalSize<u32>>,
    /// Lower bound on the window's inner size, if any.
    pub min_size: Option<LogicalSize<u32>>,
    /// Close this window when <kbd>Esc</kbd> is pressed while it has focus.
    pub exit_on_esc: bool,
    /// Keep the window hidden until its first frame has been presented, which avoids a flash of
    /// unrendered background at startup. Requires that you call
    /// [`GlassWindow::mark_first_frame_presented`] (or use [`GlassWindow::render_default`], which
    /// does it for you).
    pub hide_until_first_frame: bool,
    /// Any further winit attributes, applied before the fields above. Use this for settings
    /// `glass` does not expose, such as window icons or decorations.
    pub other_attributes: Option<WindowAttributes>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "App".to_string(),
            width: 1920,
            height: 1080,
            pos: WindowPos::Centered,
            surface_config: SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: GlassWindow::default_surface_format(),
                color_space: Default::default(),
                width: 1920,
                height: 1080,
                present_mode: PresentMode::AutoVsync,
                desired_maximum_frame_latency: 2,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
            },
            exit_on_esc: false,
            hide_until_first_frame: true,
            max_size: None,
            min_size: None,
            other_attributes: None,
        }
    }
}

/// Where a window is placed on creation.
#[derive(Debug, Copy, Clone)]
pub enum WindowPos {
    /// Centered on the primary monitor.
    Centered,
    /// Exclusive fullscreen at the monitor's best video mode. Falls back to
    /// [`WindowPos::FullScreenBorderless`] if the monitor reports no video modes.
    FullScreen,
    /// Exclusive fullscreen at the video mode closest to the configured size. Falls back to
    /// [`WindowPos::FullScreenBorderless`] if the monitor reports no video modes.
    SizedFullScreen,
    /// Borderless fullscreen on the primary monitor.
    FullScreenBorderless,
    /// Maximized, but still a normal window.
    Maximized,
    /// At an explicit physical position.
    Pos(PhysicalPosition<u32>),
}

/// A winit [`Window`] paired with the wgpu [`Surface`] it renders through.
#[derive(Debug)]
pub struct GlassWindow {
    name: String,
    window: Arc<Window>,
    surface: Surface<'static>,
    device_context: Arc<DeviceContext>,
    surface_config: wgpu::SurfaceConfiguration,
    exit_on_esc: bool,
    has_focus: bool,
    hide_until_first_frame: bool,
    first_frame_presented: bool,
    last_surface_size: [u32; 2],
}

impl GlassWindow {
    /// Creates a new [`GlassWindow`] that owns the winit [`Window`].
    ///
    /// You normally do not call this: [`GlassContext::create_window`] and
    /// [`GlassContext::create_window_immediately`] do it for you.
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::SurfaceError`] if no surface can be created for the window, or
    /// [`GlassError::UnsupportedSurfaceFormat`] if `config.surface_config.format` is not among
    /// the formats the surface reports.
    ///
    /// [`GlassContext::create_window`]: crate::GlassContext::create_window
    /// [`GlassContext::create_window_immediately`]: crate::GlassContext::create_window_immediately
    pub fn new(
        context: &Arc<DeviceContext>,
        name: String,
        config: WindowConfig,
        window: Arc<Window>,
    ) -> Result<GlassWindow, GlassError> {
        let size = [window.inner_size().width, window.inner_size().height];
        let surface = context.instance().create_surface(window.clone())?;
        let supported = surface.get_capabilities(context.adapter()).formats;
        if !supported.contains(&config.surface_config.format) {
            return Err(GlassError::UnsupportedSurfaceFormat {
                requested: config.surface_config.format,
                supported,
            });
        }
        Ok(GlassWindow {
            name,
            device_context: context.clone(),
            window,
            surface,
            surface_config: config.surface_config,
            exit_on_esc: config.exit_on_esc,
            has_focus: false,
            hide_until_first_frame: config.hide_until_first_frame,
            first_frame_presented: false,
            last_surface_size: size,
        })
    }

    /// Name this window was created under via [`GlassContext::create_window`].
    ///
    /// [`GlassContext::create_window`]: crate::GlassContext::create_window
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The surface configuration currently in effect.
    pub fn surface_config(&self) -> &SurfaceConfiguration {
        &self.surface_config
    }

    /// Recreates surface after e.g device lost events
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::SurfaceError`] if the new surface cannot be created, or
    /// [`GlassError::UnsupportedSurfaceFormat`] if `config.format` is unsupported.
    pub(crate) fn recreate_surface(
        &mut self,
        device: &Device,
        config: &SurfaceConfiguration,
    ) -> Result<(), GlassError> {
        let window = self.window.clone();
        self.surface = self.device_context.instance().create_surface(window)?;
        self.configure_surface(device, config)?;
        Ok(())
    }

    /// Configure surface after resize events
    pub(crate) fn reconfigure_surface(&mut self, device: &Device) -> Result<(), GlassError> {
        self.configure_surface_with_size(
            device,
            PhysicalSize::new(self.last_surface_size[0], self.last_surface_size[1]),
        )?;
        Ok(())
    }

    /// Configure surface after resize events
    pub(crate) fn configure_surface_with_size(
        &mut self,
        device: &Device,
        size: PhysicalSize<u32>,
    ) -> Result<(), GlassError> {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_config.format,
            color_space: self.surface_config.color_space,
            width: size.width,
            height: size.height,
            present_mode: self.surface_config.present_mode,
            alpha_mode: self.surface_config.alpha_mode,
            desired_maximum_frame_latency: self.surface_config.desired_maximum_frame_latency,
            view_formats: vec![],
        };
        self.configure_surface(device, &config)?;
        Ok(())
    }

    /// Configure surface after window has changed. Use this to reconfigure the surface
    pub(crate) fn configure_surface(
        &mut self,
        device: &Device,
        config: &SurfaceConfiguration,
    ) -> Result<(), GlassError> {
        let supported = self.allowed_formats();
        if !supported.contains(&config.format) {
            return Err(GlassError::UnsupportedSurfaceFormat {
                requested: config.format,
                supported,
            });
        }
        self.surface.configure(device, config);
        self.surface_config = config.clone();
        self.last_surface_size = [config.width, config.height];
        Ok(())
    }

    /// Moves or resizes the window to `window_position`, including switching in and out of
    /// fullscreen. Falls back to borderless fullscreen when an exclusive video mode is requested
    /// on a monitor that reports none.
    pub fn set_position(&self, window_position: WindowPos) {
        match window_position {
            WindowPos::Maximized => {
                self.window.set_fullscreen(None);
                self.window.set_maximized(true)
            }
            WindowPos::FullScreen => {
                if let Some(monitor) = self.window.current_monitor() {
                    let mode = get_best_videomode(&monitor);
                    self.set_exclusive_or_borderless(mode);
                }
            }
            WindowPos::SizedFullScreen => {
                if let Some(monitor) = self.window.current_monitor() {
                    let size = self.window.inner_size();
                    let mode = get_fitting_videomode(&monitor, size.width, size.height);
                    self.set_exclusive_or_borderless(mode);
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

    fn set_exclusive_or_borderless(&self, mode: Option<VideoModeHandle>) {
        match mode {
            Some(mode) => self
                .window
                .set_fullscreen(Some(Fullscreen::Exclusive(mode))),
            None => self
                .window
                .set_fullscreen(Some(Fullscreen::Borderless(self.window.current_monitor()))),
        }
    }

    /// Return allowed texture formats for this window [`Surface`]
    pub fn allowed_formats(&self) -> Vec<TextureFormat> {
        self.surface
            .get_capabilities(self.device_context.adapter())
            .formats
    }

    /// Return [`Surface`] belonging to the window
    pub fn surface(&self) -> &Surface<'_> {
        &self.surface
    }

    /// Return [`Window`]
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Return the [`DeviceContext`] shared by every window.
    pub fn device_context(&self) -> &DeviceContext {
        &self.device_context
    }

    /// Return [`Window`] arc
    pub fn window_arc(&self) -> &Arc<Window> {
        &self.window
    }

    /// The surface format that works on the current platform.
    ///
    /// This is `Bgra8UnormSrgb` everywhere except on a Wayland session, which gets the non-sRGB
    /// `Bgra8Unorm`, since Wayland compositors commonly do not advertise the sRGB variant.
    pub fn default_surface_format() -> TextureFormat {
        crate::utils::default_surface_format()
    }

    pub(crate) fn exit_on_esc(&self) -> bool {
        self.exit_on_esc
    }

    /// `true` while this window has keyboard focus.
    pub fn is_focused(&self) -> bool {
        self.has_focus
    }

    pub(crate) fn set_focus(&mut self, has_focus: bool) {
        self.has_focus = has_focus;
    }

    /// The size the surface was last configured at, in physical pixels.
    pub fn surface_size(&self) -> [u32; 2] {
        self.last_surface_size
    }

    /// Acquires the next surface texture, runs `render_function` over it, submits and presents.
    ///
    /// Recoverable surface states are handled here: an occluded or timed-out surface skips the
    /// frame, a suboptimal or outdated one is reconfigured, and a lost one is recreated. None of
    /// those cases call `render_function`.
    ///
    /// Return extra [`CommandBuffer`]s from `render_function` to have them submitted before the
    /// encoder `glass` provides.
    pub fn render_default(
        &mut self,
        mut render_function: impl FnMut(RenderData<'_>) -> Option<Vec<CommandBuffer>>,
    ) {
        let device = self.device_context.device_arc();
        let queue = self.device_context.queue_arc();
        match self.surface().get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Commands"),
                });
                let mut commands = render_function(RenderData {
                    encoder: &mut encoder,
                    window: self,
                    frame: &frame,
                })
                .unwrap_or_default();
                commands.push(encoder.finish());
                queue.submit(commands);
                self.window().pre_present_notify();
                queue.present(frame);
                self.mark_first_frame_presented();
            }
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Suboptimal(_) | wgpu::CurrentSurfaceTexture::Outdated => {
                if let Err(e) = self.reconfigure_surface(&device) {
                    log::warn!("failed to reconfigure a stale surface: {e}");
                }
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                // `render_default` registers no error scope, so wgpu raises validation errors
                // through its own uncaptured-error handler and never reports them here.
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // Losing the surface is recoverable, so log and skip the frame rather than
                // bringing the whole app down with it.
                match self
                    .device_context
                    .instance()
                    .create_surface(self.window.clone())
                {
                    Ok(surface) => {
                        self.surface = surface;
                        if let Err(e) = self.reconfigure_surface(&device) {
                            log::warn!("failed to configure a recreated surface: {e}");
                        }
                    }
                    Err(e) => log::error!("failed to recreate a lost surface: {e}"),
                }
                return;
            }
        }

        self.window().request_redraw();
    }

    /// Reveal the window on its first presented frame, if `hide_until_first_frame` was set.
    /// Call once per frame after you present. No-op after the first call and when the flag is off.
    pub fn mark_first_frame_presented(&mut self) {
        if self.hide_until_first_frame && !self.first_frame_presented {
            self.window.set_visible(true);
        }
        self.first_frame_presented = true;
    }

    /// Once first frame has been marked as presented this returns `true`, otherwise `false`
    pub fn is_first_frame_presented(&self) -> bool {
        self.first_frame_presented
    }
}

/// The top-left position that centers a window of the given logical size on `monitor`.
///
/// The result can be negative when the window is larger than the monitor, which keeps the window
/// centered rather than clamping it into a corner.
pub fn get_centered_window_position(
    monitor: &MonitorHandle,
    window_width: u32,
    window_height: u32,
) -> LogicalPosition<i32> {
    let size: LogicalSize<i32> = monitor.size().to_logical(monitor.scale_factor());
    centered_position((size.width, size.height), (window_width, window_height))
}

/// The video mode closest to `width` x `height`, preferring the highest refresh rate among
/// equally close modes.
///
/// Returns `None` if the monitor reports no video modes, which happens on some compositors and
/// means exclusive fullscreen is unavailable.
pub fn get_fitting_videomode(
    monitor: &MonitorHandle,
    width: u32,
    height: u32,
) -> Option<VideoModeHandle> {
    monitor.video_modes().min_by_key(|mode| {
        fitting_mode_key(
            (mode.size().width, mode.size().height),
            mode.refresh_rate_millihertz(),
            (width, height),
        )
    })
}

/// The largest video mode the monitor offers, preferring the highest refresh rate among
/// equally large modes.
///
/// Returns `None` if the monitor reports no video modes, which happens on some compositors and
/// means exclusive fullscreen is unavailable.
pub fn get_best_videomode(monitor: &MonitorHandle) -> Option<VideoModeHandle> {
    monitor.video_modes().min_by_key(|mode| {
        best_mode_key(
            (mode.size().width, mode.size().height),
            mode.refresh_rate_millihertz(),
        )
    })
}

/// The arithmetic behind [`get_centered_window_position`], split out so it can be tested without
/// a monitor.
fn centered_position(monitor: (i32, i32), window: (u32, u32)) -> LogicalPosition<i32> {
    LogicalPosition::new(
        monitor.0 / 2 - window.0 as i32 / 2,
        monitor.1 / 2 - window.1 as i32 / 2,
    )
}

/// Sort key for [`get_fitting_videomode`]: closest width, then closest height, then highest
/// refresh rate. Smaller is better.
fn fitting_mode_key(
    mode_size: (u32, u32),
    refresh_rate_millihertz: u32,
    target: (u32, u32),
) -> (u32, u32, Reverse<u32>) {
    (
        mode_size.0.abs_diff(target.0),
        mode_size.1.abs_diff(target.1),
        Reverse(refresh_rate_millihertz),
    )
}

/// Sort key for [`get_best_videomode`]: largest width, then largest height, then highest refresh
/// rate. Smaller is better.
fn best_mode_key(
    mode_size: (u32, u32),
    refresh_rate_millihertz: u32,
) -> (Reverse<u32>, Reverse<u32>, Reverse<u32>) {
    (
        Reverse(mode_size.0),
        Reverse(mode_size.1),
        Reverse(refresh_rate_millihertz),
    )
}

/// Everything a [`GlassWindow::render_default`] render function needs to draw a frame.
#[derive(Debug)]
pub struct RenderData<'a> {
    /// Encoder whose commands are submitted after the ones you return.
    pub encoder: &'a mut CommandEncoder,
    /// The window being rendered.
    pub window: &'a GlassWindow,
    /// The acquired surface texture to render into.
    pub frame: &'a SurfaceTexture,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_a_window_that_fits() {
        let pos = centered_position((1920, 1080), (800, 600));
        assert_eq!(pos, LogicalPosition::new(560, 240));
    }

    #[test]
    fn centers_a_window_larger_than_the_monitor() {
        // Overhanging equally on both sides beats clamping into a corner, so a negative
        // coordinate here is the intended result.
        let pos = centered_position((1280, 720), (1920, 1080));
        assert_eq!(pos, LogicalPosition::new(-320, -180));
    }

    #[test]
    fn centering_is_exact_for_equal_sizes() {
        assert_eq!(
            centered_position((1920, 1080), (1920, 1080)),
            LogicalPosition::new(0, 0)
        );
    }

    #[test]
    fn fitting_mode_prefers_the_closest_resolution() {
        let target = (1280, 720);
        let exact = fitting_mode_key((1280, 720), 60_000, target);
        let close = fitting_mode_key((1366, 768), 60_000, target);
        let far = fitting_mode_key((3840, 2160), 60_000, target);
        assert!(exact < close);
        assert!(close < far);
    }

    #[test]
    fn fitting_mode_breaks_ties_on_refresh_rate() {
        let target = (1280, 720);
        let fast = fitting_mode_key((1280, 720), 144_000, target);
        let slow = fitting_mode_key((1280, 720), 60_000, target);
        assert!(fast < slow);
    }

    #[test]
    fn fitting_mode_weighs_width_before_height() {
        let target = (1280, 720);
        // Off by 100 on width, exact height, versus exact width and off by 100 on height.
        let wrong_width = fitting_mode_key((1380, 720), 60_000, target);
        let wrong_height = fitting_mode_key((1280, 820), 60_000, target);
        assert!(wrong_height < wrong_width);
    }

    #[test]
    fn best_mode_prefers_the_largest_resolution() {
        let big = best_mode_key((3840, 2160), 60_000);
        let small = best_mode_key((1280, 720), 240_000);
        assert!(big < small, "resolution outranks refresh rate");
    }

    #[test]
    fn best_mode_breaks_ties_on_refresh_rate() {
        let fast = best_mode_key((1920, 1080), 144_000);
        let slow = best_mode_key((1920, 1080), 60_000);
        assert!(fast < slow);
    }
}
