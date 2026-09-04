# Changelog

## [Unreleased]

## [0.8.0]

### Breaking

- Added a `prelude` module: `use glass::prelude::*;` replaces the old multi-line import block.
- `WindowConfig`, `WindowPos`, `GlassWindow`, `RenderData`, `Texture`, `DeviceConfig` and
  `DeviceContext` are re-exported at the crate root. Their module paths still work.
- Removed glob re-exports; every export is now named.
- Removed `pub use image`.
- `GlassError` is now a `thiserror` enum with boxed payloads:
  - `SurfaceConfigurationError(String)` → `UnsupportedSurfaceFormat { requested, supported }`.
  - `AdapterNotFound`, `DeviceError` and `InsufficientDevice` hold one boxed struct
    (`error::AdapterNotFoundError`, `error::DeviceCreationError`, `error::InsufficientDeviceError`)
    instead of inline fields.
- `GlassWindow::new` returns `Result<_, GlassError>`, not `Result<_, CreateSurfaceError>`.
- `get_fitting_videomode` and `get_best_videomode` return `Option<VideoModeHandle>`.
- Removed the `pipelines` module. `QuadPipeline`, `LinePipeline` and the vertex types now
  live in `examples/common/pipelines/` to copy from, since drawing is the application's
  job. `colored_quad_vertices` is gone entirely.
- `GlassWindow::render_default` returns `Result<(), GlassError>`. Recoverable surface states
  (occluded, timed out, suboptimal, outdated, lost) still return `Ok` with the frame skipped; only
  a failure to rebuild the surface is an error.
- `GlassContext::new` no longer adds `IMMEDIATES` and
  `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` to your requested features. Request what your
  pipelines need in `DeviceConfig::features`.

### Fixed

- Six panic sites now return or log instead: the unsupported-format `panic!` in
  `GlassWindow::new`, the `unwrap()` on deferred `create_window` (the failure now ends the event
  loop and is returned from `Glass::run`), lost-surface recreation, and video-mode selection on a
  monitor reporting none. `primary_render_window{,_mut}` still panic by design, now documented.
- `utils::default_texture_format` and `GlassWindow::default_surface_format` used different Wayland
  detection rules and could disagree about the same session. Both now check `XDG_SESSION_TYPE`
  first, since Xwayland exports `WAYLAND_DISPLAY` to X11 clients. Return values are unchanged.
  This alters behaviour on Linux/Xwayland and has not been verified on hardware.

### Added

- `log` dependency: adapter selection at `info`, surface and window failures at `warn`/`error`.
- `trace` feature, which makes `DeviceConfig::trace_path` actually write a wgpu API trace. The
  field was previously read and discarded.
- Crate docs from `README.md`, `#![forbid(unsafe_code)]`, `missing_docs`, 19 unit tests and
  4 doctests, where there were none.
- Package metadata, MSRV 1.87, dual MIT/Apache-2.0 licensing, committed `Cargo.lock`.

### Changed

- Split `glass.rs` into `glass.rs` (runner), `error.rs` and `context.rs`.
- Moved duplicated example code into `examples/common/mod.rs`.
- CI uses current actions, installs Linux system libraries, builds `--locked` and checks docs.

## Earlier releases

No changelog was kept. Dependency versions per release:

| Version | wgpu | winit |
| ------- | ---- | ----- |
| 0.7.0   | 30.0 | 0.30  |
| 0.6.0   | 0.29 | 0.30  |
| 0.5.0   | 0.27 | 0.30  |
| 0.4.0   | 23   | 0.30  |
| 0.3.0   | 0.20 | 0.29  |
| 0.2.0   | 0.19 | 0.29  |

[Unreleased]: https://github.com/hakolao/glass/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/hakolao/glass/compare/v0.7.0...v0.8.0
