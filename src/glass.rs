use std::{fmt::Formatter, sync::Arc};

use image::ImageError;
use indexmap::IndexMap;
use wgpu::{
    Adapter, CreateSurfaceError, Device, Instance, PowerPreference, Queue, RequestDeviceError,
    Sampler, SurfaceConfiguration,
};
use winit::{
    application::ApplicationHandler,
    error::{EventLoopError, OsError},
    event::{DeviceEvent, DeviceId, ElementState, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Fullscreen, Window, WindowId},
};

use crate::{
    device_context::{DeviceConfig, DeviceContext},
    window::{
        get_best_videomode, get_centered_window_position, get_fitting_videomode, GlassWindow,
        WindowConfig, WindowPos,
    },
    GlassApp, RenderData,
};

/// [`Glass`] is an application that exposes an easy to use API to organize your winit applications
/// which render using wgpu. Just impl [`GlassApp`] for your application (of any type) and you
/// are good to go.
pub struct Glass {
    config: GlassConfig,
    app: Box<dyn GlassApp>,
    context: GlassContext,
    runner_state: RunnerState,
}

impl Glass {
    pub fn run(
        config: GlassConfig,
        app_create_fn: impl FnOnce(&mut GlassContext) -> Box<dyn GlassApp>,
    ) -> Result<(), GlassError> {
        let mut context = GlassContext::new(config.clone())?;
        let app = app_create_fn(&mut context);
        let mut glass = Glass {
            app,
            context,
            config,
            runner_state: RunnerState::default(),
        };
        let event_loop = match EventLoop::new() {
            Ok(e) => e,
            Err(e) => return Err(GlassError::EventLoopError(e)),
        };
        event_loop
            .run_app(&mut glass)
            .map_err(GlassError::EventLoopError)
    }
}

impl ApplicationHandler for Glass {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        // Ensure we're poll
        if event_loop.control_flow() != ControlFlow::Poll {
            event_loop.set_control_flow(ControlFlow::Poll);
        }

        let Glass {
            app,
            context,
            ..
        } = self;
        app.before_input(context, event_loop);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Glass {
            app,
            config,
            context,
            runner_state,
            ..
        } = self;
        // Initial windows
        if !runner_state.is_init {
            // Create windows from initial configs
            let mut winit_windows = vec![];
            for &window_config in config.window_configs.iter() {
                winit_windows.push((
                    window_config,
                    GlassContext::create_winit_window(event_loop, &window_config).unwrap(),
                ))
            }
            for (window_config, window) in winit_windows {
                let id = context.add_window(window_config, window).unwrap();
                // Configure window surface with size
                let window = context.windows.get_mut(&id).unwrap();
                window.configure_surface_with_size(
                    context.device_context.device(),
                    window.window().inner_size(),
                );
            }
            app.start(event_loop, context);
            runner_state.is_init = true;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Glass {
            app,
            context,
            runner_state,
            ..
        } = self;
        app.window_input(context, event_loop, window_id, &event);

        let mut is_extra_update = false;

        if let Some(window) = context.windows.get_mut(&window_id) {
            match event {
                WindowEvent::Resized(physical_size) => {
                    // On windows, minimized app can have 0,0 size
                    if physical_size.width > 0 && physical_size.height > 0 {
                        window.configure_surface_with_size(
                            context.device_context.device(),
                            physical_size,
                        );
                        is_extra_update = true;
                    }
                }
                WindowEvent::ScaleFactorChanged {
                    ..
                } => {
                    let size = window.window().inner_size();
                    window.configure_surface_with_size(context.device_context.device(), size);
                    is_extra_update = true;
                }
                WindowEvent::KeyboardInput {
                    event,
                    is_synthetic,
                    ..
                } => {
                    if event.logical_key == Key::Named(NamedKey::Escape)
                        && !is_synthetic
                        && window.exit_on_esc()
                        && window.is_focused()
                        && event.state == ElementState::Pressed
                    {
                        runner_state.request_window_close = true;
                        runner_state.remove_windows.push(window_id);
                    }
                }
                WindowEvent::Focused(has_focus) => {
                    window.set_focus(has_focus);
                }
                WindowEvent::CloseRequested => {
                    runner_state.request_window_close = true;
                    runner_state.remove_windows.push(window_id);
                }
                _ => (),
            }
        }
        // Update immediately, because about_to_wait isn't triggered during resize. If it did,
        // this would not be needed.

        // Winit recommends running rendering inside `RequestRedraw`, but that doesn't really
        // seem good to me, because I want render to take place immediately after update, and
        // running entire app's update within one window's `RequestRedraw` doesn't make sense
        // to me.

        // This ensures resizing's effect is instant. Kinda ugly on performance, but that doesn't
        // matter, because resize is a rare event.
        if is_extra_update {
            run_update(event_loop, app, context, runner_state);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Glass {
            app,
            context,
            ..
        } = self;
        app.device_input(context, event_loop, device_id, &event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Glass {
            app,
            context,
            runner_state,
            ..
        } = self;
        run_update(event_loop, app, context, runner_state);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let Glass {
            app,
            context,
            ..
        } = self;
        app.end(context);
    }
}

fn run_update(
    event_loop: &ActiveEventLoop,
    app: &mut Box<dyn GlassApp>,
    context: &mut GlassContext,
    runner_state: &mut RunnerState,
) {
    if context.exit {
        context.windows.clear();
        event_loop.exit();
        return;
    }
    if runner_state.request_window_close {
        for window in runner_state.remove_windows.iter() {
            context.windows.swap_remove(window);
        }
        runner_state.remove_windows.clear();
        runner_state.request_window_close = false;
        // Exit
        if context.windows.is_empty() {
            context.exit();
            return;
        }
    }
    app.update(context);

    render(app, context);

    app.end_of_frame(context);
}

fn render(app: &mut Box<dyn GlassApp>, context: &mut GlassContext) {
    for (_, window) in context.windows.iter() {
        match window.surface().get_current_texture() {
            Ok(frame) => {
                let mut encoder = context.device_context.device().create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("Render Commands"),
                    },
                );

                // Run render
                let mut buffers = app
                    .render(context, RenderData {
                        encoder: &mut encoder,
                        window,
                        frame: &frame,
                    })
                    .unwrap_or_default();
                buffers.push(encoder.finish());
                context.device_context.queue().submit(buffers);

                frame.present();
            }
            Err(error) => {
                if error == wgpu::SurfaceError::OutOfMemory {
                    panic!("Swapchain error: {error}. Rendering cannot continue.")
                }
            }
        }
        window.window().request_redraw();
    }
}

