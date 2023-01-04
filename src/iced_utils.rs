use iced_native::{program::State, renderer::Style, Program, Theme};
use iced_winit::conversion;
use wgpu::{CommandEncoder, Device, TextureFormat, TextureView};
use winit::{
    event::{Event, WindowEvent},
    window::Window,
};

use crate::GlassContext;

/// A utility struct to ease rendering with iced gui library.
pub struct IcedRenderer {
    pub viewport: iced_wgpu::Viewport,
    pub renderer: iced_wgpu::Renderer,
    pub modifiers: winit::event::ModifiersState,
    pub clipboard: iced_winit::Clipboard,
    pub cursor_position: winit::dpi::PhysicalPosition<f64>,
    pub staging_belt: wgpu::util::StagingBelt,
    pub debug: iced_native::Debug,
}

impl IcedRenderer {
    pub fn new(device: &Device, window: &Window, format: TextureFormat) -> IcedRenderer {
        let physical_size = window.inner_size();
        let viewport = iced_wgpu::Viewport::with_physical_size(
            iced_graphics::Size::new(physical_size.width, physical_size.height),
            window.scale_factor(),
        );
        let renderer = iced_wgpu::Renderer::new(iced_wgpu::Backend::new(
            device,
            iced_wgpu::Settings::default(),
            format,
        ));
        let debug = iced_native::Debug::new();
        let cursor_position = winit::dpi::PhysicalPosition::new(-1.0, -1.0);
        let modifiers = winit::event::ModifiersState::default();
        let clipboard = iced_winit::Clipboard::connect(window);
        let staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
        IcedRenderer {
            viewport,
            renderer,
            modifiers,
            clipboard,
            cursor_position,
            staging_belt,
            debug,
        }
    }

    /// Update your iced program state with winit event. In addition, update the [`IcedRenderer`]
    /// viewport state.
    pub fn update_with_event<P>(
        &mut self,
        state: &mut iced_native::program::State<P>,
        context: &GlassContext,
        event: &Event<()>,
    ) where
        P: iced_native::Program + 'static,
        <P::Renderer as iced_native::Renderer>::Theme: iced_native::application::StyleSheet,
    {
        match event {
            Event::WindowEvent {
                window_id,
                event,
                ..
            } => {
                let window = context
                    .render_window(*window_id)
                    .expect(&format!("No window with id {:?}", window_id))
                    .window();
                match event {
                    WindowEvent::CursorMoved {
                        position, ..
                    } => {
                        self.cursor_position = *position;
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        self.modifiers = *new_modifiers;
                    }
                    WindowEvent::Resized(size) => {
                        self.viewport = iced_wgpu::Viewport::with_physical_size(
                            iced_graphics::Size::new(size.width, size.height),
                            window.scale_factor(),
                        );
                    }

                    _ => {}
                }

                // Map window event to iced event
                if let Some(event) = iced_winit::conversion::window_event(
                    &event,
                    window.scale_factor(),
                    self.modifiers,
                ) {
                    state.queue_event(event);
                }
                // Update the mouse cursor
                window.set_cursor_icon(iced_winit::conversion::mouse_interaction(
                    state.mouse_interaction(),
                ));
            }
            _ => {}
        }
    }

    /// Add draw commands for iced to render command queue
    pub fn render<
        P: Program<Renderer = iced_graphics::Renderer<iced_wgpu::Backend, iced_native::Theme>>,
    >(
        &mut self,
        state: &mut State<P>,
        theme: &Theme,
        style: &Style,
        device: &Device,
        encoder: &mut CommandEncoder,
        view: &TextureView,
    ) {
        let IcedRenderer {
            viewport,
            renderer,
            debug,
            staging_belt,
            clipboard,
            cursor_position,
            ..
        } = self;
        if !state.is_queue_empty() {
            // We update iced state
            let _ = state.update(
                viewport.logical_size(),
                conversion::cursor_position(*cursor_position, viewport.scale_factor()),
                renderer,
                theme,
                style,
                clipboard,
                debug,
            );
        }

        // And then iced on top
        renderer.with_primitives(|backend, primitive| {
            backend.present(
                device,
                staging_belt,
                encoder,
                &view,
                primitive,
                &viewport,
                &debug.overlay(),
            );
        });

        // Then we submit the work
        staging_belt.finish();
    }

    pub fn after_render(&mut self) {
        self.staging_belt.recall();
    }
}
