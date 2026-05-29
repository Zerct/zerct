#!/usr/bin/env bash
set -euo pipefail

go_files=()
while IFS= read -r -d '' file; do
  go_files+=("$file")
done < <(
  find . \
    \( -path '*/.git' -o -path '*/target' -o -path '*/node_modules' \) -prune \
    -o -type f -name '*.go' -print0
)

if ((${#go_files[@]} == 0)); then
  exit 0
fi

unformatted="$(gofmt -l "${go_files[@]}")"
if [[ -n "$unformatted" ]]; then
  printf 'Go files are not gofmt-formatted:\n%s\n' "$unformatted" >&2
  exit 1
fi

standalone_go_files=()
for file in "${go_files[@]}"; do
  case "$file" in
    ./scripts/check-public-contracts/*)
      ;;
    *)
      standalone_go_files+=("$file")
      ;;
  esac
done

if ((${#standalone_go_files[@]} > 0)); then
  go vet "${standalone_go_files[@]}"
fi

if compgen -G './scripts/check-public-contracts/*.go' >/dev/null; then
  go vet ./scripts/check-public-contracts/*.go
fi
