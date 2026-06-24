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

scripts/check-public-contracts.sh runtime-cli "$native_cli" "$python_bin"
