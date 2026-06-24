#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mkdir -p target/checks
rustc --edition=2024 scripts/check-github-actions.rs -o target/checks/check-github-actions
target/checks/check-github-actions "$@"
