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

target="${TOVUK_DOCS_PUBLIC_URL:-https://docs.tovuk.com}"
sync_wait_seconds="${TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS:-30}"

if ! [[ "$sync_wait_seconds" =~ ^[0-9]+$ ]]; then
  echo "TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS must be an unsigned integer." >&2
  exit 2
fi

echo "Mintlify GitHub App owns production docs sync for this repository."
echo "Checking local docs contracts before verifying public readiness at ${target}."
./scripts/check-public-contracts.sh docs
./scripts/check-prose-style.sh --self-test
./scripts/check-prose-style.sh

if [ "$sync_wait_seconds" -gt 0 ]; then
  echo "Waiting ${sync_wait_seconds}s for Mintlify GitHub App sync before public readiness check."
  sleep "$sync_wait_seconds"
fi

export TOVUK_DOCS_CHECK_RETRIES="${TOVUK_DOCS_CHECK_RETRIES:-12}"
export TOVUK_DOCS_CHECK_RETRY_DELAY_MS="${TOVUK_DOCS_CHECK_RETRY_DELAY_MS:-10000}"
./scripts/check-public-contracts.sh mintlify-agent-readiness "$target"
