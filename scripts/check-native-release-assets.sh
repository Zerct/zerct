#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$script_dir/lib/repo-root.sh"
repo_root="$(tovuk_repo_root "$script_dir")"
cd "$repo_root"

version="${1:-}"
wait_seconds="${2:-0}"
if [ -z "$version" ]; then
  version="$(awk -F '"' '/^version = / { print $2; exit }' crates/tovuk/Cargo.toml)"
fi

deadline=$((SECONDS + wait_seconds))
tag="v$version"
required_assets=(
  "tovuk-$version-x86_64-unknown-linux-gnu"
  "tovuk-$version-x86_64-unknown-linux-gnu.sha256"
  "tovuk-$version-aarch64-unknown-linux-gnu"
  "tovuk-$version-aarch64-unknown-linux-gnu.sha256"
  "tovuk-$version-aarch64-apple-darwin"
  "tovuk-$version-aarch64-apple-darwin.sha256"
  "tovuk-$version-x86_64-apple-darwin"
  "tovuk-$version-x86_64-apple-darwin.sha256"
  "tovuk-$version-x86_64-pc-windows-msvc.exe"
  "tovuk-$version-x86_64-pc-windows-msvc.exe.sha256"
)

while true; do
  assets="$(gh release view "$tag" --json assets --jq '.assets[].name' 2>/dev/null || true)"
  missing=()
  for asset in "${required_assets[@]}"; do
    if ! grep -Fx "$asset" <<<"$assets" >/dev/null; then
      missing+=("$asset")
    fi
  done

  if [ "${#missing[@]}" -eq 0 ]; then
    printf 'All native Tovuk release assets exist for %s.\n' "$tag"
    exit 0
  fi

  if [ "$SECONDS" -ge "$deadline" ]; then
    printf 'Missing native Tovuk release assets for %s:\n' "$tag" >&2
    printf '%s\n' "${missing[@]}" >&2
    exit 1
  fi

  sleep 20
done
