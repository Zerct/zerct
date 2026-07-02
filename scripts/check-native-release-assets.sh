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
mapfile -t required_assets < <(
  python3 - "$version" <<'PY'
import json
import sys

version = sys.argv[1]
with open("native-release-targets.json", encoding="utf-8") as handle:
    targets = json.load(handle)["targets"]

for target in targets:
    print(f"tovuk-{version}-{target['triple']}{target['asset_ext']}")
PY
)

verify_asset_checksums() {
  release_asset_tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$release_asset_tmp_dir"' EXIT

  for asset in "${required_assets[@]}"; do
    gh release download "$tag" --dir "$release_asset_tmp_dir" --clobber --pattern "$asset" >/dev/null
    gh release download "$tag" --dir "$release_asset_tmp_dir" --clobber --pattern "$asset.sha256" >/dev/null
    python3 - "$release_asset_tmp_dir/$asset" "$release_asset_tmp_dir/$asset.sha256" "$asset" <<'PY'
import hashlib
import pathlib
import sys

asset_path = pathlib.Path(sys.argv[1])
checksum_path = pathlib.Path(sys.argv[2])
asset_name = sys.argv[3]

line = next((item.strip() for item in checksum_path.read_text(encoding="utf-8").splitlines() if item.strip()), "")
if not line:
    raise SystemExit(f"{asset_name}.sha256 is empty")

parts = line.split()
digest = parts[0].lower()
if len(digest) != 64 or any(character not in "0123456789abcdef" for character in digest):
    raise SystemExit(f"{asset_name}.sha256 does not contain a SHA-256 digest")

if len(parts) > 1:
    listed_asset = pathlib.Path(" ".join(parts[1:]).lstrip("*")).name
    if listed_asset != asset_name:
        raise SystemExit(f"{asset_name}.sha256 names {listed_asset}, expected {asset_name}")

actual = hashlib.sha256(asset_path.read_bytes()).hexdigest()
if actual != digest:
    raise SystemExit(f"{asset_name} checksum mismatch: expected {digest}, got {actual}")
PY
  done
}

while true; do
  assets="$(gh release view "$tag" --json assets --jq '.assets[].name' 2>/dev/null || true)"
  missing=()
  for asset in "${required_assets[@]}"; do
    if ! grep -Fx "$asset" <<<"$assets" >/dev/null; then
      missing+=("$asset")
    fi
    if ! grep -Fx "$asset.sha256" <<<"$assets" >/dev/null; then
      missing+=("$asset.sha256")
    fi
  done

  if [ "${#missing[@]}" -eq 0 ]; then
    verify_asset_checksums
    printf 'All native Tovuk release assets exist and match checksums for %s.\n' "$tag"
    exit 0
  fi

  if [ "$SECONDS" -ge "$deadline" ]; then
    printf 'Missing native Tovuk release assets for %s:\n' "$tag" >&2
    printf '%s\n' "${missing[@]}" >&2
    exit 1
  fi

  sleep 20
done
