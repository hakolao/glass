//! The runtime context handed to your [`GlassApp`](crate::GlassApp), and the configuration used to build it.

use std::sync::Arc;

use indexmap::IndexMap;
use wgpu::{Adapter, Device, Instance, PowerPreference, Queue, SurfaceConfiguration};
use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{Fullscreen, Window, WindowId},
};

use crate::{
    device_context::{DeviceConfig, DeviceContext},
    window::{
        get_best_videomode, get_centered_window_position, get_fitting_videomode, GlassWindow,
        WindowConfig, WindowPos,
    },
    GlassError,
};

/// Configuration of your windows and devices.
#[derive(Default, Debug, Clone)]
pub struct GlassConfig {
    /// How the wgpu instance, adapter, device and queue are requested.
    pub device_config: DeviceConfig,
    /// Run an extra [`GlassApp::update`](crate::GlassApp::update) while a resize is in progress, so the window keeps
    /// painting instead of showing a stale frame while the user drags its edge.
    pub run_extra_update_on_resize: bool,
    /// Reconfigure each window surface automatically on resize and scale-factor changes. Turn
    /// this off if your app wants to own surface configuration.
    pub is_surface_auto_resize: bool,
}

impl GlassConfig {
    /// A configuration that asks for the high-performance adapter and enables both surface
    /// auto-resize and the extra update during resizes.
    ///
    /// Start from this and adjust, rather than filling in every field:
    ///
    /// ```
    /// use glass::prelude::*;
    /// use glass::wgpu::{Features, Limits};
    ///
    /// let config = GlassConfig {
    ///     device_config: DeviceConfig {
    ///         features: Features::TIMESTAMP_QUERY,
    ///         limits: Limits::downlevel_defaults(),
    ///         ..DeviceConfig::performance()
    ///     },
    ///     ..GlassConfig::performance()
    /// };
    /// assert!(config.is_surface_auto_resize);
    /// ```
    pub fn performance() -> Self {
        Self {
            device_config: DeviceConfig {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            },
            run_extra_update_on_resize: true,
            is_surface_auto_resize: true,
        }
    }
}

/// The runtime context accessible through [`GlassApp`](crate::GlassApp).
///
/// You can use the context to create windows at runtime, or to access devices, which are often
/// needed for render or compute functionality.
#[derive(Debug)]
pub struct GlassContext {
    create_windows: Vec<(String, WindowConfig)>,
    windows: IndexMap<WindowId, GlassWindow>,
    device_context: Arc<DeviceContext>,
    exit: bool,
    is_resize_extra_update: bool,
}

impl GlassContext {
    /// Creates the context, which requests the wgpu instance, adapter, device and queue up front.
    ///
    /// [`wgpu::Features::IMMEDIATES`] and
    /// [`wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`] are added to the requested
    /// features, because the pipelines in [`crate::pipelines`] need them.
    ///
    /// # Errors
    ///
    /// Fails if no adapter matches [`DeviceConfig::backends`], or if the adapter that is found
    /// does not meet the requested features and limits.
    pub fn new(mut config: GlassConfig) -> Result<Self, GlassError> {
        // Add push constants feature for common pipelines
        config.device_config.features |=
            wgpu::Features::IMMEDIATES | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        let device_context = Arc::new(DeviceContext::new(&config.device_config)?);

        Ok(Self {
            device_context,
            create_windows: vec![],
            windows: IndexMap::default(),
            exit: false,
            is_resize_extra_update: false,
        })
    }

    /// `true` while this update was triggered by a resize rather than by the normal frame loop.
    /// Only ever `true` when [`GlassConfig::run_extra_update_on_resize`] is set.
    pub fn is_resize_extra_update(&self) -> bool {
        self.is_resize_extra_update
    }

    /// The wgpu instance backing every window surface.
    pub fn instance(&self) -> &Instance {
        self.device_context.instance()
    }

    /// The adapter the device was requested from.
    pub fn adapter(&self) -> &Adapter {
        self.device_context.adapter()
    }

    /// The device shared by every window.
    pub fn device(&self) -> &Device {
        self.device_context.device()
    }

    /// A cloned handle to the shared device, for passing into long-lived structs.
    pub fn device_arc(&self) -> Arc<Device> {
        self.device_context.device_arc()
    }

    /// The queue shared by every window.
    pub fn queue(&self) -> &Queue {
        self.device_context.queue()
    }

    /// A cloned handle to the shared queue, for passing into long-lived structs.
    pub fn queue_arc(&self) -> Arc<Queue> {
        self.device_context.queue_arc()
    }

