mod grid;
mod sand;
mod timer;

use glam::Vec2;
use glass::{
    device_context::DeviceConfig,
    pipelines::QuadPipeline,
    window::{GlassWindow, WindowConfig},
    Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::{
    Color, CommandBuffer, Limits, LoadOp, Operations, PresentMode, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{grid::Grid, sand::SandType, timer::Timer};

const CANVAS_SIZE: u32 = 512;
const CANVAS_SCALE: u32 = 2;

fn main() -> Result<(), GlassError> {
    Glass::run(config(), |context| {
        Box::new(SandSim::new(context)) as Box<dyn GlassApp>
    })
}

struct SandSim {
    grid: Grid,
    quad_pipeline: QuadPipeline,
    cursor_pos: Vec2,
    draw_sand: bool,
    draw_water: bool,
    draw_empty: bool,
    timer: Timer,
}

impl SandSim {
    pub fn new(context: &GlassContext) -> SandSim {
        let quad_pipeline = QuadPipeline::new(context.device(), wgpu::ColorTargetState {
            format: GlassWindow::default_surface_format(),
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent::OVER,
                alpha: wgpu::BlendComponent::OVER,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        });
        let grid = Grid::new(
            context.device(),
            &quad_pipeline,
            context.sampler_nearest_clamp_to_edge(),
            CANVAS_SIZE,
            CANVAS_SIZE,
        );
        SandSim {
            grid,
            quad_pipeline,
            cursor_pos: Vec2::ZERO,
            draw_sand: false,
            draw_water: false,
            draw_empty: false,
            timer: Timer::new(),
        }
    }
}

impl GlassApp for SandSim {
    fn window_input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: &WindowEvent,
    ) {
        match event {
            WindowEvent::CursorMoved {
                position, ..
            } => {
                self.cursor_pos = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.draw_sand = state == &ElementState::Pressed;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                self.draw_empty = state == &ElementState::Pressed;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Middle,
                state,
                ..
            } => {
                self.draw_water = state == &ElementState::Pressed;
            }
            _ => (),
        }
    }

    fn update(&mut self, context: &mut GlassContext) {
        if self.draw_sand || self.draw_empty || self.draw_water {
            let screen_size = context.primary_render_window().surface_size();
            let scale_factor = context.primary_render_window().window().scale_factor() as f32;
            let pos = cursor_to_canvas(
                self.cursor_pos / scale_factor,
                screen_size[0] as f32 / scale_factor,
                screen_size[1] as f32 / scale_factor,
            );
            let rounded = pos.round().as_ivec2();
            self.grid.draw_sand_radius(
                rounded.x,
                rounded.y,
                if self.draw_sand {
                    SandType::Sand
                } else if self.draw_water {
                    SandType::Water
                } else {
                    SandType::Empty
                },
                5.0,
            );
        }
        self.grid.simulate();
        self.grid.simulate();
        self.grid.update_texture(context.queue());
    }

    fn render(
        &mut self,
        _context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
        let SandSim {
            grid,
            quad_pipeline,
            ..
        } = self;
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
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            quad_pipeline.draw(
                &mut rpass,
                &grid.grid_bind_group,
                [0.0; 4],
                camera_projection([width, height]).to_cols_array_2d(),
                [
                    grid.texture.size[0] * CANVAS_SCALE as f32,
                    grid.texture.size[1] * CANVAS_SCALE as f32,
                ],
                1.0,
            );
        }
        None
    }

    fn end_of_frame(&mut self, context: &mut GlassContext) {
        self.timer.update();
        if let Some(w) = context.primary_render_window_maybe() {
            w.window()
                .set_title(&format!("Sand Grid - FPS: {:.2}", self.timer.avg_fps()));
        }
    }
}

fn cursor_to_canvas(cursor: Vec2, screen_width: f32, screen_height: f32) -> Vec2 {
    let half_screen = Vec2::new(screen_width, screen_height) / 2.0;
    Vec2::new(1.0, -1.0) * (cursor - half_screen) / CANVAS_SCALE as f32 + CANVAS_SIZE as f32 * 0.5
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

#[rustfmt::skip]
pub const OPENGL_TO_WGPU: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

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
            width: CANVAS_SIZE * CANVAS_SCALE,
            height: CANVAS_SIZE * CANVAS_SCALE,
            present_mode: PresentMode::Immediate,
            exit_on_esc: true,
            ..WindowConfig::default()
        }],
    }
}
