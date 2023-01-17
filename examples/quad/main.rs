use glass::{
    device_context::DeviceConfig, texture::Texture, window::WindowConfig, Glass, GlassApp,
    GlassConfig, GlassContext, RenderData,
};
use wgpu::{
    AddressMode, Backends, BindGroup, FilterMode, PowerPreference, SamplerDescriptor,
    TextureFormat, TextureUsages,
};
use winit::event_loop::EventLoop;

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
        with_common_pipelines: true,
        device_config: DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            backends: Backends::VULKAN,
            ..DeviceConfig::default()
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

        let tree_pipeline = &context.common_pipeline().quad;
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
            tree_pipeline.draw(
                &mut rpass,
                &tree_data.tree_bind_group,
                [0.0; 4],
                camera_projection([width, height]).to_cols_array_2d(),
                tree_data.tree.size,
            );
        }
    }
}

struct ExampleData {
    tree: Texture,
    tree_bind_group: BindGroup,
}

fn create_example_data(context: &GlassContext) -> ExampleData {
    let tree = create_tree_texture(context);
    // Create bind group
    let tree_bind_group = context.common_pipeline().quad.create_bind_group(
        context.device(),
        &tree.view,
        &tree.sampler,
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
