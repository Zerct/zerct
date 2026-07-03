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

mode="${1:-sync}"
check_only=0
case "$mode" in
  sync)
    ;;
  --check | check)
    check_only=1
    ;;
  *)
    echo "usage: $0 [sync|--check]" >&2
    exit 2
    ;;
esac

source_manifest="native-release-targets.json"
generated_manifests=(
  "packages/tovuk/native-release-targets.json"
  "packages/tovuk-py/src/tovuk/native_release_targets.json"
)

sync_manifest() {
  local generated_manifest="$1"

  if [ "$check_only" -eq 1 ]; then
    if ! cmp -s "$source_manifest" "$generated_manifest"; then
      echo "$generated_manifest is stale; run scripts/sync-native-release-targets.sh" >&2
      return 1
    fi
    return 0
  fi

  mkdir -p "$(dirname -- "$generated_manifest")"
  if ! cmp -s "$source_manifest" "$generated_manifest"; then
    cp "$source_manifest" "$generated_manifest"
  fi
}

for generated_manifest in "${generated_manifests[@]}"; do
  sync_manifest "$generated_manifest"
done
