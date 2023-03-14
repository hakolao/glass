use std::{borrow::Cow, time::Instant};

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use glass::{
    device_context::DeviceConfig, pipelines::PipelineKey, texture::Texture, window::WindowConfig,
    Glass, GlassApp, GlassConfig, GlassContext, RenderData,
};
use wgpu::{
    AddressMode, Backends, BindGroup, BindGroupDescriptor, CommandEncoder, ComputePassDescriptor,
    ComputePipelineDescriptor, Extent3d, FilterMode, Limits, PowerPreference, PresentMode,
    PushConstantRange, SamplerDescriptor, ShaderStages, StorageTextureAccess, TextureFormat,
    TextureUsages,
};
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
};

const INIT_PIPELINE: PipelineKey = PipelineKey::new("Game Of Life Init");
const GAME_OF_LIFE_PIPELINE: PipelineKey = PipelineKey::new("Game Of Life");
const BRUSH_PIPELINE: PipelineKey = PipelineKey::new("Brush");
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
        with_common_pipelines: true,
        device_config: DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            limits: Limits {
                // Using 32 * 32 work group size
                max_compute_invocations_per_workgroup: 1024,
                ..Limits::default()
            },
            backends: Backends::VULKAN,
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

fn main() {
    Glass::new(GameOfLifeApp::default(), config()).run();
}

// Think of this like reading a "table of contents".
// - Start is run before event loop
// - Input is run on winit input
// - Update is run every frame
// - Render is run for each window after update every frame
impl GlassApp for GameOfLifeApp {
    fn start(&mut self, _event_loop: &EventLoop<()>, context: &mut GlassContext) {
        // Create pipelines
        create_game_of_life_pipeline(context);
        // Create data & bindgroups to match pipelines
        self.data = Some(create_canvas_data(context));
        init_game_of_life(self, context);
    }

    fn input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &EventLoopWindowTarget<()>,
        event: &Event<()>,
    ) {
        handle_inputs(self, event);
    }

    fn update(&mut self, context: &mut GlassContext) {
        run_update(self, context);
    }

    fn render(&mut self, context: &GlassContext, render_data: RenderData) {
        render(self, context, render_data);
    }
}

struct GameOfLifeApp {
    data: Option<CanvasData>,
    cursor_pos: Vec2,
    prev_cursor_pos: Option<Vec2>,
    draw: bool,
    dt_sum: f32,
    num_dts: f32,
    time: Instant,
    updated_time: Instant,
    count: usize,
}

impl Default for GameOfLifeApp {
    fn default() -> Self {
        Self {
            data: None,
            cursor_pos: Default::default(),
            prev_cursor_pos: None,
            draw: false,
            dt_sum: 0.0,
            num_dts: 0.0,
            time: Instant::now(),
            updated_time: Instant::now(),
            count: 0,
        }
    }
}

impl GameOfLifeApp {
    fn data(&self) -> &CanvasData {
        self.data.as_ref().unwrap()
    }

    fn cursor_to_canvas(&self, width: f32, height: f32) -> (Vec2, Vec2) {
        let half_screen = Vec2::new(width, height) / 2.0;
        let current_canvas_pos = self.cursor_pos - half_screen + WIDTH as f32 / 2.0;
        let prev_canvas_pos =
            self.prev_cursor_pos.unwrap_or(current_canvas_pos) - half_screen + HEIGHT as f32 / 2.0;
        (current_canvas_pos, prev_canvas_pos)
    }
}

fn run_update(app: &mut GameOfLifeApp, context: &mut GlassContext) {
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
    // Submit
    context.queue().submit(Some(encoder.finish()));
}

fn render(app: &mut GameOfLifeApp, context: &GlassContext, render_data: RenderData) {
    let RenderData {
        encoder,
        frame,
        window,
        ..
    } = render_data;
    let (width, height) = {
        let size = window.window().inner_size();
        (size.width as f32, size.height as f32)
    };
    let canvas_data = app.data();
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let canvas_pipeline = &context.common_pipeline().quad;
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        canvas_pipeline.draw(
            &mut rpass,
            &canvas_data.canvas_bind_group,
            [0.0; 4],
            camera_projection([width, height]).to_cols_array_2d(),
            canvas_data.canvas.size,
        );
    }
}

