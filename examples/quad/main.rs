use glass::{
    device_context::DeviceConfig,
    pipelines::QuadPipeline,
    texture::Texture,
    window::{GlassWindow, WindowConfig},
    Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::{BindGroup, CommandBuffer, Limits, StoreOp, TextureFormat, TextureUsages};
use winit::event_loop::ActiveEventLoop;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
#[rustfmt::skip]
const OPENGL_TO_WGPU: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

fn main() -> Result<(), GlassError> {
    Glass::run(config(), |_| Box::new(TreeApp::default()))
}

fn config() -> GlassConfig {
    GlassConfig {
        device_config: DeviceConfig {
            limits: Limits {
                // Needed for push constants
                max_push_constant_size: 128,
                ..Default::default()
            },
            ..DeviceConfig::performance()
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
    quad_pipeline: Option<QuadPipeline>,
    data: Option<ExampleData>,
}

impl GlassApp for TreeApp {
    fn start(&mut self, _event_loop: &ActiveEventLoop, context: &mut GlassContext) {
        let quad_pipeline = QuadPipeline::new(context.device(), wgpu::ColorTargetState {
            format: GlassWindow::default_surface_format(),
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent::OVER,
                alpha: wgpu::BlendComponent::OVER,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        });
        self.data = Some(create_example_data(context, &quad_pipeline));
        self.quad_pipeline = Some(quad_pipeline);
    }

    fn render(
        &mut self,
        _context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
        let TreeApp {
            quad_pipeline,
            data,
            ..
        } = self;
        let quad_pipeline = quad_pipeline.as_ref().unwrap();
        let tree_data = data.as_ref().unwrap();
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            quad_pipeline.draw(
                &mut rpass,
                &tree_data.tree_bind_group,
                [250.0, 250.0, 0.0, 0.0],
                camera_projection([width, height]).to_cols_array_2d(),
                tree_data.tree.size,
                1.0,
            );
        }
        None
    }
}

struct ExampleData {
    tree: Texture,
    tree_bind_group: BindGroup,
}

fn create_example_data(context: &GlassContext, quad_pipeline: &QuadPipeline) -> ExampleData {
    let tree = create_tree_texture(context);
    // Create bind group
    let tree_bind_group = quad_pipeline.create_bind_group(
        context.device(),
        &tree.views[0],
        context.sampler_linear_clamp_to_edge(),
    );
    ExampleData {
        tree,
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
