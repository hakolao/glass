use glam::Vec3;
use glass::{
    device_context::DeviceConfig,
    pipelines::{Line, LinePipeline},
    window::{GlassWindow, WindowConfig},
    Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::{Features, Limits};
use winit::event_loop::EventLoop;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn config() -> GlassConfig {
    GlassConfig {
        device_config: DeviceConfig {
            limits: Limits {
                max_push_constant_size: 128,
                ..Default::default()
            },
            features: Features::POLYGON_MODE_LINE,
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

fn main() -> Result<(), GlassError> {
    Glass::new(LineApp::default(), config()).run()
}

#[derive(Default)]
struct LineApp {
    line_pipeline: Option<LinePipeline>,
}

impl GlassApp for LineApp {
    fn start(&mut self, _event_loop: &EventLoop<()>, context: &mut GlassContext) {
        self.line_pipeline = Some(LinePipeline::new(
            context.device(),
            wgpu::ColorTargetState {
                format: GlassWindow::surface_format(),
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            },
        ));
    }

    fn render(&mut self, _context: &GlassContext, render_data: RenderData) {
        let LineApp {
            line_pipeline,
        } = self;
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
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        let line_pipeline = line_pipeline.as_ref().unwrap();
        let size = 500;
        let space = 10;
        for i in -(size / space)..(size / space) {
            let green = [0.0, 1.0, 0.0, 1.0];
            let red = [1.0, 0.0, 0.0, 1.0];
            let lines = [
                Line::new(
                    Vec3::new(-size as f32, (i * space) as f32, 0.0),
                    Vec3::new(size as f32, (i * space) as f32, 0.0),
                    green,
                ),
                Line::new(
                    Vec3::new((i * space) as f32, -size as f32, 0.0),
                    Vec3::new((i * space) as f32, size as f32, 0.0),
                    red,
                ),
            ];
            for line in lines {
                line_pipeline.draw(
                    &mut rpass,
                    [0.0; 4],
                    camera_projection([WIDTH as f32, HEIGHT as f32]).to_cols_array_2d(),
                    line,
                );
            }
        }
    }
}

#[rustfmt::skip]
const OPENGL_TO_WGPU: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

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
