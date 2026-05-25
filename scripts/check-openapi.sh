#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

vacuum_version="${VACUUM_VERSION:-v0.26.6}"
docs_openapi_path="$(
  node -e '
    const fs = require("node:fs");
    const config = JSON.parse(fs.readFileSync("docs/docs.json", "utf8"));
    const openapi = config.api?.openapi;
    if (!openapi) {
      process.exit(1);
    }
    console.log(`docs/${openapi}`);
  '
)"

if [ ! -f "$docs_openapi_path" ]; then
  echo "Missing OpenAPI file referenced by docs/docs.json: $docs_openapi_path" >&2
  exit 1
fi

openapi_files=()
while IFS= read -r openapi_file; do
  openapi_files+=("$openapi_file")
done < <(
  git ls-files \
    | awk 'tolower($0) ~ "(^|[/._-])(openapi|swagger)([/._-]|$)" && tolower($0) ~ "\\.(json|ya?ml)$" { print }' \
    | sort -u
)

if [ "${#openapi_files[@]}" -eq 0 ]; then
  echo "No OpenAPI files found." >&2
  exit 1
fi

if ! printf '%s\n' "${openapi_files[@]}" | grep -Fxq "$docs_openapi_path"; then
  echo "docs/docs.json references $docs_openapi_path, but it was not discovered as an OpenAPI file." >&2
  exit 1
fi

vacuum_cmd=(go run "github.com/daveshanley/vacuum@${vacuum_version}")
if command -v vacuum >/dev/null 2>&1 && [ "$(vacuum version)" = "${vacuum_version#v}" ]; then
  vacuum_cmd=(vacuum)
fi

"${vacuum_cmd[@]}" lint \
  --ruleset .vacuum.yaml \
  --hard-mode \
  --fail-severity hint \
  --min-score 100 \
  --details \
  --all-results \
  --no-style \
  --no-banner \
  "${openapi_files[@]}"
