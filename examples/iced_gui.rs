use glass::{iced_utils::IcedRenderer, Glass, GlassApp, GlassConfig, GlassContext, RenderData};
use iced_graphics::{Alignment, Size};
use iced_native::{
    program::State, renderer::Style, row, widget::button, Command, Element, Program, Theme,
};
use iced_wgpu::Renderer;
use winit::{
    event::Event,
    event_loop::{EventLoop, EventLoopWindowTarget},
};

fn main() {
    Glass::new(GuiApp::default(), GlassConfig::default()).run();
}

impl GlassApp for GuiApp {
    fn start(&mut self, _event_loop: &EventLoop<()>, context: &mut GlassContext) {
        initialize_gui_app(self, context);
    }

    fn input(
        &mut self,
        context: &mut GlassContext,
        _event_loop: &EventLoopWindowTarget<()>,
        event: &Event<()>,
    ) {
        handle_input(self, context, event);
    }

    fn render(&mut self, context: &GlassContext, render_data: RenderData) {
        render(self, context, render_data);
    }

    fn after_render(&mut self, _context: &GlassContext) {
        self.gui().renderer.after_render();
    }
}

#[derive(Default)]
struct GuiApp {
    gui: Option<Gui>,
}

impl GuiApp {
    fn gui(&mut self) -> &mut Gui {
        self.gui.as_mut().unwrap()
    }
}

struct Gui {
    renderer: IcedRenderer,
    state: State<GuiProgram>,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
}

struct GuiProgram;

impl Program for GuiProgram {
    type Message = Message;
    type Renderer = Renderer;

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        println!("Pressed the button");
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        row![button("Hello").on_press(Message::None),]
            .padding(10)
            .spacing(20)
            .align_items(Alignment::Center)
            .into()
    }
}

fn initialize_gui_app(app: &mut GuiApp, context: &mut GlassContext) {
    let window = context.primary_render_window();
    let physical_size = window.window().inner_size();
    let mut iced_renderer = IcedRenderer::new(
        context.device(),
        window.window(),
        window.surface_format(context.adapter()),
    );
    let state = State::new(
        GuiProgram,
        Size::new(physical_size.width as f32, physical_size.height as f32),
        &mut iced_renderer.renderer,
        &mut iced_renderer.debug,
    );

    app.gui = Some(Gui {
        renderer: iced_renderer,
        state,
    });
}

fn handle_input(app: &mut GuiApp, context: &GlassContext, event: &Event<()>) {
    let gui = app.gui();
    gui.renderer
        .update_with_event(&mut gui.state, context, event);
}

fn render(app: &mut GuiApp, context: &GlassContext, render_data: RenderData) {
    let RenderData {
        encoder,
        frame,
        ..
    } = render_data;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    {
        let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        // Render Your Scene Here if you have one...
    }
    let gui = app.gui();
    gui.renderer.render(
        &mut gui.state,
        &Theme::Dark,
        &Style {
            text_color: iced_native::Color::WHITE,
        },
        context.device(),
        encoder,
        &view,
    );
}
