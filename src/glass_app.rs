use wgpu::{CommandEncoder, SurfaceTexture};
use winit::{
    event::Event,
    event_loop::{EventLoop, EventLoopWindowTarget},
};

use crate::{window::GlassWindow, GlassContext};

/// All necessary data required to render with wgpu. This data only lives for the duration of
/// rendering.
/// The command queue will be submitted each frame.
pub struct RenderData<'a> {
    pub encoder: &'a mut CommandEncoder,
    pub window: &'a GlassWindow,
    pub frame: &'a SurfaceTexture,
}

/// A trait to define all stages of your Glass app. Each function here is run at a specific stage
/// within winit event loop. When you impl this for your app, think of this as the
/// table of contents of your app flow.
pub trait GlassApp {
    /// Run at start
    fn start(&mut self, _event_loop: &EventLoop<()>, _context: &mut GlassContext) {}
    /// Run on each event received from winit
    fn input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &EventLoopWindowTarget<()>,
        _event: &Event<()>,
    ) {
    }
    /// Run each frame
    fn update(&mut self, _context: &mut GlassContext) {}
    /// Run each frame for each window after update
    fn render(&mut self, _context: &GlassContext, _render_data: RenderData) {
        let RenderData {
            encoder,
            frame,
            ..
        } = _render_data;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let _r = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        }
    }
    /// Run each frame for each window after rendering per window
    fn post_processing(&mut self, _context: &GlassContext, _render_data: RenderData) {}
    /// Run each frame for each window after post processing
    fn after_render(&mut self, _context: &GlassContext) {}
    /// Run each frame last
    fn end_of_frame(&mut self, _context: &mut GlassContext) {}
    /// Run at exit
    fn end(&mut self, _context: &mut GlassContext) {}
}
