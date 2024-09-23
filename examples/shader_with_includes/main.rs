use std::{borrow::Cow, path::PathBuf};
use flume::{Receiver, unbounded};

use glass::{
    Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::{CommandBuffer, Device, MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, StoreOp, TextureFormat};
use wgpu::naga::Module;
use winit::event_loop::ActiveEventLoop;
use glass::utils::WatchedShaderModule;

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
    layout: Option<PipelineLayout>,
    shader: Option<WatchedShaderModule>,
}

impl GlassApp for TriangleApp {
    fn start(&mut self, _event_loop: &ActiveEventLoop, context: &mut GlassContext) {
        let (layout, pipeline, shader) = create_shader_and_pipeline(context);
        self.triangle_pipeline = Some(pipeline);
        self.layout = Some(layout);
        self.shader = Some(shader);
    }

    fn end_of_frame(&mut self, context: &mut GlassContext) {
        if let Some(shader) = &mut self.shader {
            let changed_paths = shader.changed_paths();
            if !changed_paths.is_empty() {
                println!("Changed\n{:#?}", changed_paths);
                shader.reload().unwrap();
                // Hot reload
                let (new_pipeline, rx) = create_pipeline_uncaptured(
                    context.device(),
                    &self.layout.as_ref().unwrap(),
                    shader.module().unwrap().into(),
                    |_device, layout, module| {
                        create_pipeline(context, layout, module)
                    },
                );
                if let Ok(err) = rx.try_recv() {
                    println!("Pipeline Creation Failed: {}", err);
                } else {
                    self.triangle_pipeline = Some(new_pipeline);
                    println!("Reloaded triangle pipeline");
                }
            }
        }
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

fn create_shader_and_pipeline(context: &GlassContext) -> (PipelineLayout, RenderPipeline, WatchedShaderModule) {
    // Dynamic includes
    let shader_module = WatchedShaderModule::new(&PathBuf::from(
        "examples/shader_with_includes/triangle_with_include.wgsl",
    ))
        .unwrap();
    let layout = context
        .device()
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
    let pipeline = create_pipeline(context, &layout, shader_module.module().unwrap().into());
    (layout, pipeline, shader_module)
}

fn create_pipeline(context: &GlassContext, layout: &PipelineLayout, module: Module) -> RenderPipeline {
    let shader = context
        .device()
        .create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Naga(Cow::Owned(module)),
        });

    let pipeline = context
        .device()
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
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

pub fn create_pipeline_uncaptured<T>(
    device: &Device,
    layout: &PipelineLayout,
    shader: Module,
    create_pipeline_fn: impl Fn(&Device, &PipelineLayout, Module) -> T,
) -> (T, Receiver<wgpu::Error>) {
    let (tx, rx) = unbounded::<wgpu::Error>();
    device.on_uncaptured_error(Box::new(move |e: wgpu::Error| {
        tx.send(e).expect("sending shader hot reload error failed");
    }));
    let p = create_pipeline_fn(device, layout, shader);
    device.on_uncaptured_error(Box::new(|e| panic!("{}", e)));
    (p, rx)
}