#[derive(Default)]
struct RunnerState {
    is_init: bool,
    request_window_close: bool,
    remove_windows: Vec<WindowId>,
}

/// Configuration of your windows and devices.
#[derive(Debug, Clone)]
pub struct GlassConfig {
    pub device_config: DeviceConfig,
    pub window_configs: Vec<WindowConfig>,
}

impl GlassConfig {
    pub fn windowless() -> Self {
        Self {
            device_config: DeviceConfig::default(),
            window_configs: vec![],
        }
    }

    pub fn performance(width: u32, height: u32) -> Self {
        Self {
            device_config: DeviceConfig {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            },
            window_configs: vec![WindowConfig {
                width,
                height,
                exit_on_esc: false,
                ..WindowConfig::default()
            }],
        }
    }
}

impl Default for GlassConfig {
    fn default() -> Self {
        Self {
            device_config: DeviceConfig::default(),
            window_configs: vec![WindowConfig::default()],
        }
    }
}

#[derive(Debug)]
pub enum GlassError {
    WindowError(OsError),
    SurfaceError(CreateSurfaceError),
    AdapterError,
    DeviceError(RequestDeviceError),
    ImageError(ImageError),
    EventLoopError(EventLoopError),
}

impl std::fmt::Display for GlassError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GlassError::WindowError(e) => format!("WindowError: {}", e),
            GlassError::SurfaceError(e) => format!("SurfaceError: {}", e),
            GlassError::AdapterError => "AdapterError".to_owned(),
            GlassError::DeviceError(e) => format!("DeviceError: {}", e),
            GlassError::ImageError(e) => format!("ImageError: {}", e),
            GlassError::EventLoopError(e) => format!("EventLoopError: {}", e),
        };
        write!(f, "{}", s)
    }
}

/// The runtime context accessible through [`GlassApp`].
/// You can use the context to create windows at runtime. Or access devices, which are often
/// needed for render or compute functionality.
pub struct GlassContext {
    device_context: DeviceContext,
    windows: IndexMap<WindowId, GlassWindow>,
    exit: bool,
}

impl GlassContext {
    pub fn new(mut config: GlassConfig) -> Result<Self, GlassError> {
        // Modify features & limits needed for common pipelines
        // Add push constants feature for common pipelines
        config.device_config.features |= wgpu::Features::PUSH_CONSTANTS
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        config.device_config.limits = wgpu::Limits {
            ..config.device_config.limits
        };
        let device_context = DeviceContext::new(&config.device_config)?;

        Ok(Self {
            device_context,
            windows: IndexMap::default(),
            exit: false,
        })
    }

