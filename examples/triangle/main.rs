use std::borrow::Cow;

use glass::{pipelines::PipelineKey, Glass, GlassApp, GlassConfig, GlassContext, RenderData};
use wgpu::{
    MultisampleState, PipelineLayoutDescriptor, PrimitiveState, RenderPipelineDescriptor,
    ShaderModuleDescriptor, TextureFormat,
};
use winit::event_loop::EventLoop;

const TRIANGLE_PIPELINE: PipelineKey = PipelineKey::new("Triangle");
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn main() {
    Glass::new(
        TriangleApp::default(),
        GlassConfig::performance(WIDTH, HEIGHT),
    )
    .run();
}

#[derive(Default)]
struct TriangleApp;

impl GlassApp for TriangleApp {
    fn start(&mut self, _event_loop: &EventLoop<()>, context: &mut GlassContext) {
        create_triangle_pipeline(context);
    }

    fn render(&mut self, context: &GlassContext, render_data: RenderData) {
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
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        let triangle_pipeline = context.draw_pipeline(&TRIANGLE_PIPELINE).unwrap();
        rpass.set_pipeline(&triangle_pipeline.pipeline);
        rpass.draw(0..3, 0..1);
    }
}

fn create_triangle_pipeline(context: &mut GlassContext) {
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
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(TextureFormat::Bgra8UnormSrgb.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });
    context.add_draw_pipeline(TRIANGLE_PIPELINE, pipeline);
}
