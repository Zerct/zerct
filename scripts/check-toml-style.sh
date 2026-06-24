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

toml_files=()
while IFS= read -r -d '' file; do
  toml_files+=("$file")
done < <(
  find . \
    \( -path '*/.git' -o -path '*/target' -o -path '*/node_modules' \) -prune \
    -o -type f -name '*.toml' -print0
)

if ((${#toml_files[@]} == 0)); then
  exit 0
fi

taplo format --check "${toml_files[@]}"
taplo lint --no-schema "${toml_files[@]}"
