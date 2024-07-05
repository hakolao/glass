use glass::{Glass, GlassApp, GlassConfig, GlassError};

fn main() -> Result<(), GlassError> {
    Glass::run(GlassConfig::default(), |_| Box::new(HelloWorld))
}

struct HelloWorld;

impl GlassApp for HelloWorld {}
