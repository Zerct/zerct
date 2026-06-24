# shellcheck shell=bash
# Resolve the public repository root for Git checkouts and exported snapshots.

tovuk_repo_root() {
  local script_dir="$1"
  local git_root

  if git_root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null)"; then
    printf '%s\n' "$git_root"
    return
  fi

  cd -- "$script_dir/.." && pwd
}
