# Glass

[![CI](https://github.com/hakolao/glass/actions/workflows/tests.yml/badge.svg)](https://github.com/hakolao/glass/actions/workflows/tests.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

`Glass` helps you skip annoying _wgpu_ boilerplate, _winit_ boilerplate and _window_ organization,
so you can focus on your render or compute pipelines.

It gives you a window with a configured surface, a device and queue shared across every window, a
lifecycle trait to hang your app off, and two ready-made pipelines for the things almost everyone
needs: drawing a textured quad and drawing lines.

```rust,no_run
use glass::prelude::*;

fn main() -> Result<(), GlassError> {
    Glass::run(GlassConfig::default(), |context| {
        context.create_window("main", WindowConfig {
            width: 1920,
            height: 1080,
            exit_on_esc: true,
            ..WindowConfig::default()
        });
        Box::new(YourApp)
    })
}

struct YourApp;

impl GlassApp for YourApp {
    fn update(&mut self, context: &mut GlassContext) {
        context.primary_render_window_mut().render_default(|_render_data| None);
    }
}
```

## Install

Not published to crates.io. Depend on it from git:

```toml
[dependencies]
glass = { git = "https://github.com/hakolao/glass" }
```

## Versions

`glass` re-exports `wgpu` and `winit`, and their types appear all over its API, so their versions
are part of its contract. Use `glass::wgpu` and `glass::winit` rather than depending on them
separately.

| `glass` | `wgpu` | `winit` | MSRV |
| ------- | ------ | ------- | ---- |
| 0.8     | 30.0   | 0.30    | 1.87 |

## Features

- `wgpu_serde` — enables `serde` impls on the re-exported `wgpu` types. Off by default.

## Platforms

Built and tested on Windows, Linux and macOS through whichever backends `wgpu` picks by default.
Narrow that with [`DeviceConfig::backends`]. On Linux, `glass` detects a Wayland session and
selects a non-sRGB surface format for it, since Wayland compositors commonly do not advertise the
sRGB variant.

## Examples

Eight runnable examples live in [examples/](https://github.com/hakolao/glass/tree/master/examples), from an empty window to a compute-shader
game of life and a falling-sand simulation:

```bash
cargo run --example hello_world
cargo run --example triangle
cargo run --example quad
cargo run --example multiple_windows
cargo run --example game_of_life
cargo run --example lines
cargo run --example sand
cargo run --example hdr
```

They need a GPU, so none of them run in CI. `./run_all_examples.sh` (or `.ps1`) runs the lot.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Note that `rustfmt.toml` uses nightly-only options, so
formatting needs `cargo +nightly fmt`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.

[`DeviceConfig::backends`]: https://docs.rs/glass/latest/glass/device_context/struct.DeviceConfig.html#structfield.backends
