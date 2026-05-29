#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

workflow_dir=".github/workflows"
if [ ! -d "$workflow_dir" ]; then
  printf 'missing %s\n' "$workflow_dir" >&2
  exit 1
fi

status=0

reject_match() {
  local pattern="$1"
  local message="$2"
  if rg -n --glob '*.yml' --glob '*.yaml' "$pattern" "$workflow_dir"; then
    printf '%s\n' "$message" >&2
    status=1
  fi
}

reject_match 'blacksmith-' \
  'Blacksmith runners are forbidden; use Tovuk trusted self-hosted runners or GitHub-hosted runners'
reject_match 'useblacksmith/(cache|setup-(go|node|python|ruby|java)|rust-cache)' \
  'Blacksmith cache forks are forbidden; use official cache-aware actions on GitHub-hosted runners'
reject_match 'actions/cache@(v[0-4]|main|master)' \
  'actions/cache must stay on the latest stable major'
reject_match 'pull_request_target:' \
  'pull_request_target is forbidden for this public repository'

for workflow in "$workflow_dir"/*.yml "$workflow_dir"/*.yaml; do
  [ -e "$workflow" ] || continue

  if ! rg -q '^permissions:' "$workflow"; then
    printf '%s: missing explicit permissions block\n' "$workflow" >&2
    status=1
  fi

  if ! rg -q '^concurrency:' "$workflow"; then
    printf '%s: missing explicit concurrency block\n' "$workflow" >&2
    status=1
  fi

  if rg -q 'actions/checkout@' "$workflow" && ! rg -q 'persist-credentials: false' "$workflow"; then
    printf '%s: checkout must set persist-credentials: false\n' "$workflow" >&2
    status=1
  fi

  if rg -q 'self-hosted' "$workflow"; then
    if ! rg -q 'public-trusted-ci' "$workflow"; then
      printf '%s: public self-hosted jobs must use the public-trusted-ci label\n' "$workflow" >&2
      status=1
    fi

    if ! rg -q "github.actor == 'kriptoburak'" "$workflow"; then
      printf '%s: public self-hosted jobs must require github.actor == kriptoburak\n' "$workflow" >&2
      status=1
    fi

    if ! rg -q "github.event.pull_request.head.repo.full_name == github.repository" "$workflow"; then
      printf '%s: public self-hosted pull_request jobs must require same-repository heads\n' "$workflow" >&2
      status=1
    fi

    if ! rg -q "github.event.pull_request.base.ref == 'main'" "$workflow"; then
      printf '%s: public self-hosted pull_request jobs must require base branch main\n' "$workflow" >&2
      status=1
    fi

    if ! rg -q "github.ref == 'refs/heads/main'" "$workflow"; then
      printf '%s: public self-hosted push and workflow_dispatch jobs must require refs/heads/main\n' "$workflow" >&2
      status=1
    fi
  fi

  if rg -q 'cargo (build|check|test|clippy|package|publish)' "$workflow" \
    && ! rg -q 'public-trusted-ci' "$workflow" \
    && ! rg -q 'actions/cache@v5' "$workflow"; then
    printf '%s: GitHub-hosted Rust jobs must use actions/cache@v5\n' "$workflow" >&2
    status=1
  fi
done

if ! rg -q 'public-trusted-ci' "$workflow_dir"; then
  printf 'no Tovuk public trusted self-hosted runner labels found in workflows\n' >&2
  status=1
fi

exit "$status"
