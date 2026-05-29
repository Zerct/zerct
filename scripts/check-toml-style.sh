#!/usr/bin/env bash
set -euo pipefail

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
