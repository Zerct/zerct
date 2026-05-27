#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"
python_bin="$(command -v python3.11 || command -v python3)"

node --check packages/zerct/bin/zerct.js
node scripts/check-package-versions.mjs
node scripts/check-docs.mjs
node scripts/check-prose-style.mjs
scripts/check-openapi.sh
node packages/zerct/bin/zerct.js --version
(cd packages/zerct && npm pack --dry-run >/dev/null)

"$python_bin" -m compileall -q packages/zerct-py/src
PYTHONPATH=packages/zerct-py/src "$python_bin" -m zerct --version

cargo fmt --check --manifest-path crates/zerct/Cargo.toml
cargo check --locked --manifest-path crates/zerct/Cargo.toml
cargo clippy --locked --manifest-path crates/zerct/Cargo.toml --all-targets --all-features -- -D warnings
cargo package --locked --manifest-path crates/zerct/Cargo.toml --allow-dirty --no-verify >/dev/null

test -f examples/hello-rust/Cargo.lock
cargo check --locked --manifest-path examples/hello-rust/Cargo.toml
cargo clippy --locked --manifest-path examples/hello-rust/Cargo.toml --all-targets --all-features -- -D warnings
node packages/zerct/bin/zerct.js doctor examples/hello-rust --json >/dev/null

policy_fixture="$(mktemp -d)"
trap 'rm -rf "$policy_fixture"' EXIT
cat >"$policy_fixture/zerct.toml" <<'EOF'
name = "strict-web"
kind = "static_frontend"

[build]
check = "bun ci && bun run typecheck && bun run lint"
EOF
cat >"$policy_fixture/package.json" <<'EOF'
{
  "scripts": {
    "build": "vite build",
    "typecheck": "tsgo --noEmit",
    "lint": "oxlint src && prettier --check src"
  }
}
EOF
touch "$policy_fixture/bun.lock"
mkdir -p "$policy_fixture/src"
printf 'export const ok = true\n' >"$policy_fixture/src/main.ts"

for command in \
  "node packages/zerct/bin/zerct.js doctor $policy_fixture --json" \
  "PYTHONPATH=packages/zerct-py/src $python_bin -m zerct doctor $policy_fixture --json" \
  "cargo run --quiet --manifest-path crates/zerct/Cargo.toml -- doctor $policy_fixture --json"; do
  if eval "$command" >/tmp/zerct-policy-check.json 2>/tmp/zerct-policy-check.err; then
    printf 'expected policy fixture to fail: %s\n' "$command" >&2
    exit 1
  fi
  grep -q 'native frontend lint' /tmp/zerct-policy-check.json
done
