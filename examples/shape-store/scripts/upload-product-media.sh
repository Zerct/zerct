#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd "$script_dir/.." && pwd)"
source_dir="$project_dir/web/public/products"
output_dir="${TOVUK_PRODUCT_MEDIA_DIR:-$project_dir/.tovuk/product-media}"
service="${TOVUK_SERVICE:-shape-store}"
remote_prefix="${TOVUK_PRODUCT_MEDIA_PREFIX:-products}"
if [[ -n "${TOVUK_CLI:-}" ]]; then
  read -r -a tovuk_cli <<<"$TOVUK_CLI"
else
  tovuk_cli=(npx -y tovuk@latest)
fi

if ! command -v sips >/dev/null 2>&1; then
  printf 'sips is required to render SVG product assets to PNG on macOS.\n' >&2
  exit 1
fi

mkdir -p "$output_dir"

for svg in "$source_dir"/*.svg; do
  name="$(basename "$svg" .svg)"
  sips -s format png --out "$output_dir/$name.png" "$svg" >/dev/null
done

if [[ "${TOVUK_GENERATE_ONLY:-0}" == "1" ]]; then
  printf 'generated %s PNG product media files in %s\n' "$(find "$output_dir" -name '*.png' | wc -l | tr -d ' ')" "$output_dir"
  exit 0
fi

for png in "$output_dir"/*.png; do
  remote_path="$remote_prefix/$(basename "$png")"
  "${tovuk_cli[@]}" storage upload \
    --service "$service" \
    "$png" \
    "$remote_path" \
    --public \
    --content-type image/png \
    --json
done

"${tovuk_cli[@]}" storage list --service "$service" --json
