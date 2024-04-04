use glam::Vec2;
use glass::{
    egui::Color32,
    pipelines::QuadPipeline,
    texture::Texture,
    wgpu::{BlendState, ColorTargetState, ColorWrites, Extent3d, TextureFormat, TextureUsages},
    window::GlassWindow,
    winit::{
        event::Event,
        event_loop::{EventLoop, EventLoopWindowTarget},
    },
    GlassApp, GlassContext, RenderData,
};
use wgpu::StoreOp;
use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

use crate::{
    camera::Camera, circle_pipeline::CirclePipeline, color::Color, fluid_sim::FluidScene,
    post_processing::PostProcessing, rectangle_pipeline::RectanglePipeline, timer::Timer,
};

pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;

pub struct FluidSimApp {
    circle_pipeline: Option<CirclePipeline>,
    rectangle_pipeline: Option<RectanglePipeline>,
    quad_pipeline: Option<QuadPipeline>,
    post_processing: Option<PostProcessing>,
    render_target: Option<Texture>,
    camera: Camera,
    input: WinitInputHelper,
    fluid_scene: FluidScene,
    timer: Timer,
}

impl FluidSimApp {
    pub fn new() -> FluidSimApp {
        FluidSimApp {
            circle_pipeline: None,
            rectangle_pipeline: None,
            quad_pipeline: None,
            render_target: None,
            post_processing: None,
            camera: Camera::new([WIDTH as f32, HEIGHT as f32]),
            input: WinitInputHelper::default(),
            fluid_scene: FluidScene::new(WIDTH as f32, HEIGHT as f32),
            timer: Timer::new(),
        }
    }
}

