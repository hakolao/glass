use std::{borrow::Cow, time::Instant};

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use glass::{
    device_context::DeviceConfig,
    texture::Texture,
    utils::{PipelineKey, Pipelines},
    window::WindowConfig,
    Glass, GlassApp, GlassConfig, GlassContext, RenderData,
};
use wgpu::{
    util::DeviceExt, AddressMode, Backends, BindGroupDescriptor, Buffer, CommandEncoder,
    ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, FilterMode, Limits,
    PowerPreference, PresentMode, PushConstantRange, SamplerDescriptor, ShaderStages,
    StorageTextureAccess, TextureFormat, TextureUsages,
};
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
};

const CANVAS_QUAD_PIPELINE: PipelineKey = PipelineKey::new("Canvas");
const INIT_PIPELINE: PipelineKey = PipelineKey::new("Game Of Life Init");
const GAME_OF_LIFE_PIPELINE: PipelineKey = PipelineKey::new("Game Of Life");
const BRUSH_PIPELINE: PipelineKey = PipelineKey::new("Brush");
const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const FPS_60: f32 = 16.0 / 1000.0;

fn config() -> GlassConfig {
    GlassConfig {
        device_config: DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            limits: Limits {
                // Using push constants, up the limit
                max_push_constant_size: 256,
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
        self.data = Some(create_canvas_data(context));
        create_canvas_pipeline(self, context);
        create_game_of_life_pipeline(self, context);
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
    pipelines: Pipelines,
    data: Option<CanvasData>,
    cursor_pos: Vec2,
    prev_cursor_pos: Option<Vec2>,
    draw: bool,
    dt_sum: f32,
    num_dts: f32,
    time: Instant,
    updated_time: Instant,
}

impl Default for GameOfLifeApp {
    fn default() -> Self {
        Self {
            pipelines: Default::default(),
            data: None,
            cursor_pos: Default::default(),
            prev_cursor_pos: None,
            draw: false,
            dt_sum: 0.0,
            num_dts: 0.0,
            time: Instant::now(),
            updated_time: Instant::now(),
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

    let canvas_pipeline = &app
        .pipelines
        .draw_pipeline(&CANVAS_QUAD_PIPELINE)
        .unwrap()
        .pipeline;
    // This must match the bind group layout of our pipeline
    let canvas_bind_group_layout = canvas_pipeline.get_bind_group_layout(0);
    // Preferably should be created in advance
    let canvas_bind_group = context
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &canvas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas_data.canvas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&canvas_data.canvas.sampler),
                },
            ],
            label: Some("canvas_bind_group"),
        });
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
        rpass.set_pipeline(canvas_pipeline);
        rpass.set_bind_group(0, &canvas_bind_group, &[]);
        rpass.set_vertex_buffer(0, canvas_data.vertices.slice(..));
        rpass.set_index_buffer(canvas_data.indices.slice(..), wgpu::IndexFormat::Uint16);
        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX,
            0,
            bytemuck::cast_slice(&[QuadPushConstants::new(
                canvas_data.canvas.size,
                width,
                height,
            )]),
        );
        rpass.draw_indexed(0..(QUAD_INDICES.len() as u32), 0, 0..1);
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
    let draw_pipeline = app.pipelines.compute_pipeline(&BRUSH_PIPELINE).unwrap();
    let draw_bing_group_layout = draw_pipeline.pipeline.get_bind_group_layout(0);
    let draw_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Draw Bind Group"),
        layout: &draw_bing_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&app.data().canvas.view),
        }],
    });

    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("Update"),
    });
    let (end, start) = app.cursor_to_canvas(width, height);
    let pc = GameOfLifePushConstants::new(start, end, 10.0);
    cpass.set_pipeline(&draw_pipeline.pipeline);
    cpass.set_bind_group(0, &draw_bind_group, &[]);
    cpass.set_push_constants(0, bytemuck::cast_slice(&[pc]));
    cpass.dispatch_workgroups(WIDTH / 32, HEIGHT / 32, 1);
}

