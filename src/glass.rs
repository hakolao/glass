use indexmap::IndexMap;
use wgpu::{
    Adapter, ComputePipeline, Device, Instance, PowerPreference, Queue, RenderPipeline,
    SurfaceConfiguration, TextureFormat,
};
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::{Window, WindowId},
};

use crate::{
    device_context::{DeviceConfig, DeviceContext},
    pipelines::{CommonPipelines, PipelineKey, Pipelines},
    window::{GlassWindow, WindowConfig},
    GlassApp, RenderData,
};

/// [`Glass`] is an application that exposes an easy to use API to organize your winit applications
/// which render using wgpu. Just impl [`GlassApp`] for your application (of any type) and you
/// are good to go.
pub struct Glass<A> {
    app: A,
    config: GlassConfig,
}

impl<A: GlassApp + 'static> Glass<A> {
    pub fn new(app: A, config: GlassConfig) -> Glass<A> {
        Glass {
            app,
            config,
        }
    }

    pub fn run(mut self) {
        let event_loop = EventLoop::new();
        let mut context = GlassContext::new(&event_loop, self.config.clone());
        self.app.start(&event_loop, &mut context);
        let mut remove_windows = vec![];
        let mut request_window_close = false;

        event_loop.run(move |event, event_loop, control_flow| {
            control_flow.set_poll();

            // Run input fn
            self.app.input(&mut context, event_loop, &event);
            match event {
                Event::WindowEvent {
                    window_id,
                    event: window_event,
                    ..
                } => {
                    if let Some(window) = context.windows.get_mut(&window_id) {
                        match window_event {
                            WindowEvent::Resized(physical_size) => {
                                // On windows, minimized app can have 0,0 size
                                if physical_size.width > 0 && physical_size.height > 0 {
                                    window.configure_surface_with_size(
                                        &context.device_context.device(),
                                        physical_size,
                                    );
                                }
                            }
                            WindowEvent::ScaleFactorChanged {
                                new_inner_size, ..
                            } => {
                                window.configure_surface_with_size(
                                    &context.device_context.device(),
                                    *new_inner_size,
                                );
                            }
                            WindowEvent::KeyboardInput {
                                input,
                                is_synthetic,
                                ..
                            } => {
                                if let Some(key) = input.virtual_keycode {
                                    if !is_synthetic
                                        && window.exit_on_esc()
                                        && window.is_focused()
                                        && key == VirtualKeyCode::Escape
                                        && input.state == ElementState::Pressed
                                    {
                                        request_window_close = true;
                                        remove_windows.push(window_id);
                                    }
                                }
                            }
                            WindowEvent::Focused(has_focus) => {
                                window.set_focus(has_focus);
                            }
                            WindowEvent::CloseRequested => {
                                request_window_close = true;
                                remove_windows.push(window_id);
                            }
                            _ => (),
                        }
                    }
                }
                Event::MainEventsCleared => {
                    self.app.update(&mut context);
                    // Close window(s)
                    if request_window_close {
                        for window in remove_windows.iter() {
                            context.windows.remove(window);
                        }
                        remove_windows.clear();
                        request_window_close = false;
                        // Exit
                        if context.windows.is_empty() {
                            control_flow.set_exit();
                            // Run end
                            self.app.end(&mut context);
                        }
                    }
                    // Render
                    for (_, window) in context.windows.iter() {
                        match window.surface().get_current_texture() {
                            Ok(frame) => {
                                let mut encoder = context
                                    .device_context
                                    .device()
                                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                        label: Some("Render Commands"),
                                    });

                                // Run render & post processing functions
                                self.app.render(&context, RenderData {
                                    encoder: &mut encoder,
                                    window,
                                    frame: &frame,
                                });
                                self.app.post_processing(&context, RenderData {
                                    encoder: &mut encoder,
                                    window,
                                    frame: &frame,
                                });

                                context
                                    .device_context
                                    .queue()
                                    .submit(Some(encoder.finish()));

                                frame.present();

                                self.app.after_render(&context);
                            }
                            Err(error) => {
                                if error == wgpu::SurfaceError::OutOfMemory {
                                    panic!("Swapchain error: {error}. Rendering cannot continue.")
                                }
                            }
                        }
                        window.window().request_redraw();
                    }
                    // End of frame
                    self.app.end_of_frame(&mut context);
                }
                _ => {}
            }
        });
    }
}

/// Configuration of your windows and devices.
#[derive(Debug, Clone)]
pub struct GlassConfig {
    pub with_common_pipelines: bool,
    pub device_config: DeviceConfig,
    pub window_configs: Vec<WindowConfig>,
}

impl GlassConfig {
    pub fn windowless() -> Self {
        Self {
            with_common_pipelines: false,
            device_config: DeviceConfig::default(),
            window_configs: vec![],
        }
    }

