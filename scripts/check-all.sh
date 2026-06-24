#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"
python_bin="$(command -v python3.11 || command -v python3)"
native_cli="$repo_root/crates/tovuk/target/release/tovuk"
export TOVUK_NATIVE_BINARY="$native_cli"

scripts/check-public-contracts.sh repo-hygiene

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
