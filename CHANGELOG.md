# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Releases before 0.8.0 predate this file and are reconstructed from the commit log, so they list
only the dependency versions each one targeted — which is the part worth looking up, since `glass`
re-exports `wgpu` and `winit`.

## [Unreleased]

## [0.8.0]

The first release with documentation, tests and CI worth the name. Every public item is
documented, the crate carries a lint block, and there are 19 unit tests and 6 doctests where
before there were none.

### Breaking changes

- **A `prelude` module.** `use glass::prelude::*;` replaces the multi-line `use glass::{...}`
  block every app needed. All eight examples now use it.
- **The main types are re-exported at the crate root.** `WindowConfig`, `WindowPos`, `GlassWindow`,
  `RenderData`, `Texture`, `DeviceConfig` and `DeviceContext` join the `Glass*` types there. The
  `window`, `texture`, `device_context` and `pipelines` modules stay public, so existing paths
  still resolve.
- **Glob re-exports are gone.** `pub use glass::*` at the root and in `pipelines` used to leak any
  newly-`pub` item automatically; every export is now named.
- **`pub use image` is gone.** `image` appeared in the API only through `ImageError` and
  `DynamicImage`, so re-exporting the whole crate made its version a semver obligation for no
  benefit. `wgpu` and `winit` are still re-exported and still part of the contract.
- **`GlassError` is restructured.** It is now a `thiserror` enum whose bulky variants box their
  payload, so `Result<T, GlassError>` is cheap to return:
  - `GlassError::SurfaceConfigurationError(String)` becomes
    `GlassError::UnsupportedSurfaceFormat { requested, supported }`, which names the format you
    asked for and the ones the surface actually offers.
  - `AdapterNotFound`, `DeviceError` and `InsufficientDevice` now hold a single boxed payload
    struct (`error::AdapterNotFoundError`, `error::DeviceCreationError`,
    `error::InsufficientDeviceError`) instead of inline fields.
  - Error messages are one line each. The adapter enumeration that used to be baked into the
    `AdapterNotFound` message now goes through the `log` facade.
- **`GlassWindow::new` returns `Result<_, GlassError>`** rather than `Result<_, CreateSurfaceError>`.
- **`get_fitting_videomode` and `get_best_videomode` return `Option<VideoModeHandle>`.** A monitor
  can report no video modes at all, in which case exclusive fullscreen is simply unavailable.
- **`pipelines::colored_quad_vertices` is removed.** It was already marked `#[allow(unused)]` and
  duplicated `TEXTURED_QUAD_VERTICES`.

### Fixed

- **No more panics on recoverable failures.** Six panic sites in library code now return or log:
  - `GlassWindow::new` returned an error for an unsupported surface format in one code path and
    called `panic!` for the identical condition in another. It errors in both now.
  - A window queued with `GlassContext::create_window` used to `unwrap()` inside the event loop, so
    any failure aborted the process. The failure now ends the event loop and is returned from
    `Glass::run`, matching what `create_window_immediately` already did.
  - Recreating a lost surface, and selecting a video mode on a monitor that reports none, no longer
    panic.
  - `primary_render_window` and `primary_render_window_mut` still panic by design — they are the
    convenience form of the `_maybe` variants — but now say so under `# Panics` and carry a message
    pointing at the alternative.
- **Wayland detection can no longer contradict itself.** `utils::default_texture_format` checked
  only `WAYLAND_DISPLAY` while `GlassWindow::default_surface_format` checked `XDG_SESSION_TYPE`
  first, so the two could disagree about the same session. Both now go through one helper that
  prefers `XDG_SESSION_TYPE`, because a Wayland compositor running Xwayland exports
  `WAYLAND_DISPLAY` to X11 clients too. The formats each function returns are unchanged.

  This changes behaviour on a Linux/Xwayland session, which is the one case the old detection got
  wrong. It has not been verified on hardware — see the note in the pull request.

### Added

- `log` is now a dependency. `glass` emits the chosen adapter at `info`, and surface
  reconfiguration, device-lost recovery and window-creation failures at `warn`/`error`. Install any
  logger to see them; the library installs none.
- Crate-level documentation, generated from `README.md`, so the README snippet is compile-checked.
- A lint block: `#![forbid(unsafe_code)]` (the crate contains no `unsafe`) plus warnings for
  `missing_docs`, `missing_debug_implementations`, `rust_2018_idioms` and
  `rustdoc::broken_intra_doc_links`.
- `Debug` implementations across the public API.
- `LinePushConstants` and `QuadPushConstants` are now reachable from `pipelines`; they were `pub`
  but never re-exported.
- Package metadata: description, license, repository, keywords, categories and an MSRV of 1.87.
- `LICENSE-MIT` alongside the existing Apache-2.0 licence; the crate is now dual licensed.
- `CHANGELOG.md`, `CONTRIBUTING.md`, `.editorconfig`, and a committed `Cargo.lock`.

### Changed

- `glass.rs` was 665 lines holding the runner, the error type and the context. It is now
  `glass.rs` (runner), `error.rs` and `context.rs`.
- `examples/common/mod.rs` holds the `OPENGL_TO_WGPU` matrix and `camera_projection`, which were
  copy-pasted identically into three examples.
- CI runs on current actions (`checkout@v4`, `dtolnay/rust-toolchain`) instead of the archived
  `actions-rs`, installs the X11/Wayland system libraries a winit crate needs on Linux, builds with
  `--locked`, and checks the documentation.
- `run_checks.sh` and `run_checks.ps1` had drifted apart and neither failed on a failing step. They
  now run the same four checks and abort on the first failure.

## [0.7.0]

- wgpu 30.0, winit 0.30. Improved adapter, device and surface creation errors; surface size
  reconfiguration; `image` trimmed to the PNG and JPEG decoders.

## [0.6.0]

- wgpu 0.29. Tonemapping and bloom pipelines removed; render control left to the app.

## [0.5.0]

- wgpu 0.27, then 25 and 24 during the cycle. Shader includes removed.

## [0.4.0]

- wgpu 23, winit 0.30. Window creation API changed.

## [0.3.0]

- wgpu 0.20, egui 0.28.

## [0.2.0]

- wgpu 0.19, then 0.18. Added the `wgpu_serde` feature.

## [0.1.0]

- Initial release.

[Unreleased]: https://github.com/hakolao/glass/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/hakolao/glass/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/hakolao/glass/releases/tag/v0.7.0
[0.6.0]: https://github.com/hakolao/glass
[0.5.0]: https://github.com/hakolao/glass
[0.4.0]: https://github.com/hakolao/glass
[0.3.0]: https://github.com/hakolao/glass
[0.2.0]: https://github.com/hakolao/glass
[0.1.0]: https://github.com/hakolao/glass
