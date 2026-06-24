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

shell_sources=(scripts/*.sh scripts/lib/*.sh)
shell_entrypoints=(scripts/*.sh)

bash -n "${shell_sources[@]}"
shellcheck -x "${shell_entrypoints[@]}"
shfmt -i 2 -ci -d "${shell_sources[@]}"