    pub fn sampler_nearest_repeat(&self) -> &Arc<Sampler> {
        self.device_context.sampler_nearest_repeat()
    }

    pub fn sampler_linear_repeat(&self) -> &Arc<Sampler> {
        self.device_context.sampler_linear_repeat()
    }

    pub fn sampler_nearest_clamp_to_edge(&self) -> &Arc<Sampler> {
        self.device_context.sampler_nearest_clamp_to_edge()
    }

    pub fn sampler_linear_clamp_to_edge(&self) -> &Arc<Sampler> {
        self.device_context.sampler_linear_clamp_to_edge()
    }

    #[allow(unused)]
    pub fn instance(&self) -> &Instance {
        self.device_context.instance()
    }

    pub fn adapter(&self) -> &Adapter {
        self.device_context.adapter()
    }

    pub fn device(&self) -> &Device {
        self.device_context.device()
    }

    pub fn device_arc(&self) -> Arc<Device> {
        self.device_context.device_arc()
    }

    pub fn queue(&self) -> &Queue {
        self.device_context.queue()
    }

    pub fn queue_arc(&self) -> Arc<Queue> {
        self.device_context.queue_arc()
    }

    pub fn configure_surface(&mut self, window_id: &WindowId, config: &SurfaceConfiguration) {
        if let Some(window) = self.windows.get_mut(window_id) {
            window.configure_surface(self.device_context.device(), config);
        } else {
            panic!("No window with id {:?}", window_id);
        }
    }

    pub fn primary_render_window_maybe(&self) -> Option<&GlassWindow> {
        self.windows.first().map(|(_k, v)| v)
    }

    pub fn primary_render_window(&self) -> &GlassWindow {
        self.windows.first().unwrap().1
    }

    pub fn primary_render_window_mut(&mut self) -> &mut GlassWindow {
        self.windows.first_mut().unwrap().1
    }

    pub fn render_window(&self, id: WindowId) -> Option<&GlassWindow> {
        self.windows.get(&id)
    }

    pub fn render_window_mut(&mut self, id: WindowId) -> Option<&mut GlassWindow> {
        self.windows.get_mut(&id)
    }

    pub fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        config: WindowConfig,
    ) -> Result<WindowId, GlassError> {
        let reconfigure_device = self.windows.is_empty();
        let window = Self::create_winit_window(event_loop, &config)?;
        let id = self.add_window(config, window)?;
        // Reconfigure devices with surface so queue families are correct
        let window = self.windows.get_mut(&id).unwrap();
        if reconfigure_device {
            let surface = window.surface();
            self.device_context.reconfigure_with_surface(surface)?;
        }
        // Configure surface with size
        window.configure_surface_with_size(
            self.device_context.device(),
            window.window().inner_size(),
        );
        Ok(id)
    }

    fn add_window(
        &mut self,
        config: WindowConfig,
        window: Arc<Window>,
    ) -> Result<WindowId, GlassError> {
        let id = window.id();
        let render_window = match GlassWindow::new(&self.device_context, config, window) {
            Ok(window) => window,
            Err(e) => return Err(GlassError::SurfaceError(e)),
        };
        self.windows.insert(id, render_window);
        Ok(id)
    }

    fn create_winit_window(
        event_loop: &ActiveEventLoop,
        config: &WindowConfig,
    ) -> Result<Arc<Window>, GlassError> {
        let mut window_attributes = Window::default_attributes()
            .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height))
            .with_title(config.title);

        // Min size
        if let Some(inner_size) = config.min_size {
            window_attributes = window_attributes.with_min_inner_size(inner_size);
        }

        // Max size
        if let Some(inner_size) = config.max_size {
            window_attributes = window_attributes.with_max_inner_size(inner_size);
        }

        window_attributes = match &config.pos {
            WindowPos::Maximized => window_attributes.with_maximized(true),
            WindowPos::FullScreen => {
                if let Some(monitor) = event_loop.primary_monitor() {
                    window_attributes
                        .with_fullscreen(Some(Fullscreen::Exclusive(get_best_videomode(&monitor))))
                } else {
                    window_attributes
                }
            }
            WindowPos::SizedFullScreen => {
                if let Some(monitor) = event_loop.primary_monitor() {
                    window_attributes.with_fullscreen(Some(Fullscreen::Exclusive(
                        get_fitting_videomode(&monitor, config.width, config.height),
                    )))
                } else {
                    window_attributes
                }
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

        match event_loop.create_window(window_attributes) {
            Ok(w) => Ok(Arc::new(w)),
            Err(e) => Err(GlassError::WindowError(e)),
        }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}
