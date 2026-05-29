#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"
python_bin="$(command -v python3.11 || command -v python3)"
native_cli="$repo_root/crates/tovuk/target/release/tovuk"
export TOVUK_NATIVE_BINARY="$native_cli"
strict_rust_check="cargo fmt --all --check && cargo check --locked --release --all-targets --all-features && cargo test --locked --release --all-targets --all-features && cargo clippy --locked --release --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::dbg_macro -D clippy::todo -D clippy::unimplemented -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used -D clippy::large_futures -D clippy::large_include_file -D clippy::large_stack_frames -D clippy::mem_forget -D clippy::rc_buffer -D clippy::rc_mutex -D clippy::redundant_clone -D clippy::clone_on_ref_ptr"
strict_clippy_args=(
  --locked
  --release
  --all-targets
  --all-features
  --
  -D warnings
  -D clippy::all
  -D clippy::pedantic
  -D clippy::dbg_macro
  -D clippy::todo
  -D clippy::unimplemented
  -D clippy::panic
  -D clippy::unwrap_used
  -D clippy::expect_used
  -D clippy::large_futures
  -D clippy::large_include_file
  -D clippy::large_stack_frames
  -D clippy::mem_forget
  -D clippy::rc_buffer
  -D clippy::rc_mutex
  -D clippy::redundant_clone
  -D clippy::clone_on_ref_ptr
)

