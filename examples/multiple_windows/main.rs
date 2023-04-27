use glass::{
    window::WindowConfig, Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::Color;
use winit::{
    event::{DeviceEvent, ElementState, Event, VirtualKeyCode},
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::WindowId,
};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

fn main() -> Result<(), GlassError> {
    Glass::new(MultiWindowApp::default(), GlassConfig::windowless()).run()
}

const CLEAR_COLORS: [Color; 5] = [
    Color::WHITE,
    Color::GREEN,
    Color::RED,
    Color::BLACK,
    Color::BLUE,
];

/// Example buffer data etc.
#[derive(Default)]
struct MultiWindowApp {
    pub window_ids: Vec<WindowId>,
}

impl GlassApp for MultiWindowApp {
    fn start(&mut self, event_loop: &EventLoop<()>, context: &mut GlassContext) {
        println!("Press space to create windows, esc to close all but last");
        self.window_ids.push(
            context
                .create_window(event_loop, WindowConfig {
                    width: WIDTH,
                    height: HEIGHT,
                    exit_on_esc: true,
                    ..WindowConfig::default()
                })
                .unwrap(),
        );
    }

    fn input(
        &mut self,
        context: &mut GlassContext,
        event_loop: &EventLoopWindowTarget<()>,
        event: &Event<()>,
    ) {
        if let Event::DeviceEvent {
            event: DeviceEvent::Key(input),
            ..
        } = event
        {
            if let Some(key) = input.virtual_keycode {
                if key == VirtualKeyCode::Space && input.state == ElementState::Pressed {
                    // Create window
                    self.window_ids.push(
                        context
                            .create_window(event_loop, WindowConfig {
                                width: WIDTH,
                                height: HEIGHT,
                                exit_on_esc: true,
                                ..WindowConfig::default()
                            })
                            .unwrap(),
                    );
                    println!("Window ids: {:#?}", self.window_ids);
                }
            }
        }
    }

    fn render(&mut self, _context: &GlassContext, render_data: RenderData) {
        let RenderData {
            encoder,
            frame,
            window,
            ..
        } = render_data;
        // Select clear color by window id
        let clear_color =
            CLEAR_COLORS[u64::from(window.window().id()) as usize % CLEAR_COLORS.len()];
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }
    }
}
