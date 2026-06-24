#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

cargo run --locked --quiet --manifest-path crates/tovuk/Cargo.toml --example check-github-actions -- "$@"
