#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

cargo run --locked --quiet --manifest-path crates/tovuk/Cargo.toml --example check-public-contracts -- "$@"