impl GlassApp for FluidSimApp {
    fn start(&mut self, _event_loop: &EventLoop<()>, context: &mut GlassContext) {
        self.render_target = Some(create_render_target(context));
        self.circle_pipeline = Some(CirclePipeline::new(context.device(), ColorTargetState {
            format: TextureFormat::Rgba16Float,
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: ColorWrites::ALL,
        }));
        self.rectangle_pipeline =
            Some(RectanglePipeline::new(context.device(), ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            }));
        self.quad_pipeline = Some(QuadPipeline::new(context.device(), ColorTargetState {
            format: GlassWindow::default_surface_format(),
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        }));
        self.post_processing = Some(PostProcessing::new(context));
    }

    fn input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &EventLoopWindowTarget<()>,
        event: &Event<()>,
    ) {
        self.input.update(event);
    }

    fn update(&mut self, context: &mut GlassContext) {
        context
            .primary_render_window()
            .window()
            .set_title(&format!("FPS: {:.3}", self.timer.avg_fps()));
        let (_, scroll_diff) = self.input.scroll_diff();
        if scroll_diff > 0.0 {
            self.camera.set_scale(self.camera.scale() / 1.05);
        } else if scroll_diff < 0.0 {
            self.camera.set_scale(self.camera.scale() * 1.05);
        }
        if self.input.window_resized().is_some() || self.input.scale_factor_changed().is_some() {
            self.resize(context);
        }
        // Read inputs state
        if self.input.key_pressed(KeyCode::Space) {
            self.fluid_scene.toggle_pause();
        }
        if self.input.key_pressed(KeyCode::KeyR) {
            self.fluid_scene.reset();
        }
        if self.input.key_pressed(KeyCode::KeyG) {
            self.fluid_scene.toggle_grid();
        }
        if self.input.key_pressed(KeyCode::KeyP) {
            self.fluid_scene.toggle_particles();
        }
        if self.input.key_pressed(KeyCode::KeyF) {
            self.fluid_scene.toggle_gravity();
        }
        if let Some((x, y)) = self.input.cursor() {
            let screen_size = context.primary_render_window().surface_size();
            let scale_factor = context.primary_render_window().window().scale_factor() as f32;
            let pos = cursor_to_world(
                Vec2::new(x, y) / scale_factor,
                &[
                    screen_size[0] as f32 / scale_factor,
                    screen_size[1] as f32 / scale_factor,
                ],
                &self.camera,
            );
            if self.input.mouse_pressed(0) {
                self.fluid_scene.drag(pos, true);
            }
            if self.input.mouse_held(0) {
                self.fluid_scene.drag(pos, false);
            }
            if self.input.mouse_released(0) {
                self.fluid_scene.end_drag();
            }
        }
        // Simulate
        self.fluid_scene.simulate();
    }

    fn render(&mut self, context: &GlassContext, render_data: RenderData) {
        // Render on render target
        // Paste render target over swapchain image
        let FluidSimApp {
            circle_pipeline,
            rectangle_pipeline,
            quad_pipeline,
            post_processing,
            render_target,
            camera,
            input,
            fluid_scene,
            ..
        } = self;
        let circle_pipeline = circle_pipeline.as_ref().unwrap();
        let rectangle_pipeline = rectangle_pipeline.as_ref().unwrap();
        let quad_pipeline = quad_pipeline.as_ref().unwrap();
        let post_processing = post_processing.as_ref().unwrap();
        let render_target = render_target.as_ref().unwrap();
        let window = context.primary_render_window();
        let window_size = window.surface_size();
        let scale_factor = input.scale_factor().unwrap_or(1.0) as f32;
        let window_size_f32 = [
            window_size[0] as f32 * scale_factor,
            window_size[1] as f32 * scale_factor,
        ];
        let RenderData {
            encoder,
            frame,
            ..
        } = render_data;
        // We don't need to submit our commands, because they get submitted after `render`.

        let rt_view = render_target
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // Draw scene to render target
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &rt_view,
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

            let view_proj = camera.view_proj().to_cols_array_2d();
            // Draw bounds
            rectangle_pipeline.draw(
                &mut rpass,
                view_proj,
                [0.0, 0.0],
                Color32::RED.into(),
                WIDTH as f32,
                HEIGHT as f32,
                2.0 / HEIGHT as f32,
                0.01,
            );

            // Draw circle(s)
            if fluid_scene.show_particles {
                let radius = fluid_scene.render_radius();
                for i in 0..fluid_scene.fluid.num_particles {
                    let tank_pos = fluid_scene.fluid.particle_pos[i];
                    let pos = fluid_scene.render_pos(tank_pos);
                    let color = fluid_scene.fluid.particle_color[i];
                    circle_pipeline.draw(
                        &mut rpass,
                        view_proj,
                        pos.into(),
                        Color {
                            color: [color.x, color.y, color.z, 1.0],
                        },
                        radius,
                        radius,
                        0.01,
                    );
                }
            }

            if fluid_scene.show_grid {
                let size = fluid_scene.render_cell_size();
                for x in 0..fluid_scene.fluid.f_num_x {
                    for y in 0..fluid_scene.fluid.f_num_y {
                        let fluid_pos = Vec2::new(
                            (x as f32 + 0.5) * fluid_scene.fluid.h,
                            (y as f32 + 0.5) * fluid_scene.fluid.h,
                        );
                        let pos = fluid_scene.render_pos(fluid_pos);
                        let i = x * fluid_scene.fluid.f_num_y + y;
                        let color = fluid_scene.fluid.cell_color[i];
                        rectangle_pipeline.draw(
                            &mut rpass,
                            view_proj,
                            pos.into(),
                            Color {
                                color: [color.x, color.y, color.z, 0.3],
                            },
                            size,
                            size,
                            size * 0.5,
                            0.01,
                        );
                    }
                }
            }

            // Obstacle
            let pos = fluid_scene.render_pos(fluid_scene.obstacle_pos);
            let radius = fluid_scene.render_obstacle_radius();
            circle_pipeline.draw(
                &mut rpass,
                view_proj,
                pos.into(),
                Color {
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                radius,
                radius,
                0.01,
            );
        }

        // Post Processing
        post_processing.run(context, encoder, render_target);
        let post_processed_target = post_processing.output();

        let main_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_target_bind_group = quad_pipeline.create_bind_group(
            context.device(),
            &post_processed_target.views[0],
            &post_processed_target.sampler,
        );
        // Draw render target over swapchain image
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &main_view,
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
                &render_target_bind_group,
                // Center
                [0.0, 0.0, 0.0, 0.0],
                camera.centered_projection().to_cols_array_2d(),
                window_size_f32,
                1.0,
            );
        }
    }

    fn end_of_frame(&mut self, _context: &mut GlassContext) {
        self.timer.update();
    }
}

impl FluidSimApp {
    fn resize(&mut self, context: &GlassContext) {
        let window_size = context.primary_render_window().surface_size();
        self.render_target = Some(create_render_target(context));
        self.camera
            .update(&[window_size[0] as f32, window_size[1] as f32]);
    }
}

pub fn create_render_target(context: &GlassContext) -> Texture {
    Texture::empty(
        context.device(),
        "Render Target",
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        1,
        TextureFormat::Rgba16Float,
        &Default::default(),
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    )
}

pub fn cursor_to_world(cursor_pos: Vec2, screen_size: &[f32; 2], camera: &Camera) -> Vec2 {
    (cursor_pos - Vec2::new(screen_size[0] / 2.0, screen_size[1] / 2.0))
        * camera.scale()
        // Invert y here, because we want world positions to grow up, and right
        * Vec2::new(1.0, -1.0)
        + Vec2::new(camera.pos.x, camera.pos.y)
}
