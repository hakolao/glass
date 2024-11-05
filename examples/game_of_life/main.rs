use std::{borrow::Cow, time::Instant};

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use glass::{
    device_context::DeviceConfig,
    pipelines::QuadPipeline,
    texture::Texture,
    window::{GlassWindow, WindowConfig},
    Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::{
    Backends, BindGroup, BindGroupDescriptor, CommandBuffer, CommandEncoder, ComputePassDescriptor,
    ComputePipeline, ComputePipelineDescriptor, Extent3d, InstanceFlags, Limits, MemoryHints,
    PowerPreference, PresentMode, PushConstantRange, ShaderStages, StorageTextureAccess, StoreOp,
    TextureFormat, TextureUsages,
};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const FPS_60: f32 = 16.0 / 1000.0;
#[rustfmt::skip]
const OPENGL_TO_WGPU: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

fn config() -> GlassConfig {
    GlassConfig {
        device_config: DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            memory_hints: MemoryHints::Performance,
            features: wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            limits: Limits {
                max_push_constant_size: 128,
                ..Limits::default()
            },
            backends: Backends::all(),
            instance_flags: InstanceFlags::from_build_config(),
            trace_path: None,
        },
        window_configs: vec![WindowConfig {
            width: WIDTH,
            height: HEIGHT,
            exit_on_esc: true,
            present_mode: PresentMode::AutoNoVsync,
            ..WindowConfig::default()
        }],
    }
}

fn main() -> Result<(), GlassError> {
    Glass::run(config(), |_| Box::new(GameOfLifeApp::default()))
}

// Think of this like reading a "table of contents".
// - Start is run before event loop
// - Input is run on winit input
// - Update is run every frame
// - Render is run for each window after update every frame
impl GlassApp for GameOfLifeApp {
    fn start(&mut self, _event_loop: &ActiveEventLoop, context: &mut GlassContext) {
        // Create pipelines
        let (init_pipeline, game_of_life_pipeline, draw_pipeline) =
            create_game_of_life_pipeline(context);
        let quad_pipeline = QuadPipeline::new(context.device(), wgpu::ColorTargetState {
            format: GlassWindow::default_surface_format(),
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent::OVER,
                alpha: wgpu::BlendComponent::OVER,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        });
        self.data = Some(create_canvas_data(
            context,
            &quad_pipeline,
            &init_pipeline,
            &draw_pipeline,
        ));
        self.init_pipeline = Some(init_pipeline);
        self.game_of_life_pipeline = Some(game_of_life_pipeline);
        self.draw_pipeline = Some(draw_pipeline);
        self.quad_pipeline = Some(quad_pipeline);
        init_game_of_life(self, context);
    }

    fn window_input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: &WindowEvent,
    ) {
        handle_inputs(self, event);
    }

    fn update(&mut self, context: &mut GlassContext) {
        run_update(self, context);
    }

    fn render(
        &mut self,
        _context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
        render(self, render_data)
    }
}

struct GameOfLifeApp {
    quad_pipeline: Option<QuadPipeline>,
    init_pipeline: Option<ComputePipeline>,
    game_of_life_pipeline: Option<ComputePipeline>,
    draw_pipeline: Option<ComputePipeline>,
    data: Option<CanvasData>,
    cursor_pos: Vec2,
    prev_cursor_pos: Option<Vec2>,
    draw: bool,
    dt_sum: f32,
    num_dts: f32,
    time: Instant,
    updated_time: Instant,
    count: usize,
    commands: Option<CommandBuffer>,
}

impl Default for GameOfLifeApp {
    fn default() -> Self {
        Self {
            quad_pipeline: None,
            init_pipeline: None,
            game_of_life_pipeline: None,
            draw_pipeline: None,
            data: None,
            cursor_pos: Default::default(),
            prev_cursor_pos: None,
            draw: false,
            dt_sum: 0.0,
            num_dts: 0.0,
            time: Instant::now(),
            updated_time: Instant::now(),
            count: 0,
            commands: None,
        }
    }
}

impl GameOfLifeApp {
    fn cursor_to_canvas(&self, width: f32, height: f32, scale_factor: f32) -> (Vec2, Vec2) {
        let half_screen = Vec2::new(width, height) / scale_factor / 2.0;
        let current_canvas_pos = self.cursor_pos / scale_factor - half_screen + WIDTH as f32 / 2.0;
        let prev_canvas_pos = self.prev_cursor_pos.unwrap_or(current_canvas_pos) / scale_factor
            - half_screen
            + HEIGHT as f32 / 2.0;
        (current_canvas_pos, prev_canvas_pos)
    }
}

