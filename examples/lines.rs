use glam::{Mat4, Vec2, Vec3};
use glass::{
    device_context::DeviceConfig,
    pipelines::{ColoredVertex, Line, LinePipeline},
    window::{GlassWindow, WindowConfig},
    Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use rapier2d::prelude::*;
use wgpu::{util::DeviceExt, Buffer, CommandBuffer, Features, Limits, StoreOp};
use winit::event_loop::ActiveEventLoop;

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
    Glass::run(config(), |context| Box::new(LineApp::new(context)))
}

struct LineApp {
    line_pipeline: LinePipeline,
    physics_pipeline: PhysicsPipeline,
    physics_world: PhysicsWorld,
    view_proj: Mat4,
    lines: DebugLines,
    another_line_buffer: Option<Buffer>,
}

impl LineApp {
    fn new(context: &mut GlassContext) -> LineApp {
        LineApp {
            line_pipeline: LinePipeline::new(context.device(), wgpu::ColorTargetState {
                format: GlassWindow::default_surface_format(),
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            physics_pipeline: PhysicsPipeline::new(),
            physics_world: PhysicsWorld::new(Vec2::new(0.0, -9.81)),
            view_proj: camera_projection([WIDTH as f32, HEIGHT as f32]),
            lines: DebugLines::new(),
            another_line_buffer: None,
        }
    }
}

impl GlassApp for LineApp {
    fn start(&mut self, _event_loop: &ActiveEventLoop, context: &mut GlassContext) {
        // Add ground level
        let y_pos = 0.0;
        let ground = self
            .physics_world
            .rigid_body_set
            .insert(RigidBodyBuilder::fixed().translation(vector![0.0, y_pos]));

        // Add bridge
        let density = 20.0;
        let x_base = -8.0;
        let count = 32;
        let part_half_width = 0.25;
        let part_half_height = 0.125;
        let part_width = 2.0 * part_half_width;
        let mut prev = ground;

        for i in 0..count {
            let rigid_body = RigidBodyBuilder::dynamic()
                .linear_damping(0.1)
                .angular_damping(0.1)
                .translation(vector![
                    x_base + part_half_width + part_width * i as f32,
                    y_pos
                ]);
            let handle = self.physics_world.rigid_body_set.insert(rigid_body);
            let collider =
                ColliderBuilder::cuboid(part_half_width, part_half_height).density(density);
            self.physics_world.collider_set.insert_with_parent(
                collider,
                handle,
                &mut self.physics_world.rigid_body_set,
            );

            let pivot = point![x_base + part_width * i as f32, y_pos];
            let joint = RevoluteJointBuilder::new()
                .local_anchor1(
                    self.physics_world.rigid_body_set[prev]
                        .position()
                        .inverse_transform_point(&pivot),
                )
                .local_anchor2(
                    self.physics_world.rigid_body_set[handle]
                        .position()
                        .inverse_transform_point(&pivot),
                )
                .contacts_enabled(false);
            self.physics_world
                .impulse_joint_set
                .insert(prev, handle, joint, true);
            prev = handle;
        }

        let pivot = point![x_base + part_width * count as f32, y_pos];
        let joint = RevoluteJointBuilder::new()
            .local_anchor1(
                self.physics_world.rigid_body_set[prev]
                    .position()
                    .inverse_transform_point(&pivot),
            )
            .local_anchor2(
                self.physics_world.rigid_body_set[ground]
                    .position()
                    .inverse_transform_point(&pivot),
            )
            .contacts_enabled(false);
        self.physics_world
            .impulse_joint_set
            .insert(prev, ground, joint, true);

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

        let mut some_lines = vec![];
        let line1 = Line::new([0.0, 0.0, 0.0], [512.0, 512.0, 0.0], [1.0, 0.0, 0.0, 1.0]);
        let line2 = Line::new([512.0, 0.0, 0.0], [512.0, 512.0, 0.0], [0.0, 1.0, 0.0, 1.0]);
        let line3 = Line::new([0.0, 0.0, 0.0], [512.0, 0.0, 0.0], [0.0, 0.0, 1.0, 1.0]);
        for line in [line1, line2, line3] {
            some_lines.push(ColoredVertex::new_2d(
                [line.start[0], line.start[1]],
                line.color,
            ));
            some_lines.push(ColoredVertex::new_2d(
                [line.end[0], line.end[1]],
                line.color,
            ));
        }
        let line_vertices =
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Line Buffer"),
                    contents: bytemuck::cast_slice(&some_lines),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        self.another_line_buffer = Some(line_vertices);
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

    fn render(
        &mut self,
        _context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
        let LineApp {
            line_pipeline,
            view_proj,
            lines,
            another_line_buffer,
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
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        for line in lines.lines.iter() {
            line_pipeline.draw(&mut rpass, view_proj.to_cols_array_2d(), *line);
        }
        let another_line_buffer = another_line_buffer.as_ref().unwrap();
        line_pipeline.draw_line_buffer(
            &mut rpass,
            view_proj.to_cols_array_2d(),
            another_line_buffer,
            0..6,
        );
        None
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
            (Vec3::new(a.x, a.y, 0.0) * PHYSICS_TO_PIXELS).into(),
            (Vec3::new(b.x, b.y, 0.0) * PHYSICS_TO_PIXELS).into(),
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
    pub broad_phase: DefaultBroadPhase,
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
            broad_phase: DefaultBroadPhase::new(),
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
