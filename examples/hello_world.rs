use glass::{window::WindowConfig, Glass, GlassApp, GlassConfig, GlassContext, GlassError};

fn main() -> Result<(), GlassError> {
    Glass::run(GlassConfig::default(), |context| {
        context.create_window(WindowConfig {
            width: 1920,
            height: 1080,
            exit_on_esc: true,
            ..WindowConfig::default()
        });
        Box::new(HelloWorld)
    })
}

struct HelloWorld;

impl GlassApp for HelloWorld {
    fn update(&mut self, _context: &mut GlassContext) {
        _context
            .primary_render_window_mut()
            .render_default(self, |_, _| None);
    }
}
