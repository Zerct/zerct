#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$script_dir/lib/repo-root.sh"
# shellcheck source=scripts/lib/tool-path.sh
. "$script_dir/lib/tool-path.sh"
repo_root="$(tovuk_repo_root "$script_dir")"
cd "$repo_root"
tovuk_prepend_tool_path

cargo run --locked --quiet --manifest-path crates/tovuk-public-checks/Cargo.toml --bin check-public-contracts -- "$@"
