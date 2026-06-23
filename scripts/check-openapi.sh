#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

vacuum_version="${VACUUM_VERSION:-0.26.6}"
docs_openapi_path="$(scripts/check-public-contracts.sh openapi-path)"

if [ ! -f "$docs_openapi_path" ]; then
  echo "Missing OpenAPI file referenced by docs/docs.json: $docs_openapi_path" >&2
  exit 1
fi

openapi_files=()
while IFS= read -r openapi_file; do
  openapi_files+=("$openapi_file")
done < <(
  git ls-files |
    awk 'tolower($0) ~ "(^|[/._-])(openapi|swagger)([/._-]|$)" && tolower($0) ~ "\\.(json|ya?ml)$" { print }' |
    sort -u
)

if [ "${#openapi_files[@]}" -eq 0 ]; then
  echo "No OpenAPI files found." >&2
  exit 1
fi

if ! printf '%s\n' "${openapi_files[@]}" | grep -Fxq "$docs_openapi_path"; then
  echo "docs/docs.json references $docs_openapi_path, but it was not discovered as an OpenAPI file." >&2
  exit 1
fi

vacuum_bin="$(VACUUM_VERSION="${vacuum_version#v}" scripts/install-vacuum.sh)"
installed_vacuum_version="$("$vacuum_bin" version)"
if [ "$installed_vacuum_version" != "${vacuum_version#v}" ]; then
  echo "vacuum ${vacuum_version#v} is required; found $installed_vacuum_version." >&2
  exit 1
fi

"$vacuum_bin" lint \
  --ruleset .vacuum.yaml \
  --hard-mode \
  --fail-severity hint \
  --min-score 100 \
  --details \
  --all-results \
  --no-style \
  --no-banner \
  "${openapi_files[@]}"
