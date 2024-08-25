use glass::{
    window::WindowConfig, Glass, GlassApp, GlassConfig, GlassContext, GlassError, RenderData,
};
use wgpu::{Color, CommandBuffer, StoreOp};
use winit::{
    event::{DeviceEvent, DeviceId, ElementState},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

fn main() -> Result<(), GlassError> {
    Glass::run(GlassConfig::windowless(), |_| {
        Box::new(MultiWindowApp::default())
    })
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
    fn start(&mut self, event_loop: &ActiveEventLoop, context: &mut GlassContext) {
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

    fn device_input(
        &mut self,
        context: &mut GlassContext,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: &DeviceEvent,
    ) {
        if let DeviceEvent::Key(input) = event {
            if input.physical_key == PhysicalKey::Code(KeyCode::Space)
                && input.state == ElementState::Pressed
            {
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

    fn render(
        &mut self,
        _context: &GlassContext,
        render_data: RenderData,
    ) -> Option<Vec<CommandBuffer>> {
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
}
