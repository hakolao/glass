# Glass

![Apache](https://img.shields.io/badge/license-Apache-blue.svg)
![CI](https://github.com/hakolao/glass/workflows/CI/badge.svg)

`Glass` helps you skip annoying _wgpu_ boilerplate, _winit_ boilerplate and _window_ organization. You can just focus on
your
render or compute pipelines.

Example:

```rust
fn main() {
    Glass::run(GlassConfig::default(), |context| {
        // Create window if relevant
        // context.create_window("main", WindowConfig {
        //     width: 1920,
        //     height: 1080,
        //     exit_on_esc: true,
        //     ..WindowConfig::default()
        // });
        Box::new(YourApp)
    })
}

struct YourApp;

impl GlassApp for YourApp {}
```

See `example` folder for more.
