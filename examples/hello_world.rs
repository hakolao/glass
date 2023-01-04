use glass::{Glass, GlassApp, GlassConfig};

fn main() {
    Glass::new(HelloWorld, GlassConfig::default()).run();
}

struct HelloWorld;

impl GlassApp for HelloWorld {}
