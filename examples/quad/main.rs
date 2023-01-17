use glass::{
    device_context::DeviceConfig,
    pipelines::{QuadPipeline, QUAD_INDICES, TEXTURED_QUAD_VERTICES},
    texture::Texture,
    utils::{PipelineKey, Pipelines},
    window::WindowConfig,
    Glass, GlassApp, GlassConfig, GlassContext, RenderData,
};
use wgpu::{
    util::DeviceExt, AddressMode, Backends, BindGroup, Buffer, FilterMode, Limits, PowerPreference,
    RenderPipeline, SamplerDescriptor, ShaderStages, TextureFormat, TextureUsages,
};
use winit::event_loop::EventLoop;

const QUAD_TREE_PIPELINE: PipelineKey = PipelineKey::new("Quad Tree");
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
#[rustfmt::skip]
const OPENGL_TO_WGPU: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

fn main() {
    Glass::new(TreeApp::default(), config()).run();
}

fn config() -> GlassConfig {
    GlassConfig {
        device_config: DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::PUSH_CONSTANTS,
            limits: Limits {
                // Using push constants in quad pipeline, up the limit
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
        create_tree_pipeline(self, context);
        // Created pipeline
        let tree_render_pipeline = &self
            .pipelines
            .draw_pipeline(&QUAD_TREE_PIPELINE)
            .unwrap()
            .pipeline;
        self.data = Some(create_example_data(context, tree_render_pipeline));
    }

    fn render(&mut self, _context: &GlassContext, render_data: RenderData) {
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
            rpass.set_bind_group(0, &tree_data.tree_bind_group, &[]);
            rpass.set_vertex_buffer(0, tree_data.vertices.slice(..));
            rpass.set_index_buffer(tree_data.indices.slice(..), wgpu::IndexFormat::Uint16);
            rpass.set_push_constants(
                ShaderStages::VERTEX,
                0,
                bytemuck::cast_slice(&[QuadPipeline::push_constants(
                    [0.0; 4],
                    camera_projection([width, height]).to_cols_array_2d(),
                    tree_data.tree.size,
                )]),
            );
            rpass.draw_indexed(0..(QUAD_INDICES.len() as u32), 0, 0..1);
        }
    }
}

struct ExampleData {
    tree: Texture,
    vertices: Buffer,
    indices: Buffer,
    tree_bind_group: BindGroup,
}

fn create_tree_pipeline(app: &mut TreeApp, context: &mut GlassContext) {
    let pipeline = QuadPipeline::new(context, wgpu::ColorTargetState {
        format: context
            .primary_render_window()
            .surface_format(context.adapter()),
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent::REPLACE,
            alpha: wgpu::BlendComponent::REPLACE,
        }),
        write_mask: wgpu::ColorWrites::ALL,
    });
    app.pipelines
        .add_draw_pipeline(QUAD_TREE_PIPELINE, pipeline);
}

fn create_example_data(context: &GlassContext, tree_pipeline: &RenderPipeline) -> ExampleData {
    let tree = create_tree_texture(context);
    let vertices = context
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(TEXTURED_QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let indices = context
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
    // Create bind group
    let tree_bind_group_layout = tree_pipeline.get_bind_group_layout(0);
    let tree_bind_group = context
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &tree_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tree.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&tree.sampler),
                },
            ],
            label: Some("tree_bind_group"),
        });
    ExampleData {
        tree,
        vertices,
        indices,
        tree_bind_group,
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