    /// Applies `config` to the surface of the window registered under `window_id`.
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::WindowNotFoundError`] if the window is gone, or
    /// [`GlassError::UnsupportedSurfaceFormat`] if the surface cannot use `config.format`.
    pub fn configure_surface(
        &mut self,
        window_id: &WindowId,
        config: &SurfaceConfiguration,
    ) -> Result<(), GlassError> {
        if let Some(window) = self.windows.get_mut(window_id) {
            window.configure_surface(self.device_context.device(), config)?;
            Ok(())
        } else {
            Err(GlassError::WindowNotFoundError {
                window_id: *window_id,
            })
        }
    }

    /// Reconfigures the window surface at `size`, keeping its other surface settings.
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::WindowNotFoundError`] if the window is gone.
    pub fn reconfigure_surface_with_size(
        &mut self,
        window_id: &WindowId,
        size: PhysicalSize<u32>,
    ) -> Result<(), GlassError> {
        if let Some(window) = self.windows.get_mut(window_id) {
            window.configure_surface_with_size(self.device_context.device(), size)?;
            Ok(())
        } else {
            Err(GlassError::WindowNotFoundError {
                window_id: *window_id,
            })
        }
    }

    /// Reconfigures the window surface at its last known size. Useful after the surface has gone
    /// stale.
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::WindowNotFoundError`] if the window is gone.
    pub fn reconfigure_surface(&mut self, window_id: &WindowId) -> Result<(), GlassError> {
        if let Some(window) = self.windows.get_mut(window_id) {
            window.reconfigure_surface(self.device_context.device())?;
            Ok(())
        } else {
            Err(GlassError::WindowNotFoundError {
                window_id: *window_id,
            })
        }
    }

    /// Creates a brand new surface for the window and configures it with `config`. Use this after
    /// a device-lost event, where reconfiguring the existing surface is not enough.
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::WindowNotFoundError`] if the window is gone, or
    /// [`GlassError::SurfaceError`] if the new surface cannot be created.
    pub fn recreate_surface(
        &mut self,
        window_id: &WindowId,
        config: &SurfaceConfiguration,
    ) -> Result<(), GlassError> {
        if let Some(window) = self.windows.get_mut(window_id) {
            window.recreate_surface(self.device_context.device(), config)?;
            Ok(())
        } else {
            Err(GlassError::WindowNotFoundError {
                window_id: *window_id,
            })
        }
    }

    /// The first window in creation order, or `None` if no window exists yet.
    pub fn primary_render_window_maybe(&self) -> Option<&GlassWindow> {
        self.windows.first().map(|(_k, v)| v)
    }

    /// The first window in creation order.
    ///
    /// # Panics
    ///
    /// Panics if no window exists. This is the convenience form for single-window apps that
    /// create their window before first use; call [`GlassContext::primary_render_window_maybe`]
    /// if a window might not exist yet.
    pub fn primary_render_window(&self) -> &GlassWindow {
        self.windows
            .first()
            .expect("no window exists yet; use primary_render_window_maybe")
            .1
    }

    /// Mutable variant of [`GlassContext::primary_render_window`].
    ///
    /// # Panics
    ///
    /// Panics if no window exists.
    pub fn primary_render_window_mut(&mut self) -> &mut GlassWindow {
        self.windows
            .first_mut()
            .expect("no window exists yet; use primary_render_window_maybe")
            .1
    }

    /// Returns the window created under `name`, if it exists yet. Names are expected to be
    /// unique; this returns the first matching window in creation order.
    pub fn window(&self, name: &str) -> Option<&GlassWindow> {
        self.windows.values().find(|w| w.name() == name)
    }

    /// Mutable variant of [`GlassContext::window`].
    pub fn window_mut(&mut self, name: &str) -> Option<&mut GlassWindow> {
        self.windows.values_mut().find(|w| w.name() == name)
    }

    /// Every window, keyed by [`WindowId`], in creation order.
    pub fn windows(&self) -> &IndexMap<WindowId, GlassWindow> {
        &self.windows
    }

    /// Mutable variant of [`GlassContext::windows`].
    pub fn windows_mut(&mut self) -> &mut IndexMap<WindowId, GlassWindow> {
        &mut self.windows
    }

    /// Looks a window up by its winit id, which is what window events carry.
    pub fn render_window(&self, id: WindowId) -> Option<&GlassWindow> {
        self.windows.get(&id)
    }

    /// Mutable variant of [`GlassContext::render_window`].
    pub fn render_window_mut(&mut self, id: WindowId) -> Option<&mut GlassWindow> {
        self.windows.get_mut(&id)
    }

    /// Queues a window for creation. The window is created on the next event loop iteration and
    /// can then be retrieved with [`GlassContext::window`] using `name`. Use this when no
    /// [`ActiveEventLoop`] is available (e.g. from [`GlassApp::update`](crate::GlassApp::update)).
    ///
    /// Because creation is deferred, failures cannot be returned here. They are reported out of
    /// [`crate::Glass::run`] instead, which ends the event loop.
    pub fn create_window(&mut self, name: impl Into<String>, config: WindowConfig) {
        self.create_windows.push((name.into(), config));
    }

    /// Creates a window right away and returns its [`WindowId`]. Requires an [`ActiveEventLoop`],
    /// so call this from [`GlassApp::start`](crate::GlassApp::start) or an input handler. The window is retrievable in the
    /// same call via [`GlassContext::window`] / [`GlassContext::render_window`] using the returned
    /// name or id.
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::WindowError`] if the OS refuses the window,
    /// [`GlassError::SurfaceError`] if no surface can be created for it, or
    /// [`GlassError::UnsupportedSurfaceFormat`] if `config.surface_config.format` is not
    /// supported by that surface.
    pub fn create_window_immediately(
        &mut self,
        event_loop: &ActiveEventLoop,
        name: impl Into<String>,
        config: WindowConfig,
    ) -> Result<WindowId, GlassError> {
        self.spawn_window(event_loop, name.into(), config)
    }

    fn spawn_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        name: String,
        config: WindowConfig,
    ) -> Result<WindowId, GlassError> {
        let window = Self::create_winit_window(event_loop, &config)?;
        let id = self.add_window(name, config, window)?;
        // Inserted by `add_window` just above, so this lookup cannot fail.
        let window = &mut self.windows[&id];
        window.configure_surface_with_size(
            self.device_context.device(),
            window.window().inner_size(),
        )?;
        Ok(id)
    }

    /// Drains the queue filled by [`GlassContext::create_window`]. Stops at the first failure so
    /// that it can be surfaced out of the event loop instead of panicking.
    pub(crate) fn create_queued_windows(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), GlassError> {
        for (name, config) in std::mem::take(&mut self.create_windows) {
            self.spawn_window(event_loop, name, config)?;
        }
        Ok(())
    }

    fn add_window(
        &mut self,
        name: String,
        config: WindowConfig,
        window: Arc<Window>,
    ) -> Result<WindowId, GlassError> {
        let id = window.id();
        let render_window = GlassWindow::new(&self.device_context, name, config, window)?;
        self.windows.insert(id, render_window);
        Ok(id)
    }

    fn create_winit_window(
        event_loop: &ActiveEventLoop,
        config: &WindowConfig,
    ) -> Result<Arc<Window>, GlassError> {
        let mut window_attributes = config
            .other_attributes
            .clone()
            .unwrap_or_else(Window::default_attributes)
            .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height))
            .with_title(config.title.clone());

        // Min size
        if let Some(inner_size) = config.min_size {
            window_attributes = window_attributes.with_min_inner_size(inner_size);
        }

        // Max size
        if let Some(inner_size) = config.max_size {
            window_attributes = window_attributes.with_max_inner_size(inner_size);
        }

        // Hide first frame
        if config.hide_until_first_frame {
            window_attributes = window_attributes.with_visible(false);
        }

        window_attributes = match &config.pos {
            WindowPos::Maximized => window_attributes.with_maximized(true),
            WindowPos::FullScreen => {
                // A monitor reporting no video modes cannot do exclusive fullscreen, so fall back
                // to borderless rather than failing window creation.
                let mode = event_loop
                    .primary_monitor()
                    .and_then(|monitor| get_best_videomode(&monitor));
                Self::with_exclusive_or_borderless(window_attributes, event_loop, mode)
            }
            WindowPos::SizedFullScreen => {
                let mode = event_loop.primary_monitor().and_then(|monitor| {
                    get_fitting_videomode(&monitor, config.width, config.height)
                });
                Self::with_exclusive_or_borderless(window_attributes, event_loop, mode)
            }
            WindowPos::FullScreenBorderless => window_attributes
                .with_fullscreen(Some(Fullscreen::Borderless(event_loop.primary_monitor()))),
            WindowPos::Pos(pos) => window_attributes.with_position(*pos),
            WindowPos::Centered => {
                if let Some(monitor) = event_loop.primary_monitor() {
                    window_attributes.with_position(get_centered_window_position(
                        &monitor,
                        config.width,
                        config.height,
                    ))
                } else {
                    window_attributes
                }
            }
        };

        Ok(Arc::new(event_loop.create_window(window_attributes)?))
    }

    fn with_exclusive_or_borderless(
        attributes: winit::window::WindowAttributes,
        event_loop: &ActiveEventLoop,
        mode: Option<winit::monitor::VideoModeHandle>,
    ) -> winit::window::WindowAttributes {
        match mode {
            Some(mode) => attributes.with_fullscreen(Some(Fullscreen::Exclusive(mode))),
            None => attributes
                .with_fullscreen(Some(Fullscreen::Borderless(event_loop.primary_monitor()))),
        }
    }

    /// Asks the event loop to exit after the current update. Every window is dropped.
    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.exit
    }

    pub(crate) fn clear_windows(&mut self) {
        self.windows.clear();
    }

    pub(crate) fn remove_window(&mut self, id: &WindowId) {
        self.windows.swap_remove(id);
    }

    pub(crate) fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    pub(crate) fn set_resize_extra_update(&mut self, value: bool) {
        self.is_resize_extra_update = value;
    }
}
