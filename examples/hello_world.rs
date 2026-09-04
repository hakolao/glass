use glass::prelude::*;

fn main() -> Result<(), GlassError> {
    Glass::run(GlassConfig::default(), |context| {
        context.create_window("main", WindowConfig {
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
    fn update(&mut self, context: &mut GlassContext) {
        context.primary_render_window_mut().render_default(|_| None);
    }
}