fn run_update(app: &mut GameOfLifeApp, context: &GlassContext) {
    let now = Instant::now();
    app.dt_sum += (now - app.time).as_secs_f32();
    app.num_dts += 1.0;
    if app.num_dts == 100.0 {
        // Set fps
        context.primary_render_window().window().set_title(&format!(
            "Game Of Life: {:.2}",
            1.0 / (app.dt_sum / app.num_dts)
        ));
        app.num_dts = 0.0;
        app.dt_sum = 0.0;
    }
    app.time = Instant::now();

    // Use only single command queue
    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Computes"),
        });
    // Update 60fps
    if (app.time - app.updated_time).as_secs_f32() > FPS_60 {
        update_game_of_life(app, context, &mut encoder);
        app.updated_time = app.time;
    }
    if app.draw {
        draw_game_of_life(app, context, &mut encoder);
    }
    // Update prev cursor pos
    app.prev_cursor_pos = Some(app.cursor_pos);
    app.commands = Some(encoder.finish());
}

fn render(app: &mut GameOfLifeApp, render_data: RenderData) -> Option<Vec<CommandBuffer>> {
    let GameOfLifeApp {
        data,
        quad_pipeline,
        ..
    } = app;
    let canvas_data = data.as_ref().unwrap();
    let quad_pipeline = quad_pipeline.as_ref().unwrap();
    let RenderData {
        encoder,
        frame,
        window,
        ..
    } = render_data;
    let (width, height) = {
        let scale_factor = window.window().scale_factor() as f32;
        let size = window.window().inner_size();
        (
            size.width as f32 / scale_factor,
            size.height as f32 / scale_factor,
        )
    };
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        quad_pipeline.draw(
            &mut rpass,
            &canvas_data.canvas_bind_group,
            [0.0; 4],
            camera_projection([width, height]).to_cols_array_2d(),
            canvas_data.canvas.size,
            1.0,
        );
        Some(vec![app.commands.take().unwrap()])
    }
}

fn handle_inputs(app: &mut GameOfLifeApp, event: &WindowEvent) {
    match event {
        WindowEvent::CursorMoved {
            position, ..
        } => {
            app.cursor_pos = Vec2::new(position.x as f32, position.y as f32);
        }
        WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            ..
        } => {
            app.draw = state == &ElementState::Pressed;
        }
        _ => (),
    }
}

fn draw_game_of_life(
    app: &mut GameOfLifeApp,
    context: &GlassContext,
    encoder: &mut CommandEncoder,
) {
    let scale_factor = context.primary_render_window().window().scale_factor() as f32;
    let (width, height) = {
        let size = context.primary_render_window().window().inner_size();
        (size.width as f32, size.height as f32)
    };
    let (end, start) = app.cursor_to_canvas(width, height, scale_factor);
    let GameOfLifeApp {
        data,
        draw_pipeline,
        ..
    } = app;
    let data = data.as_ref().unwrap();
    let draw_pipeline = draw_pipeline.as_ref().unwrap();

    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("Update"),
        timestamp_writes: None,
    });
    let pc = GameOfLifePushConstants::new(start, end, 10.0);
    cpass.set_pipeline(draw_pipeline);
    cpass.set_bind_group(0, &data.draw_bind_group, &[]);
    cpass.set_push_constants(0, bytemuck::cast_slice(&[pc]));
    cpass.dispatch_workgroups(WIDTH / 8, HEIGHT / 8, 1);
}

fn update_game_of_life(
    app: &mut GameOfLifeApp,
    context: &GlassContext,
    encoder: &mut CommandEncoder,
) {
    let GameOfLifeApp {
        data,
        game_of_life_pipeline,
        ..
    } = app;
    let data = data.as_ref().unwrap();
    let game_of_life_pipeline = game_of_life_pipeline.as_ref().unwrap();
    let (canvas, data_in) = if app.count % 2 == 0 {
        (&data.canvas.views[0], &data.data_in.views[0])
    } else {
        (&data.data_in.views[0], &data.canvas.views[0])
    };
    let update_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Update Bind Group"),
        layout: &game_of_life_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(canvas),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(data_in),
            },
        ],
    });
    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("Update"),
        timestamp_writes: None,
    });
    let pc = GameOfLifePushConstants::new(Vec2::ZERO, Vec2::ZERO, 0.0);
    cpass.set_pipeline(game_of_life_pipeline);
    cpass.set_bind_group(0, &update_bind_group, &[]);
    cpass.set_push_constants(0, bytemuck::cast_slice(&[pc]));
    cpass.dispatch_workgroups(WIDTH / 8, HEIGHT / 8, 1);

    app.count += 1;
}

