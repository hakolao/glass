use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use glass::{
    device_context::DeviceConfig,
    texture::Texture,
    utils::{PipelineKey, Pipelines},
    window::WindowConfig,
    Glass, GlassApp, GlassConfig, GlassContext, RenderData,
};
use wgpu::{
    util::DeviceExt, AddressMode, Backends, Buffer, FilterMode, Limits, PowerPreference,
    PushConstantRange, SamplerDescriptor, ShaderStages, TextureFormat, TextureUsages,
};
use winit::event_loop::EventLoop;

const QUAD_TREE_PIPELINE: PipelineKey = PipelineKey::new("Quad Tree");
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn main() {
    Glass::new(TreeApp::default(), config()).run();
}

fn config() -> GlassConfig {
    GlassConfig {
        device_config: DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::PUSH_CONSTANTS,
            limits: Limits {
                // Using push constants, up the limit
                max_push_constant_size: 256,
                ..Limits::default()
            },
            backends: Backends::VULKAN,
        },
        window_configs: vec![WindowConfig {
            width: WIDTH,
            height: HEIGHT,
            exit_on_esc: true,
            ..WindowConfig::default()
        }],
    }
}

/// Example buffer data etc.
#[derive(Default)]
struct TreeApp {
    pipelines: Pipelines,
    data: Option<ExampleData>,
}

impl TreeApp {
    fn data(&self) -> &ExampleData {
        self.data.as_ref().unwrap()
    }
}

impl GlassApp for TreeApp {
    fn start(&mut self, _event_loop: &EventLoop<()>, context: &mut GlassContext) {
        self.data = Some(create_example_data(context));
        create_tree_pipeline(self, context);
    }

    fn render(&mut self, context: &GlassContext, render_data: RenderData) {
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
        let tree_data = self.data();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let tree_pipeline = &self
            .pipelines
            .draw_pipeline(&QUAD_TREE_PIPELINE)
            .unwrap()
            .pipeline;
        // This must match the bind group layout of our pipeline
        let tree_bind_group_layout = tree_pipeline.get_bind_group_layout(0);
        // Preferably should be created in advance
        let tree_bind_group = context
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &tree_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tree_data.tree.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&tree_data.tree.sampler),
                    },
                ],
                label: Some("tree_bind_group"),
            });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(tree_pipeline);
            rpass.set_bind_group(0, &tree_bind_group, &[]);
            rpass.set_vertex_buffer(0, tree_data.vertices.slice(..));
            rpass.set_index_buffer(tree_data.indices.slice(..), wgpu::IndexFormat::Uint16);
            rpass.set_push_constants(
                wgpu::ShaderStages::VERTEX,
                0,
                bytemuck::cast_slice(&[PushConstants::new(tree_data.tree.size, width, height)]),
            );
            rpass.draw_indexed(0..(QUAD_INDICES.len() as u32), 0, 0..1);
        }
    }
}

struct ExampleData {
    tree: Texture,
    vertices: Buffer,
    indices: Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    scale: [f32; 2],
}

impl PushConstants {
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
        PushConstants {
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

fn create_tree_pipeline(app: &mut TreeApp, context: &mut GlassContext) {
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
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("quad.wgsl"))),
        });

    let layout = context
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tree Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..std::mem::size_of::<PushConstants>() as u32,
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
        .add_draw_pipeline(QUAD_TREE_PIPELINE, layout, pipeline);
}

fn create_example_data(context: &GlassContext) -> ExampleData {
    let tree = create_tree_texture(context);
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

    ExampleData {
        tree,
        vertices,
        indices,
    }
}

fn create_tree_texture(app: &GlassContext) -> Texture {
    let diffuse_bytes = include_bytes!("tree.png");
    Texture::from_bytes(
        app.device(),
        app.queue(),
        diffuse_bytes,
        "tree.png",
        TextureFormat::Rgba8UnormSrgb,
        &SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        },
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
    )
    .unwrap()
}
