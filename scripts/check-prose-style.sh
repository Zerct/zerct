#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mkdir -p target/checks
rustc --edition=2024 scripts/check-prose-style.rs -o target/checks/check-prose-style
target/checks/check-prose-style "$@"