fn init_game_of_life(app: &mut GameOfLifeApp, context: &GlassContext) {
    let GameOfLifeApp {
        data,
        init_pipeline,
        ..
    } = app;
    let data = data.as_ref().unwrap();
    let init_pipeline = init_pipeline.as_ref().unwrap();
    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });

    {
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Init"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(init_pipeline);
        cpass.set_bind_group(0, &data.init_bind_group, &[]);
        cpass.set_push_constants(
            0,
            bytemuck::cast_slice(&[GameOfLifePushConstants::new(Vec2::ZERO, Vec2::ZERO, 0.0)]),
        );
        cpass.dispatch_workgroups(WIDTH / 8, HEIGHT / 8, 1);
    }
    context.queue().submit(Some(encoder.finish()));
}

struct CanvasData {
    canvas: Texture,
    data_in: Texture,
    canvas_bind_group: BindGroup,
    init_bind_group: BindGroup,
    draw_bind_group: BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GameOfLifePushConstants {
    draw_start: [f32; 2],
    draw_end: [f32; 2],
    draw_radius: f32,
}

impl GameOfLifePushConstants {
    pub fn new(draw_start: Vec2, draw_end: Vec2, draw_radius: f32) -> Self {
        Self {
            draw_start: draw_start.to_array(),
            draw_end: draw_end.to_array(),
            draw_radius,
        }
    }
}

fn create_canvas_data(
    context: &GlassContext,
    quad_pipeline: &QuadPipeline,
    init_pipeline: &ComputePipeline,
    draw_pipeline: &ComputePipeline,
) -> CanvasData {
    let canvas = Texture::empty(
        context.device(),
        "canvas.png",
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        1,
        TextureFormat::Rgba16Float,
        TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
    );
    let data_in = Texture::empty(
        context.device(),
        "data_in.png",
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        1,
        TextureFormat::Rgba16Float,
        TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
    );
    // Create bind groups to match pipeline layouts (except update, create that dynamically each frame)
    let canvas_bind_group = quad_pipeline.create_bind_group(
        context.device(),
        &canvas.views[0],
        context.sampler_linear_clamp_to_edge(),
    );
    // These must match the bind group layout of our pipeline
    let init_bind_group_layout = init_pipeline.get_bind_group_layout(0);
    let init_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Init Bind Group"),
        layout: &init_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&canvas.views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&data_in.views[0]),
            },
        ],
    });
    let draw_bing_group_layout = draw_pipeline.get_bind_group_layout(0);
    let draw_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Draw Bind Group"),
        layout: &draw_bing_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&data_in.views[0]),
        }],
    });
    CanvasData {
        canvas,
        data_in,
        canvas_bind_group,
        init_bind_group,
        draw_bind_group,
    }
}

fn create_game_of_life_pipeline(
    context: &GlassContext,
) -> (ComputePipeline, ComputePipeline, ComputePipeline) {
    let dr_layout = context
        .device()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: StorageTextureAccess::ReadWrite,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    format: TextureFormat::Rgba16Float,
                },
                count: None,
            }],
            label: Some("draw_bind_group_layout"),
        });
    let bg_layout = context
        .device()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        format: TextureFormat::Rgba16Float,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        format: TextureFormat::Rgba16Float,
                    },
                    count: None,
                },
            ],
            label: Some("gol_bind_group_layout"),
        });

    let game_of_life_shader = context
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("game_of_life.wgsl"))),
        });
    let brush_shader = context
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("draw.wgsl"))),
        });

    let game_of_life_init_layout =
        context
            .device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Game of Life Init Layout"),
                bind_group_layouts: &[&bg_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..std::mem::size_of::<GameOfLifePushConstants>() as u32,
                }],
            });
    let init_pipeline = context
        .device()
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Init Pipeline"),
            layout: Some(&game_of_life_init_layout),
            module: &game_of_life_shader,
            entry_point: Some("init"),
            compilation_options: Default::default(),
            cache: None,
        });

    let game_of_life_layout =
        context
            .device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Game of Life Layout"),
                bind_group_layouts: &[&bg_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..std::mem::size_of::<GameOfLifePushConstants>() as u32,
                }],
            });
    let update_pipeline = context
        .device()
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Update Pipeline"),
            layout: Some(&game_of_life_layout),
            module: &game_of_life_shader,
            entry_point: Some("update"),
            compilation_options: Default::default(),
            cache: None,
        });

    let draw_layout = context
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Draw Layout"),
            bind_group_layouts: &[&dr_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..std::mem::size_of::<GameOfLifePushConstants>() as u32,
            }],
        });
    let draw_pipeline = context
        .device()
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Draw Pipeline"),
            layout: Some(&draw_layout),
            module: &brush_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

    (init_pipeline, update_pipeline, draw_pipeline)
}

fn camera_projection(screen_size: [f32; 2]) -> glam::Mat4 {
    let half_width = screen_size[0] / 2.0;
    let half_height = screen_size[1] / 2.0;
    OPENGL_TO_WGPU
        * glam::Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            0.0,
            1000.0,
        )
}
