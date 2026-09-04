# Contributing

Run `./run_checks.sh` (or `.\run_checks.ps1`) before opening a pull request. It runs the same
formatting, clippy, test and doc checks as CI.

`rustfmt.toml` uses nightly-only options, so format with `cargo +nightly fmt`. A plain
`cargo fmt` on stable silently ignores them and produces a diff CI rejects. Everything else works
on stable.

Nothing GPU-related is tested automatically. Run `./run_all_examples.sh` before a release and
interact with each example.

New public items need a doc comment, an explicit re-export in `lib.rs` (the crate has no glob
re-exports), and a `CHANGELOG.md` entry under `Unreleased`.

Contributions are dual licensed under Apache-2.0 and MIT.
