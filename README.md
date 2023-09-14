# Glass
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)
![CI](https://github.com/hakolao/glass/workflows/CI/badge.svg)

- Don't you wish you could just read your app's flow like prose?
- Don't you wish you could just focus on _wgpu_ pipelines without any wrapping around their types?
- Don't you wish you could just access the winit event directly from the event loop without any wrapper types?

`Glass` aims to do just that. Resulting in very readable code flow for your app. Its main purposes are to allow you
to skip annoying _wgpu_ boilerplate, _winit_ boilerplate and _window_ organization. You can just focus on your
render or compute pipelines and organize your app how you like.

Example:
```rust
fn main() {
    Glass::new(MyApp, GlassConfig::default()).run();
}

// Organize your app in anyway you like
struct MyApp;

// Implement methods that you need (to render or read inputs)
impl GlassApp for MyApp {}
```

See `example` folder for more.

```rust
pub trait GlassApp {
    /// Run at start
    fn start(&mut self, _event_loop: &EventLoop<()>, _context: &mut GlassContext) {}
    /// Run on each event received from winit
    fn input(
        &mut self,
        _context: &mut GlassContext,
        _event_loop: &EventLoopWindowTarget<()>,
        _event: &Event<()>,
    ) {
    }
    /// Run each frame
    fn update(&mut self, _context: &mut GlassContext) {}
    /// Run each frame for each window after update
    fn render(&mut self, _context: &GlassContext, _render_data: RenderData) {}
    /// Run each frame for each window after post processing
    fn after_render(&mut self, _context: &GlassContext) {}
    /// Run each frame last
    fn end_of_frame(&mut self, _context: &mut GlassContext) {}
    /// Run at exit
    fn end(&mut self, _context: &mut GlassContext) {}
}
```

# For whom
- People who want to learn rendering
- People annoyed at complexities of game engines, and wanting to have more control over their app
- People who wish to go back to the roots of coding (simplicity, and no magic)