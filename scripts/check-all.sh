#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"
python_bin="$(command -v python3.11 || command -v python3)"
native_cli="$repo_root/crates/tovuk/target/release/tovuk"
export TOVUK_NATIVE_BINARY="$native_cli"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/tovuk-public-check.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

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

tracked_go_files="$(
  while IFS= read -r path; do
    if [[ -e "$path" ]]; then
      printf '%s\n' "$path"
    fi
  done < <(git ls-files '*.go')
)"
if [[ -n "$tracked_go_files" ]]; then
  printf 'Tracked Go source is not allowed in the public repo; use Rust-native checks:\n%s\n' "$tracked_go_files" >&2
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
  local out_file="$tmp_dir/retired-command.out"
  local err_file="$tmp_dir/retired-command.err"

  if "$@" >"$out_file" 2>"$err_file"; then
    printf 'expected retired command to fail: %s\n' "$label" >&2
    exit 1
  fi
  grep -q '"code": "unknown_command"' "$err_file"
}

required_help_commands=(
  'tovuk scraper list'
  'tovuk scraper health'
  'tovuk request create'
  'tovuk request results'
  'tovuk pricing'
  'tovuk usage'
  'tovuk billing checkout'
  'tovuk support create'
  'tovuk abuse list --operator'
)
retired_help_commands=(
  'tovuk deploy'
  'tovuk service'
  'tovuk storage'
  'tovuk sqlite'
  'tovuk queue'
)
retired_commands=(
  new
  check
  dev
  deploy
  service
  logs
  sqlite
  kv
  queue
  cron
  state
  binding
  limits
  env
  secrets
  domains
  storage
  nodes
  init
  install
  preview
  capabilities
  me
  activity
  services
  overview
  deploys
  builds
  status
  inspect
  platform
  caps
  limit
  files
  media
)

assert_help_contract() {
  local label="$1"
  shift
  local help_output required_command retired_command

  for help_output in "$@"; do
    for required_command in "${required_help_commands[@]}"; do
      assert_contains "$help_output" "$required_command" "$label"
    done
    for retired_command in "${retired_help_commands[@]}"; do
      assert_not_contains "$help_output" "$retired_command" "$label"
    done
  done
}

assert_retired_commands() {
  local label="$1"
  shift
  local retired_command

  for retired_command in "${retired_commands[@]}"; do
    assert_unknown_command "$label $retired_command" "$@" "$retired_command" --json
  done
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
scripts/check-public-contracts.sh package-versions
scripts/check-public-contracts.sh cli-contract
scripts/check-public-contracts.sh docs
./scripts/check-prose-style.sh --self-test
./scripts/check-prose-style.sh
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
native_cli_default_output="$("$native_cli")"
native_cli_help_output="$("$native_cli" help)"
native_cli_flag_help_output="$("$native_cli" --help)"
assert_help_contract \
  'native CLI help' \
  "$native_cli_default_output" \
  "$native_cli_help_output" \
  "$native_cli_flag_help_output"
test "$("$native_cli" -V)" = "$native_cli_version"
test "$("$native_cli" --api=https://api.example.test --version)" = "$native_cli_version"
if "$native_cli" --json --definitely-unknown >"$tmp_dir/native-unknown-flag.out" 2>"$tmp_dir/native-unknown-flag.err"; then
  printf 'expected native CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' "$tmp_dir/native-unknown-flag.err"

assert_retired_commands 'native CLI retired command' "$native_cli"

"$python_bin" -m compileall -q packages/tovuk-py/src
PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --version
python_cli_default_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk)"
python_cli_help_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk help)"
python_cli_flag_help_output="$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --help)"
assert_help_contract \
  'Python CLI help' \
  "$python_cli_default_output" \
  "$python_cli_help_output" \
  "$python_cli_flag_help_output"
test "$(PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --api=https://api.example.test --version)" = "$native_cli_version"
if PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk --json --definitely-unknown >"$tmp_dir/python-unknown-flag.out" 2>"$tmp_dir/python-unknown-flag.err"; then
  printf 'expected Python CLI unknown flag to fail\n' >&2
  exit 1
fi
grep -q '"code": "unknown_argument"' "$tmp_dir/python-unknown-flag.err"
assert_retired_commands 'Python CLI retired command' env PYTHONPATH=packages/tovuk-py/src "$python_bin" -m tovuk