write_strict_clippy_lints() {
  cat >>"$1" <<'EOF'

[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
dbg_macro = "deny"
todo = "deny"
unimplemented = "deny"
panic = "deny"
unwrap_used = "deny"
expect_used = "deny"
large_futures = "deny"
large_include_file = "deny"
large_stack_frames = "deny"
mem_forget = "deny"
rc_buffer = "deny"
rc_mutex = "deny"
redundant_clone = "deny"
clone_on_ref_ptr = "deny"
EOF
}

cargo fmt --check --manifest-path crates/tovuk/Cargo.toml
cargo check --locked --release --all-targets --all-features --manifest-path crates/tovuk/Cargo.toml
cargo test --locked --release --all-targets --all-features --manifest-path crates/tovuk/Cargo.toml
cargo clippy --manifest-path crates/tovuk/Cargo.toml "${strict_clippy_args[@]}"
cargo build --locked --release --manifest-path crates/tovuk/Cargo.toml
cargo package --locked --manifest-path crates/tovuk/Cargo.toml --allow-dirty --no-verify >/dev/null
(cd crates/tovuk && cargo machete)
mkdir -p target
cargo metadata --locked --manifest-path crates/tovuk/Cargo.toml --all-features --format-version 1 >target/tovuk-cargo-deny-metadata.json
cargo deny --manifest-path crates/tovuk/Cargo.toml check --config deny.toml --metadata-path target/tovuk-cargo-deny-metadata.json all

npm --prefix packages/tovuk run check
node scripts/check-package-versions.mjs
node scripts/check-cli-contract.mjs
node scripts/check-docs.mjs
node scripts/check-prose-style.mjs
scripts/check-github-actions.sh
scripts/check-shell-style.sh
scripts/check-toml-style.sh
scripts/check-typos.sh
scripts/check-openapi.sh
ruby -c Formula/tovuk.rb >/dev/null
if command -v brew >/dev/null 2>&1; then
  brew style Formula/tovuk.rb
fi

native_cli_version="$("$native_cli" --version)"
printf '%s\n' "$native_cli_version"
"$native_cli" --help | grep -q 'tovuk support create'
"$native_cli" --help | grep -q 'tovuk support resolve'
"$native_cli" --help | grep -q 'tovuk billing checkout'
test "$("$native_cli" -V)" = "$native_cli_version"
test "$("$native_cli" --api=https://api.example.test --wait-timeout=9 --version)" = "$native_cli_version"
if "$native_cli" --json --definitely-unknown >/tmp/tovuk-unknown-flag.out 2>/tmp/tovuk-unknown-flag.err; then
  printf 'expected native CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' /tmp/tovuk-unknown-flag.err

"$python_bin" -m compileall -q packages/tovuk-py/src
PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --version
PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --help | grep -q 'tovuk support create'
PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --help | grep -q 'tovuk support resolve'
PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --help | grep -q 'tovuk billing checkout'
test "$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --api=https://api.example.test --wait-timeout=9 --version)" = "$native_cli_version"
if PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --json --definitely-unknown >/tmp/tovuk-unknown-flag.out 2>/tmp/tovuk-unknown-flag.err; then
  printf 'expected Python CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' /tmp/tovuk-unknown-flag.err

test -f examples/hello-rust/Cargo.lock
cargo check --locked --release --all-targets --all-features --manifest-path examples/hello-rust/Cargo.toml
cargo test --locked --release --all-targets --all-features --manifest-path examples/hello-rust/Cargo.toml
cargo clippy --manifest-path examples/hello-rust/Cargo.toml "${strict_clippy_args[@]}"
"$native_cli" doctor examples/hello-rust --json >/dev/null

rust_policy_fixture="$(mktemp -d)"
js_backend_fixture="$(mktemp -d)"
plain_static_fixture="$(mktemp -d)"
fullstack_fixture="$(mktemp -d)"
trap 'rm -rf "$rust_policy_fixture" "$js_backend_fixture" "$plain_static_fixture" "$fullstack_fixture"' EXIT

cat >"$rust_policy_fixture/tovuk.toml" <<'EOF'
name = "missing-lints"

[run]
command = "./target/release/missing-lints"
EOF
cat >"$rust_policy_fixture/Cargo.toml" <<'EOF'
[package]
name = "missing-lints"
version = "0.1.0"
edition = "2024"
EOF
cat >"$rust_policy_fixture/Cargo.lock" <<'EOF'
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "missing-lints"
version = "0.1.0"
EOF
mkdir -p "$rust_policy_fixture/src"
printf 'fn main() {}\n' >"$rust_policy_fixture/src/main.rs"

for command in \
  "$native_cli doctor $rust_policy_fixture --json" \
  "PYTHONPATH=packages/tovuk-py/src $python_bin -m tovuk doctor $rust_policy_fixture --json"; do
  if eval "$command" >/tmp/tovuk-policy-check.json 2>/tmp/tovuk-policy-check.err; then
    printf 'expected Rust policy fixture to fail: %s\n' "$command" >&2
    exit 1
  fi
  grep -q 'cargo lints' /tmp/tovuk-policy-check.json
done

cat >"$js_backend_fixture/tovuk.toml" <<'EOF'
name = "node-backend"

[run]
command = "node server.js"
EOF
cat >"$js_backend_fixture/Cargo.toml" <<'EOF'
[package]
name = "node-backend"
version = "0.1.0"
edition = "2024"

[lints.rust]
unsafe_code = "forbid"
warnings = "deny"
EOF
write_strict_clippy_lints "$js_backend_fixture/Cargo.toml"
cat >"$js_backend_fixture/Cargo.lock" <<'EOF'
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "node-backend"
version = "0.1.0"
EOF
mkdir -p "$js_backend_fixture/src"
printf 'fn main() {}\n' >"$js_backend_fixture/src/main.rs"
printf 'import http from "node:http"\n' >"$js_backend_fixture/src/server.ts"

for command in \
  "$native_cli doctor $js_backend_fixture --json" \
  "PYTHONPATH=packages/tovuk-py/src $python_bin -m tovuk doctor $js_backend_fixture --json"; do
  if eval "$command" >/tmp/tovuk-policy-check.json 2>/tmp/tovuk-policy-check.err; then
    printf 'expected JS backend fixture to fail: %s\n' "$command" >&2
    exit 1
  fi
  grep -q 'runtime commands cannot invoke JavaScript or TypeScript runtimes' /tmp/tovuk-policy-check.json
done

cat >"$plain_static_fixture/tovuk.toml" <<'EOF'
name = "plain-static"
kind = "static_frontend"

[build]
check = ":"
command = ":"
output = "."
EOF
cat >"$plain_static_fixture/index.html" <<'EOF'
<!doctype html>
<h1>plain static</h1>
EOF

for command in \
  "$native_cli doctor $plain_static_fixture --json" \
  "PYTHONPATH=packages/tovuk-py/src $python_bin -m tovuk doctor $plain_static_fixture --json"; do
  eval "$command" >/tmp/tovuk-policy-check.json
  grep -q '"ok": true' /tmp/tovuk-policy-check.json
done

cat >"$fullstack_fixture/tovuk.toml" <<EOF
name = "fullstack-ok"
kind = "fullstack"

[backend]
root = "api"
check = "$strict_rust_check"
build = "cargo build --release"
command = "./target/release/api"
port = 3000
health = "/api/healthz"

[frontend]
root = "web"
check = ":"
build = ":"
output = "."
EOF
mkdir -p "$fullstack_fixture/api/src" "$fullstack_fixture/web"
cat >"$fullstack_fixture/api/Cargo.toml" <<'EOF'
[package]
name = "api"
version = "0.1.0"
edition = "2024"

[lints.rust]
unsafe_code = "forbid"
warnings = "deny"
EOF
write_strict_clippy_lints "$fullstack_fixture/api/Cargo.toml"
cat >"$fullstack_fixture/api/Cargo.lock" <<'EOF'
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "api"
version = "0.1.0"
EOF
printf 'fn main() {}\n' >"$fullstack_fixture/api/src/main.rs"
cat >"$fullstack_fixture/web/index.html" <<'EOF'
<!doctype html>
<h1>fullstack static</h1>
EOF

"$native_cli" doctor "$fullstack_fixture" --json >/tmp/tovuk-policy-check.json
grep -q '"ok": true' /tmp/tovuk-policy-check.json

printf 'all checks passed\n'
