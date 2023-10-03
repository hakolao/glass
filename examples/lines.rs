use glam::{Mat4, Vec2, Vec3};
use glass::{
    device_context::DeviceConfig,
    pipelines::{Line, LinePipeline},
    window::{GlassWindow, WindowConfig},
    Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use rapier2d::prelude::*;
use wgpu::{Features, Limits};
use winit::event_loop::EventLoop;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
/// Height of screen is 10 meters. This much we need to multiply positions in physics world
/// to convert to pixels
const PHYSICS_TO_PIXELS: f32 = HEIGHT as f32 / 10.0;

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
    Glass::new(LineApp::new(), config()).run()
}

struct LineApp {
    line_pipeline: Option<LinePipeline>,
    physics_pipeline: PhysicsPipeline,
    physics_world: PhysicsWorld,
    view_proj: Mat4,
    lines: DebugLines,
}

impl LineApp {
    fn new() -> LineApp {
        LineApp {
            line_pipeline: None,
            physics_pipeline: PhysicsPipeline::new(),
            physics_world: PhysicsWorld::new(Vec2::new(0.0, -9.81)),
            view_proj: camera_projection([WIDTH as f32, HEIGHT as f32]),
            lines: DebugLines::new(),
        }
    }
}

impl GlassApp for LineApp {
    fn start(&mut self, _event_loop: &EventLoop<()>, context: &mut GlassContext) {
        self.line_pipeline = Some(LinePipeline::new(
            context.device(),
            wgpu::ColorTargetState {
                format: GlassWindow::default_surface_format(),
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            },
        ));
        // Add ground
        let collider = ColliderBuilder::cuboid(100.0, 0.1)
            .translation(vector![0.0, -5.0])
            .build();
        self.physics_world.collider_set.insert(collider);

        // Add ball
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 10.0])
            .build();
        let collider = ColliderBuilder::ball(0.5).restitution(1.2).build();
        let ball_body_handle = self.physics_world.rigid_body_set.insert(rigid_body);
        self.physics_world.collider_set.insert_with_parent(
            collider,
            ball_body_handle,
            &mut self.physics_world.rigid_body_set,
        );
    }

    fn update(&mut self, _context: &mut GlassContext) {
        let LineApp {
            physics_pipeline,
            physics_world,
            lines,
            ..
        } = self;
        // Clear lines
        lines.clear();
        let PhysicsWorld {
            gravity,
            rigid_body_set,
            collider_set,
            integration_parameters,
            island_manager,
            broad_phase,
            narrow_phase,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            debug_render,
            ..
        } = physics_world;
        physics_pipeline.step(
            &vector![gravity.x, gravity.y],
            integration_parameters,
            island_manager,
            broad_phase,
            narrow_phase,
            rigid_body_set,
            collider_set,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            None,
            &(),
            &(),
        );

        // Update lines
        debug_render.render(
            lines,
            rigid_body_set,
            collider_set,
            impulse_joint_set,
            multibody_joint_set,
            narrow_phase,
        );
    }

    fn render(&mut self, _context: &GlassContext, render_data: RenderData) {
        let LineApp {
            line_pipeline,
            view_proj,
            lines,
            ..
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
        for line in lines.lines.iter() {
            line_pipeline.draw(&mut rpass, view_proj.to_cols_array_2d(), *line);
        }
    }
}

pub struct DebugLines {
    lines: Vec<Line>,
}

impl DebugLines {
    fn new() -> DebugLines {
        DebugLines {
            lines: vec![],
        }
    }

    fn add_line(&mut self, line: Line) {
        self.lines.push(line);
    }

    fn clear(&mut self) {
        self.lines.clear();
    }
}

impl DebugRenderBackend for DebugLines {
    fn draw_line(
        &mut self,
        _object: DebugRenderObject,
        a: Point<Real>,
        b: Point<Real>,
        color: [f32; 4],
    ) {
        let line = Line::new(
            Vec3::new(a.x, a.y, 0.0) * PHYSICS_TO_PIXELS,
            Vec3::new(b.x, b.y, 0.0) * PHYSICS_TO_PIXELS,
            color,
        );
        self.add_line(line);
    }
}

struct PhysicsWorld {
    pub gravity: Vec2,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub integration_parameters: IntegrationParameters,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub debug_render: DebugRenderPipeline,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec2) -> PhysicsWorld {
        PhysicsWorld {
            gravity,
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::default(),
            debug_render: DebugRenderPipeline::new(
                DebugRenderStyle::default(),
                DebugRenderMode::default(),
            ),
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
        * Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            0.0,
            1000.0,
        )
}
