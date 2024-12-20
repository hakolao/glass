use std::borrow::Cow;

use glass::{Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData};
use wgpu::{
    CommandBuffer, MultisampleState, PipelineLayoutDescriptor, PrimitiveState, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, StoreOp, TextureFormat,
};
use winit::event_loop::ActiveEventLoop;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn main() -> Result<(), GlassError> {
    Glass::run(GlassConfig::performance(WIDTH, HEIGHT), |_| {
        Box::new(TriangleApp::default())
    })
}

#[derive(Default)]
struct TriangleApp {
    triangle_pipeline: Option<RenderPipeline>,
}

impl GlassApp for TriangleApp {
    fn start(&mut self, _event_loop: &ActiveEventLoop, context: &mut GlassContext) {
        self.triangle_pipeline = Some(create_triangle_pipeline(context));
    }

    fn render(
        &mut self,
        _context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
        let RenderData {
            encoder,
            frame,
            ..
        } = render_data;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let triangle_pipeline = self.triangle_pipeline.as_ref().unwrap();
        rpass.set_pipeline(triangle_pipeline);
        rpass.draw(0..3, 0..1);
        None
    }
}

fn create_triangle_pipeline(context: &GlassContext) -> RenderPipeline {
    let shader = context
        .device()
        .create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("triangle.wgsl"))),
        });
    let layout = context
        .device()
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
    let pipeline = context
        .device()
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(TextureFormat::Bgra8UnormSrgb.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
    pipeline
}
