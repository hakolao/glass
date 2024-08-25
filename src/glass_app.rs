use wgpu::{CommandBuffer, CommandEncoder, StoreOp, SurfaceTexture};
use winit::{
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
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
    fn start(&mut self, _event_loop: &ActiveEventLoop, _context: &mut GlassContext) {}
    /// Run on winit's `new_events`
    fn before_input(&mut self, _context: &mut GlassContext, _event_loop: &ActiveEventLoop) {}
    /// Run on each device event from winit
    fn device_input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        _event: &DeviceEvent,
    ) {
    }
    /// Run on each window event from winit
    fn window_input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: &WindowEvent,
    ) {
    }
    /// Run each frame, called within winit's `about_to_wait`.
    fn update(&mut self, _context: &mut GlassContext) {}
    /// Run each frame for each window after update
    fn render(
        &mut self,
        _context: &GlassContext,
        _render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
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
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        None
    }
    /// Run each frame last
    fn end_of_frame(&mut self, _context: &mut GlassContext) {}
    /// Run at exit
    fn end(&mut self, _context: &mut GlassContext) {}
}
