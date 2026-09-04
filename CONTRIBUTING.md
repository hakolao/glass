# Contributing

## Running the checks

```bash
./run_checks.sh     # or .\run_checks.ps1 on Windows
```

That runs formatting, clippy, the tests and the documentation build — the same four things CI
runs. Run it before opening a pull request.

## Formatting needs nightly

`rustfmt.toml` uses options that are still nightly-only (`imports_granularity`, `group_imports`,
`format_strings`, `overflow_delimited_expr`, `reorder_impl_items`, `struct_lit_single_line`). A
plain `cargo fmt` on stable **silently ignores them**, so it will produce a diff CI then rejects.
Always format with:

```bash
cargo +nightly fmt
```

Everything else — building, clippy, tests — works on stable, and CI runs clippy on stable so that
lint results do not depend on which nightly you happen to have.

## Tests

`cargo test` covers the parts of the crate that do not need a GPU: the Wayland session detection,
the window-centering arithmetic, the video-mode ranking, and the vertex layouts against the types
they describe. Doctests compile-check the README snippet and the main examples.

Nothing that touches a GPU or opens a window is tested automatically, because CI runners have
neither. That work is covered by the examples, so run them before a release:

```bash
./run_all_examples.sh   # or .\run_all_examples.ps1
```

Interact with each one rather than just watching it open — press <kbd>Space</kbd> in
`multiple_windows` and `hdr`, draw in `sand`, resize a window, and press <kbd>Esc</kbd> to exit.

## MSRV

The minimum supported Rust version is in `Cargo.toml` as `rust-version`. It is set by the
dependencies rather than by this crate, so raising it needs no ceremony beyond noting it in
`CHANGELOG.md`.

## A note on dev builds

`Cargo.toml` sets `opt-level = 3` for the dev profile and for all dependencies. Debug builds of a
graphics crate are otherwise slow enough to be misleading — a shader-heavy example can drop below
interactive frame rates — at the cost of slower compiles. Override it locally if you are debugging
and want the faster edit cycle.

## Adding to the API

- Every public item needs a doc comment; `#![warn(missing_docs)]` will tell you which ones you
  missed. One clear sentence is enough.
- Fallible functions get an `# Errors` section naming which `GlassError` variants they return.
  Functions that panic get a `# Panics` section, and should usually offer a non-panicking
  alternative next to them.
- Re-export new public types explicitly from `lib.rs` and, if an app is likely to need them, from
  `prelude.rs`. The crate deliberately has no glob re-exports.
- Note anything user-visible in `CHANGELOG.md` under `Unreleased`.

## Licence

Contributions are dual licensed under Apache-2.0 and MIT, matching the crate.
