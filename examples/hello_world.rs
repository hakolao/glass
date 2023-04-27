use glass::{Glass, GlassApp, GlassConfig, GlassError};

fn main() -> Result<(), GlassError> {
    Glass::new(HelloWorld, GlassConfig::default()).run()
}

struct HelloWorld;

impl GlassApp for HelloWorld {}
