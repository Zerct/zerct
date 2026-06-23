#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"
python_bin="$(command -v python3.11 || command -v python3)"
native_cli="$repo_root/crates/tovuk/target/release/tovuk"
export TOVUK_NATIVE_BINARY="$native_cli"

if git check-ignore -q AGENTS.md; then
  printf 'AGENTS.md must be tracked Codex project guidance, not ignored.\n' >&2
  exit 1
fi
if ! git ls-files --error-unmatch AGENTS.md >/dev/null 2>&1; then
  printf 'AGENTS.md must be tracked so Codex project guidance travels with the repo.\n' >&2
  exit 1
fi

if rg -n 'npx[[:space:]]+tovuk' README.md docs packages crates skills Formula .github scripts --glob '!scripts/check-all.sh'; then
  printf 'Use native `tovuk` guidance instead of `npx tovuk`.\n' >&2
  exit 1
fi

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

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local label="$3"

  if ! grep -Fq -- "$needle" <<<"$haystack"; then
    printf 'expected %s to contain: %s\n' "$label" "$needle" >&2
    exit 1
  fi
}

assert_not_contains() {
  local haystack="$1"
  local needle="$2"
  local label="$3"

  if grep -Fq -- "$needle" <<<"$haystack"; then
    printf 'expected %s to omit: %s\n' "$label" "$needle" >&2
    exit 1
  fi
}

assert_unknown_command() {
  local label="$1"
  shift

  if "$@" >/tmp/tovuk-retired-command.out 2>/tmp/tovuk-retired-command.err; then
    printf 'expected retired command to fail: %s\n' "$label" >&2
    exit 1
  fi
  grep -q '"code": "unknown_command"' /tmp/tovuk-retired-command.err
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
./scripts/check-prose-style.sh --self-test
./scripts/check-prose-style.sh
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
for help_output in "$native_cli_default_output" "$native_cli_help_output" "$native_cli_flag_help_output"; do
  assert_contains "$help_output" 'tovuk scraper list' 'native CLI help'
  assert_contains "$help_output" 'tovuk scraper health' 'native CLI help'
  assert_contains "$help_output" 'tovuk request create' 'native CLI help'
  assert_contains "$help_output" 'tovuk request results' 'native CLI help'
  assert_contains "$help_output" 'tovuk pricing' 'native CLI help'
  assert_contains "$help_output" 'tovuk usage' 'native CLI help'
  assert_contains "$help_output" 'tovuk billing checkout' 'native CLI help'
  assert_contains "$help_output" 'tovuk support create' 'native CLI help'
  assert_contains "$help_output" 'tovuk abuse list --operator' 'native CLI help'
  assert_not_contains "$help_output" 'tovuk deploy' 'native CLI help'
  assert_not_contains "$help_output" 'tovuk service' 'native CLI help'
  assert_not_contains "$help_output" 'tovuk storage' 'native CLI help'
  assert_not_contains "$help_output" 'tovuk sqlite' 'native CLI help'
  assert_not_contains "$help_output" 'tovuk queue' 'native CLI help'
done
test "$("$native_cli" -V)" = "$native_cli_version"
test "$("$native_cli" --api=https://api.example.test --version)" = "$native_cli_version"
if "$native_cli" --json --definitely-unknown >/tmp/tovuk-unknown-flag.out 2>/tmp/tovuk-unknown-flag.err; then
  printf 'expected native CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' /tmp/tovuk-unknown-flag.err

for retired_command in \
  new check dev deploy service logs sqlite kv queue cron state binding limits env secrets domains storage nodes \
  init install preview capabilities me activity services overview deploys builds status inspect platform caps limit files media; do
  assert_unknown_command "$retired_command" "$native_cli" "$retired_command" --json
done

"$python_bin" -m compileall -q packages/tovuk-py/src
PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --version
python_cli_default_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk)"
python_cli_help_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk help)"
python_cli_flag_help_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --help)"
for help_output in "$python_cli_default_output" "$python_cli_help_output" "$python_cli_flag_help_output"; do
  assert_contains "$help_output" 'tovuk scraper list' 'Python CLI help'
  assert_contains "$help_output" 'tovuk request create' 'Python CLI help'
  assert_contains "$help_output" 'tovuk request results' 'Python CLI help'
  assert_contains "$help_output" 'tovuk pricing' 'Python CLI help'
  assert_contains "$help_output" 'tovuk billing checkout' 'Python CLI help'
  assert_contains "$help_output" 'tovuk support create' 'Python CLI help'
  assert_not_contains "$help_output" 'tovuk deploy' 'Python CLI help'
  assert_not_contains "$help_output" 'tovuk service' 'Python CLI help'
  assert_not_contains "$help_output" 'tovuk storage' 'Python CLI help'
done
test "$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --api=https://api.example.test --version)" = "$native_cli_version"
if PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --json --definitely-unknown >/tmp/tovuk-unknown-flag.out 2>/tmp/tovuk-unknown-flag.err; then
  printf 'expected Python CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' /tmp/tovuk-unknown-flag.err
for retired_command in new check dev deploy service logs sqlite kv queue cron state binding limits env secrets domains storage nodes; do
  assert_unknown_command "$retired_command" env PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk "$retired_command" --json
done