fn update_game_of_life(
    app: &mut GameOfLifeApp,
    context: &mut GlassContext,
    encoder: &mut CommandEncoder,
) {
    let update_pipeline = app
        .pipelines
        .compute_pipeline(&GAME_OF_LIFE_PIPELINE)
        .unwrap();
    let update_bing_group_layout = update_pipeline.pipeline.get_bind_group_layout(0);
    let update_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Update Bind Group"),
        layout: &update_bing_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&app.data().canvas.view),
        }],
    });
    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("Update"),
    });
    let pc = GameOfLifePushConstants::new(Vec2::ZERO, Vec2::ZERO, 0.0);
    cpass.set_pipeline(&update_pipeline.pipeline);
    cpass.set_bind_group(0, &update_bind_group, &[]);
    cpass.set_push_constants(0, bytemuck::cast_slice(&[pc]));
    cpass.dispatch_workgroups(WIDTH / 32, HEIGHT / 32, 1);
}

fn init_game_of_life(app: &mut GameOfLifeApp, context: &mut GlassContext) {
    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });
    let init_pipeline = app.pipelines.compute_pipeline(&INIT_PIPELINE).unwrap();
    let init_bind_group_layout = init_pipeline.pipeline.get_bind_group_layout(0);
    let init_bind_group = context.device().create_bind_group(&BindGroupDescriptor {
        label: Some("Init Bind Group"),
        layout: &init_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&app.data().canvas.view),
        }],
    });
    {
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Init"),
        });
        cpass.set_pipeline(&init_pipeline.pipeline);
        cpass.set_bind_group(0, &init_bind_group, &[]);
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
    vertices: Buffer,
    indices: Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct QuadPushConstants {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    scale: [f32; 2],
}

impl QuadPushConstants {
    /// Camera and model scale push constants that keep the pixel size (scale) fixed even
    /// when window is resized (width, height)
    fn new(scale: [f32; 2], width: f32, height: f32) -> Self {
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        #[rustfmt::skip]
            let opengl_to_wgpu = glam::Mat4::from_cols_array(&[
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0,
            0.0, 0.0, 0.5, 1.0,
        ]);
        QuadPushConstants {
            view_position: [0.0; 4],
            view_proj: (opengl_to_wgpu
                * glam::Mat4::orthographic_rh(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    0.0,
                    1000.0,
                ))
            .to_cols_array_2d(),
            scale,
        }
    }
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const QUAD_VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, -0.5, 0.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        tex_coords: [1.0, 1.0],
    },
];

const QUAD_INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];

fn create_canvas_pipeline(app: &mut GameOfLifeApp, context: &mut GlassContext) {
    let texture_bind_group_layout =
        context
            .device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

    let shader = context
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../quad/quad.wgsl"))),
        });

    let layout = context
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Canvas Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..std::mem::size_of::<QuadPushConstants>() as u32,
            }],
        });

    let pipeline = context
        .device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: context
                        .primary_render_window()
                        .surface_format(context.adapter()),
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
    app.pipelines
        .add_draw_pipeline(CANVAS_QUAD_PIPELINE, layout, pipeline);
}

fn create_canvas_data(context: &GlassContext) -> CanvasData {
    let canvas = create_canvas_texture(context);
    let vertices = context
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let indices = context
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

    CanvasData {
        canvas,
        vertices,
        indices,
    }
}

fn create_canvas_texture(app: &GlassContext) -> Texture {
    Texture::empty(
        app.device(),
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
    .unwrap()
}

fn create_game_of_life_pipeline(app: &mut GameOfLifeApp, context: &mut GlassContext) {
    let texture_bind_group_layout =
        context
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
                label: Some("texture_bind_group_layout"),
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
                bind_group_layouts: &[&texture_bind_group_layout],
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
                bind_group_layouts: &[&texture_bind_group_layout],
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

    let brush_layout = context
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Brush Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..std::mem::size_of::<GameOfLifePushConstants>() as u32,
            }],
        });
    let brush_pipeline = context
        .device()
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Brush Pipeline"),
            layout: Some(&brush_layout),
            module: &brush_shader,
            entry_point: "main",
        });

    app.pipelines
        .add_compute_pipeline(INIT_PIPELINE, game_of_life_init_layout, init_pipeline);
    app.pipelines
        .add_compute_pipeline(GAME_OF_LIFE_PIPELINE, game_of_life_layout, update_pipeline);
    app.pipelines
        .add_compute_pipeline(BRUSH_PIPELINE, brush_layout, brush_pipeline);
}
