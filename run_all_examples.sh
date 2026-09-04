#!/usr/bin/env bash
# Runs every example in turn. Needs a GPU and a display, so CI cannot do this.
set -euo pipefail

for example in hello_world triangle quad multiple_windows game_of_life lines sand hdr; do
    echo "== $example =="
    cargo run --example "$example"
done