    pub fn performance(width: u32, height: u32) -> Self {
        Self {
            with_common_pipelines: false,
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
            with_common_pipelines: false,
            device_config: DeviceConfig::default(),
            window_configs: vec![WindowConfig::default()],
        }
    }
}

/// The runtime context accessible through [`GlassApp`].
/// You can use the context to create windows at runtime. Or access devices, which are often
/// needed for render or compute functionality.
pub struct GlassContext {
    common_pipelines: Option<CommonPipelines>,
    custom_pipelines: Pipelines,
    device_context: DeviceContext,
    windows: IndexMap<WindowId, GlassWindow>,
}

impl GlassContext {
    pub fn new(event_loop: &EventLoop<()>, mut config: GlassConfig) -> Self {
        // Create windows from initial configs
        let winit_windows = config
            .window_configs
            .iter()
            .map(|&window_config| {
                (
                    window_config,
                    Self::create_winit_window(event_loop, &window_config),
                )
            })
            .collect::<Vec<(WindowConfig, Window)>>();
        // Modify features & limits needed for common pipelines
        if config.with_common_pipelines {
            // Add push constants feature for common pipelines
            config.device_config.features |= wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
            // Add push constant limits
            config.device_config.limits = wgpu::Limits {
                max_push_constant_size: 256,
                ..config.device_config.limits
            };
        };
        let device_context = DeviceContext::new(
            &config.device_config,
            // Needed to ensure our queue families are compatible with surface
            &winit_windows,
        );
        let mut app = Self {
            common_pipelines: if config.with_common_pipelines {
                Some(CommonPipelines::new(
                    device_context.device(),
                    TextureFormat::Bgra8UnormSrgb,
                ))
            } else {
                None
            },
            custom_pipelines: Pipelines::default(),
            device_context,
            windows: IndexMap::default(),
        };
        for (window_config, window) in winit_windows {
            let id = app.add_window(window_config, window);
            // Configure window surface with size
            let window = app.windows.get_mut(&id).unwrap();
            window.configure_surface_with_size(
                &app.device_context.device(),
                window.window().inner_size(),
            );
        }
        app
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

    pub fn queue(&self) -> &Queue {
        self.device_context.queue()
    }

    pub fn configure_surface(&mut self, window_id: &WindowId, config: &SurfaceConfiguration) {
        if let Some(window) = self.windows.get_mut(window_id) {
            window.configure_surface(self.device_context.device(), config);
        } else {
            panic!("No window with id {:?}", window_id);
        }
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
        event_loop: &EventLoopWindowTarget<()>,
        config: WindowConfig,
    ) -> WindowId {
        let reconfigure_device = self.windows.is_empty();
        let window = Self::create_winit_window(event_loop, &config);
        let id = self.add_window(config, window);
        // Reconfigure devices with surface so queue families are correct
        let window = self.windows.get_mut(&id).unwrap();
        if reconfigure_device {
            let surface = window.surface();
            self.device_context.reconfigure_with_surface(surface);
        }
        // Configure surface with size
        window.configure_surface_with_size(
            &self.device_context.device(),
            window.window().inner_size(),
        );
        id
    }

    fn add_window(&mut self, config: WindowConfig, window: Window) -> WindowId {
        let id = window.id();
        let render_window = GlassWindow::new(&self.device_context, config, window);
        self.windows.insert(id, render_window);
        id
    }

    fn create_winit_window(
        event_loop: &EventLoopWindowTarget<()>,
        config: &WindowConfig,
    ) -> Window {
        // ToDo: Add more options and settings to window creation
        let window_builder = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height))
            .with_title(config.title);

        window_builder.build(event_loop).unwrap()
    }

    pub fn draw_pipeline(&self, key: &PipelineKey) -> Option<&RenderPipeline> {
        self.custom_pipelines.draw_pipeline(key)
    }

    pub fn compute_pipeline(&self, key: &PipelineKey) -> Option<&ComputePipeline> {
        self.custom_pipelines.compute_pipeline(key)
    }

    pub fn add_draw_pipeline(&mut self, pipeline_key: PipelineKey, pipeline: RenderPipeline) {
        self.custom_pipelines
            .add_draw_pipeline(pipeline_key, pipeline)
    }

    pub fn add_compute_pipeline(&mut self, pipeline_key: PipelineKey, pipeline: ComputePipeline) {
        self.custom_pipelines
            .add_compute_pipeline(pipeline_key, pipeline)
    }

    pub fn common_pipeline(&self) -> &CommonPipelines {
        self.common_pipelines
            .as_ref()
            .expect("No common pipelines, create with config `with_common_pipelines: true`")
    }
}