fn handle_inputs(app: &mut GameOfLifeApp, event: &Event<()>) {
    if let Event::WindowEvent {
        event, ..
    } = event
    {
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
                if state == &ElementState::Pressed {
                    app.draw = true;
                } else {
                    app.draw = false;
                }
            }
            _ => (),
        }
    }
}

fn draw_game_of_life(
    app: &mut GameOfLifeApp,
    context: &mut GlassContext,
    encoder: &mut CommandEncoder,
) {
    let (width, height) = {
        let size = context.primary_render_window().window().inner_size();
        (size.width as f32, size.height as f32)
    };
    let draw_pipeline = context.compute_pipeline(&BRUSH_PIPELINE).unwrap();

    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("Update"),
    });
    let (end, start) = app.cursor_to_canvas(width, height);
    let pc = GameOfLifePushConstants::new(start, end, 10.0);
    cpass.set_pipeline(draw_pipeline);
    cpass.set_bind_group(0, &app.data().draw_bind_group, &[]);
    cpass.set_push_constants(0, bytemuck::cast_slice(&[pc]));
    cpass.dispatch_workgroups(WIDTH / 32, HEIGHT / 32, 1);
}

fn update_game_of_life(
    app: &mut GameOfLifeApp,
    context: &GlassContext,
    encoder: &mut CommandEncoder,
) {
    let update_pipeline = context.compute_pipeline(&GAME_OF_LIFE_PIPELINE).unwrap();
    let (canvas, data_in) = if app.count % 2 == 0 {
        (&app.data().canvas.view, &app.data().data_in.view)
    } else {
        (&app.data().data_in.view, &app.data().canvas.view)
    };
    let update_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Update Bind Group"),
        layout: &update_pipeline.get_bind_group_layout(0),
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
    });
    let pc = GameOfLifePushConstants::new(Vec2::ZERO, Vec2::ZERO, 0.0);
    cpass.set_pipeline(update_pipeline);
    cpass.set_bind_group(0, &update_bind_group, &[]);
    cpass.set_push_constants(0, bytemuck::cast_slice(&[pc]));
    cpass.dispatch_workgroups(WIDTH / 32, HEIGHT / 32, 1);

    app.count += 1;
}

fn init_game_of_life(app: &mut GameOfLifeApp, context: &mut GlassContext) {
    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });
    let init_pipeline = context.compute_pipeline(&INIT_PIPELINE).unwrap();

    {
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Init"),
        });
        cpass.set_pipeline(init_pipeline);
        cpass.set_bind_group(0, &app.data().init_bind_group, &[]);
        cpass.set_push_constants(
            0,
            bytemuck::cast_slice(&[GameOfLifePushConstants::new(Vec2::ZERO, Vec2::ZERO, 0.0)]),
        );
        cpass.dispatch_workgroups(WIDTH / 32, HEIGHT / 32, 1);
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

fn create_canvas_data(context: &GlassContext) -> CanvasData {
    let canvas = Texture::empty(
        context.device(),
        "canvas.png",
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureFormat::Rgba16Float,
        &SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        },
        TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
    )
    .unwrap();
    let data_in = Texture::empty(
        context.device(),
        "data_in.png",
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureFormat::Rgba16Float,
        &SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        },
        TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
    )
    .unwrap();
    // Create bind groups to match pipeline layouts (except update, create that dynamically each frame)
    let init_pipeline = &context.compute_pipeline(&INIT_PIPELINE).unwrap();
    let draw_pipeline = &context.compute_pipeline(&BRUSH_PIPELINE).unwrap();
    let canvas_bind_group = context.common_pipeline().quad.create_bind_group(
        context.device(),
        &canvas.view,
        &canvas.sampler,
    );
    // These must match the bind group layout of our pipeline
    let init_bind_group_layout = init_pipeline.get_bind_group_layout(0);
    let init_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Init Bind Group"),
        layout: &init_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&canvas.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&data_in.view),
            },
        ],
    });
    let draw_bing_group_layout = draw_pipeline.get_bind_group_layout(0);
    let draw_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Draw Bind Group"),
        layout: &draw_bing_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&data_in.view),
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

fn create_game_of_life_pipeline(context: &mut GlassContext) {
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
            entry_point: "init",
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
            entry_point: "update",
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
            entry_point: "main",
        });

    context.add_compute_pipeline(INIT_PIPELINE, init_pipeline);
    context.add_compute_pipeline(GAME_OF_LIFE_PIPELINE, update_pipeline);
    context.add_compute_pipeline(BRUSH_PIPELINE, draw_pipeline);
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
