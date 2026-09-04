#!/usr/bin/env bash
# The same four checks CI runs. Formatting needs nightly; see CONTRIBUTING.md.
set -euo pipefail

cargo +nightly fmt -- --check --color always
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-Dwarnings" cargo doc --no-deps --all-features
