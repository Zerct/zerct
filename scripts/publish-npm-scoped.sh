#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

source_dir="packages/zerct"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/zerct-npm-scoped.XXXXXX")"

cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

cp "$source_dir/README.md" "$work_dir/README.md"
cp "$source_dir/package.json" "$work_dir/package.json"
cp "$source_dir/tsconfig.json" "$work_dir/tsconfig.json"
cp -R "$source_dir/src" "$work_dir/src"

node --input-type=module - "$work_dir/package.json" <<'NODE'
import { readFileSync, writeFileSync } from 'node:fs'

const path = process.argv[2]
const pkg = JSON.parse(readFileSync(path, 'utf8'))
pkg.name = '@zerct/zerct'
pkg.publishConfig = { access: 'public' }
writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`)
NODE

chmod +x "$work_dir/src/zerct.ts"

(
  cd "$work_dir"
  publish_args=(--access public --registry=https://registry.npmjs.org/)
  if [[ "${NPM_PUBLISH_PROVENANCE:-}" == "1" ]]; then
    publish_args+=(--provenance)
  fi
  npm publish "${publish_args[@]}"
)
