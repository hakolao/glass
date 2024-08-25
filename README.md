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
    Glass::run(GlassConfig::default(), |_| Box::new(YourApp))
}

// Organize your app in anyway you like
struct YourApp;

// Implement methods that you need (to render or read inputs)
impl GlassApp for YourApp {}
```

See `example` folder for more.

# For whom

- People who want to learn rendering
- People annoyed at complexities of game engines, and wanting to have more control over their app
- People who wish to go back to the roots of coding (simplicity, and no magic)