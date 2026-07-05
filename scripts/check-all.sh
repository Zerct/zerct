#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$script_dir/lib/repo-root.sh"
# shellcheck source=scripts/lib/tool-path.sh
. "$script_dir/lib/tool-path.sh"
repo_root="$(tovuk_repo_root "$script_dir")"
cd "$repo_root"
tovuk_prepend_tool_path

python_bin="$(command -v python3.11 || command -v python3)"
native_cli="$repo_root/crates/tovuk/target/release/tovuk"
export TOVUK_NATIVE_BINARY="$native_cli"

scripts/sync-native-release-targets.sh
scripts/sync-native-release-targets.sh --check
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- repo-hygiene

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
cargo fmt --check --manifest-path checks/Cargo.toml
cargo check --locked --release --all-targets --all-features --manifest-path crates/tovuk/Cargo.toml
cargo check --locked --release --all-targets --all-features --manifest-path checks/Cargo.toml
cargo test --locked --release --all-targets --all-features --manifest-path crates/tovuk/Cargo.toml
cargo test --locked --release --all-targets --all-features --manifest-path checks/Cargo.toml
cargo clippy --manifest-path crates/tovuk/Cargo.toml "${strict_clippy_args[@]}"
cargo clippy --manifest-path checks/Cargo.toml "${strict_clippy_args[@]}"
cargo build --locked --release --manifest-path crates/tovuk/Cargo.toml
cargo package --locked --manifest-path crates/tovuk/Cargo.toml --allow-dirty >/dev/null
(cd crates/tovuk && cargo machete)
(cd checks && cargo machete)
mkdir -p target
cargo metadata --locked --manifest-path crates/tovuk/Cargo.toml --all-features --format-version 1 >target/tovuk-cargo-deny-metadata.json
cargo deny --manifest-path crates/tovuk/Cargo.toml check --config deny.toml --metadata-path target/tovuk-cargo-deny-metadata.json all
cargo metadata --locked --manifest-path checks/Cargo.toml --all-features --format-version 1 >target/tovuk-public-checks-cargo-deny-metadata.json
cargo deny --manifest-path checks/Cargo.toml check --config deny.toml --metadata-path target/tovuk-public-checks-cargo-deny-metadata.json all

npm --prefix packages/tovuk run check
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- package-versions
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- cli-contract
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- docs
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-prose-style -- --self-test
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-prose-style --
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-github-actions --
scripts/check-shell-style.sh
scripts/check-toml-style.sh
typos --config .typos.toml .
scripts/check-openapi.sh
ruby -c Formula/tovuk.rb >/dev/null
if command -v brew >/dev/null 2>&1; then
  brew style Formula/tovuk.rb
fi

cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- runtime-cli "$native_cli" "$python_bin"
