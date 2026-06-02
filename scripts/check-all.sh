#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"
python_bin="$(command -v python3.11 || command -v python3)"
native_cli="$repo_root/crates/tovuk/target/release/tovuk"
export TOVUK_NATIVE_BINARY="$native_cli"
if rg -n 'npx[[:space:]]+tovuk' README.md docs packages crates skills Formula .github scripts --glob '!scripts/check-all.sh'; then
  printf 'Use native `tovuk` guidance instead of `tovuk`.\n' >&2
  exit 1
fi
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

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local label="$3"

  if ! grep -Fq -- "$needle" <<<"$haystack"; then
    printf 'expected %s to contain: %s\n' "$label" "$needle" >&2
    exit 1
  fi
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
go run scripts/check-public-contracts/*.go package-versions
go run scripts/check-public-contracts/*.go cli-contract
go run scripts/check-public-contracts/*.go docs
go run scripts/check-prose-style.go --self-test
go run scripts/check-prose-style.go
scripts/check-github-actions.sh
scripts/check-shell-style.sh
scripts/check-toml-style.sh
scripts/check-go-style.sh
scripts/check-typos.sh
scripts/check-openapi.sh
ruby -c Formula/tovuk.rb >/dev/null
if command -v brew >/dev/null 2>&1; then
  brew style Formula/tovuk.rb
fi

native_cli_version="$("$native_cli" --version)"
printf '%s\n' "$native_cli_version"
native_cli_default_output="$("$native_cli")"
native_cli_help_output="$("$native_cli" help)"
native_cli_flag_help_output="$("$native_cli" --help)"
assert_contains "$native_cli_default_output" 'tovuk deploy --dry-run' 'native CLI default help'
assert_contains "$native_cli_help_output" 'tovuk deploy --dry-run' 'native CLI help command'
assert_contains "$native_cli_flag_help_output" 'tovuk support create' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk support resolve' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk abuse report' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk abuse list --operator' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk abuse appeal' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk abuse quarantine' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk abuse release' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk storage upload' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk storage download' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk deploy --dry-run' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk pricing' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk service show' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk limits set' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" '--notify-at-percent' 'native CLI flag help'
assert_contains "$native_cli_flag_help_output" 'tovuk billing checkout' 'native CLI flag help'
test "$("$native_cli" -V)" = "$native_cli_version"
test "$("$native_cli" --api=https://api.example.test --wait-timeout=9 --version)" = "$native_cli_version"
if "$native_cli" --json --definitely-unknown >/tmp/tovuk-unknown-flag.out 2>/tmp/tovuk-unknown-flag.err; then
  printf 'expected native CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' /tmp/tovuk-unknown-flag.err
if "$native_cli" plan --json >/tmp/tovuk-retired-plan.out 2>/tmp/tovuk-retired-plan.err; then
  printf 'expected retired native CLI plan command to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_command"' /tmp/tovuk-retired-plan.err
for retired_command in \
  init install preview capabilities me activity services overview deploys builds status inspect platform caps limit files media queues bindings; do
  if "$native_cli" "$retired_command" --json >/tmp/tovuk-retired-command.out 2>/tmp/tovuk-retired-command.err; then
    printf 'expected retired native CLI command to fail: %s\n' "$retired_command" >&2
    exit 1
  fi
  grep -q '"code": "unknown_command"' /tmp/tovuk-retired-command.err
done
for retired_service_command in status resources deploys builds inspect; do
  if "$native_cli" service "$retired_service_command" service_1 --json >/tmp/tovuk-retired-service-command.out 2>/tmp/tovuk-retired-service-command.err; then
    printf 'expected retired native CLI service command to fail: %s\n' "$retired_service_command" >&2
    exit 1
  fi
  grep -q '"code": "unknown_command"' /tmp/tovuk-retired-service-command.err
done
retired_alias_cases=(
  "service del service_1"
  "service rm service_1"
  "storage put"
  "storage get"
  "storage rm"
  "database"
  "database execute"
  "database backups"
  "database rm"
  "kv bulk-get"
  "kv bulk-put"
  "kv bulk-delete"
  "kv bulk-del"
  "kv rm"
  "kv namespaces"
  "kv delete-namespace"
  "queue set"
  "queue batch-send"
  "queue rm"
  "cron set"
  "cron rm"
  "state instances"
  "state set"
  "state delete-state"
  "state rm"
  "state alarm show"
  "state alarm rm"
  "binding rm"
  "limits rm"
)
for retired_alias in "${retired_alias_cases[@]}"; do
  read -r -a retired_alias_args <<<"$retired_alias"
  if "$native_cli" "${retired_alias_args[@]}" --json >/tmp/tovuk-retired-alias.out 2>/tmp/tovuk-retired-alias.err; then
    printf 'expected retired native CLI alias to fail: %s\n' "$retired_alias" >&2
    exit 1
  fi
  grep -q '"code": "unknown' /tmp/tovuk-retired-alias.err
done

"$python_bin" -m compileall -q packages/tovuk-py/src
PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --version
python_cli_default_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk)"
python_cli_help_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk help)"
python_cli_flag_help_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --help)"
assert_contains "$python_cli_default_output" 'tovuk deploy --dry-run' 'Python CLI default help'
assert_contains "$python_cli_help_output" 'tovuk deploy --dry-run' 'Python CLI help command'
assert_contains "$python_cli_flag_help_output" 'tovuk support create' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk support resolve' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk abuse report' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk abuse list --operator' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk abuse appeal' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk abuse quarantine' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk abuse release' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk storage upload' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk storage download' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk deploy --dry-run' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk pricing' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk service show' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk limits set' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" '--notify-at-percent' 'Python CLI flag help'
assert_contains "$python_cli_flag_help_output" 'tovuk billing checkout' 'Python CLI flag help'
test "$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --api=https://api.example.test --wait-timeout=9 --version)" = "$native_cli_version"
if PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --json --definitely-unknown >/tmp/tovuk-unknown-flag.out 2>/tmp/tovuk-unknown-flag.err; then
  printf 'expected Python CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' /tmp/tovuk-unknown-flag.err
for retired_command in \
  init install preview capabilities me activity services overview deploys builds status inspect platform caps limit files media queues bindings; do
  if PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk "$retired_command" --json >/tmp/tovuk-retired-command.out 2>/tmp/tovuk-retired-command.err; then
    printf 'expected retired Python CLI command to fail: %s\n' "$retired_command" >&2
    exit 1
  fi
  grep -q '"code": "unknown_command"' /tmp/tovuk-retired-command.err
done
for retired_service_command in status resources deploys builds inspect; do
  if PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk service "$retired_service_command" service_1 --json >/tmp/tovuk-retired-service-command.out 2>/tmp/tovuk-retired-service-command.err; then
    printf 'expected retired Python CLI service command to fail: %s\n' "$retired_service_command" >&2
    exit 1
  fi
  grep -q '"code": "unknown_command"' /tmp/tovuk-retired-service-command.err
done
for retired_alias in "${retired_alias_cases[@]}"; do
  read -r -a retired_alias_args <<<"$retired_alias"
  if PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk "${retired_alias_args[@]}" --json >/tmp/tovuk-retired-alias.out 2>/tmp/tovuk-retired-alias.err; then
    printf 'expected retired Python CLI alias to fail: %s\n' "$retired_alias" >&2
    exit 1
  fi
  grep -q '"code": "unknown' /tmp/tovuk-retired-alias.err
done

test -f examples/hello-rust/Cargo.lock
cargo check --locked --release --all-targets --all-features --manifest-path examples/hello-rust/Cargo.toml
cargo test --locked --release --all-targets --all-features --manifest-path examples/hello-rust/Cargo.toml
cargo clippy --manifest-path examples/hello-rust/Cargo.toml "${strict_clippy_args[@]}"
"$native_cli" check examples/hello-rust --json >/dev/null

rust_policy_fixture="$(mktemp -d)"
js_worker_fixture="$(mktemp -d)"
plain_static_fixture="$(mktemp -d)"
fullstack_fixture="$(mktemp -d)"
trap 'rm -rf "$rust_policy_fixture" "$js_worker_fixture" "$plain_static_fixture" "$fullstack_fixture"' EXIT

cat >"$rust_policy_fixture/tovuk.toml" <<'EOF'
name = "missing-lints"
kind = "rust_worker"

[capabilities]
static_frontend = false
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

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
  "$native_cli check $rust_policy_fixture --json" \
  "PYTHONPATH=packages/tovuk-py/src $python_bin -m tovuk check $rust_policy_fixture --json"; do
  if eval "$command" >/tmp/tovuk-policy-check.json 2>/tmp/tovuk-policy-check.err; then
    printf 'expected Rust policy fixture to fail: %s\n' "$command" >&2
    exit 1
  fi
  grep -q 'cargo lints' /tmp/tovuk-policy-check.json
done

cat >"$js_worker_fixture/tovuk.toml" <<'EOF'
name = "node-worker"
kind = "rust_worker"

[capabilities]
static_frontend = false
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[run]
command = "node server.js"
EOF
cat >"$js_worker_fixture/Cargo.toml" <<'EOF'
[package]
name = "node-worker"
version = "0.1.0"
edition = "2024"

[lints.rust]
unsafe_code = "forbid"
warnings = "deny"
EOF
write_strict_clippy_lints "$js_worker_fixture/Cargo.toml"
cat >"$js_worker_fixture/Cargo.lock" <<'EOF'
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "node-worker"
version = "0.1.0"
EOF
mkdir -p "$js_worker_fixture/src"
printf 'fn main() {}\n' >"$js_worker_fixture/src/main.rs"
printf 'import http from "node:http"\n' >"$js_worker_fixture/src/server.ts"

for command in \
  "$native_cli check $js_worker_fixture --json" \
  "PYTHONPATH=packages/tovuk-py/src $python_bin -m tovuk check $js_worker_fixture --json"; do
  if eval "$command" >/tmp/tovuk-policy-check.json 2>/tmp/tovuk-policy-check.err; then
    printf 'expected JS worker fixture to fail: %s\n' "$command" >&2
    exit 1
  fi
  grep -q 'runtime commands cannot invoke JavaScript or TypeScript runtimes' /tmp/tovuk-policy-check.json
done

cat >"$plain_static_fixture/tovuk.toml" <<'EOF'
name = "plain-static"
kind = "static_frontend"

[capabilities]
static_frontend = true
worker = false
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

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
  "$native_cli check $plain_static_fixture --json" \
  "PYTHONPATH=packages/tovuk-py/src $python_bin -m tovuk check $plain_static_fixture --json"; do
  eval "$command" >/tmp/tovuk-policy-check.json
  grep -q '"ok": true' /tmp/tovuk-policy-check.json
done

cat >"$fullstack_fixture/tovuk.toml" <<EOF
name = "full-stack-ok"
kind = "fullstack"

[capabilities]
static_frontend = true
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[worker]
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
<h1>full-stack</h1>
EOF

"$native_cli" check "$fullstack_fixture" --json >/tmp/tovuk-policy-check.json
grep -q '"ok": true' /tmp/tovuk-policy-check.json

printf 'all checks passed\n'
