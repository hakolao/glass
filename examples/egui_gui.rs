use egui::{FullOutput, ViewportId};
use egui_demo_lib::DemoWindows;
use egui_wgpu::ScreenDescriptor;
use egui_winit::EventResponse;
use glass::{
    window::GlassWindow, Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::{CommandBuffer, CommandEncoder, StoreOp, TextureView};
use winit::{event::Event, event_loop::EventLoopWindowTarget};

fn main() -> Result<(), GlassError> {
    Glass::new_and_run(GlassConfig::default(), |event_loop, context| {
        Box::new(GuiApp {
            gui: GuiState::new(event_loop, context),
        })
    })
}

impl GlassApp for GuiApp {
    fn input(
        &mut self,
        context: &mut GlassContext,
        _event_loop: &EventLoopWindowTarget<()>,
        event: &Event<()>,
    ) {
        update_egui_with_winit_event(self, context, event);
    }

    fn render(
        &mut self,
        context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
        Some(render(self, context, render_data))
    }
}

struct GuiApp {
    gui: GuiState,
}

struct GuiState {
    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    repaint: bool,
    ui_app: DemoWindows,
}

impl GuiState {
    fn new(event_loop: &EventLoopWindowTarget<()>, context: &mut GlassContext) -> GuiState {
        let ctx = egui::Context::default();
        let pixels_per_point = context.primary_render_window().window().scale_factor() as f32;
        let egui_winit = egui_winit::State::new(
            ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(pixels_per_point),
            Some(context.device().limits().max_texture_dimension_2d as usize),
        );
        let renderer = egui_wgpu::Renderer::new(
            context.device(),
            GlassWindow::default_surface_format(),
            None,
            1,
        );
        GuiState {
            egui_ctx: ctx,
            egui_winit,
            renderer,
            repaint: false,
            ui_app: egui_demo_lib::DemoWindows::default(),
        }
    }
}

fn update_egui_with_winit_event(app: &mut GuiApp, context: &mut GlassContext, event: &Event<()>) {
    match event {
        Event::WindowEvent {
            window_id,
            event,
            ..
        } => {
            let gui = &mut app.gui;
            if let Some(window) = context.render_window(*window_id) {
                let EventResponse {
                    consumed,
                    repaint,
                } = gui.egui_winit.on_window_event(window.window(), event);
                gui.repaint = repaint;
                // Skip input if event was consumed by egui
                if consumed {
                    return;
                }
            }
        }
        _ => {}
    }
}

fn render(app: &mut GuiApp, context: &GlassContext, render_data: RenderData) -> Vec<CommandBuffer> {
    let RenderData {
        encoder,
        frame,
        ..
    } = render_data;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    render_egui(app, context, encoder, &view)
}

fn render_egui(
    app: &mut GuiApp,
    context: &GlassContext,
    encoder: &mut CommandEncoder,
    view: &TextureView,
) -> Vec<CommandBuffer> {
    let window = context.primary_render_window();
    let GuiState {
        egui_ctx,
        renderer,
        egui_winit,
        ui_app,
        ..
    } = &mut app.gui;
    let raw_input = egui_winit.take_egui_input(window.window());
    let FullOutput {
        shapes,
        textures_delta,
        ..
    } = egui_ctx.run(raw_input, |egui_ctx| {
        // Ui content
        ui_app.ui(egui_ctx);
    });
    let pixels_per_point = window.window().scale_factor() as f32;
    // creates triangles to paint
    let clipped_primitives = egui_ctx.tessellate(shapes, pixels_per_point);

    let size = window.surface_size();
    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: size,
        pixels_per_point,
    };

    // Upload all resources for the GPU.
    let user_cmd_bufs = {
        for (id, image_delta) in &textures_delta.set {
            renderer.update_texture(context.device(), context.queue(), *id, image_delta);
        }

        // Update buffers
        renderer.update_buffers(
            context.device(),
            context.queue(),
            encoder,
            &clipped_primitives,
            &screen_descriptor,
        )
    };

    // Render
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        // Here you would render your scene
        // Render Egui
        renderer.render(&mut render_pass, &*clipped_primitives, &screen_descriptor);
    }

    for id in &textures_delta.free {
        renderer.free_texture(id);
    }

    user_cmd_bufs
}
