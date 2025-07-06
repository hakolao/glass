use winit::{
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::GlassContext;

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
    /// Run at exit
    fn end(&mut self, _context: &mut GlassContext) {}
}
