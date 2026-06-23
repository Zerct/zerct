#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

version="${VACUUM_VERSION:-0.26.6}"
install_dir="${TOVUK_VACUUM_DIR:-$repo_root/target/tools/vacuum-$version}"
vacuum_bin="$install_dir/vacuum"

if [[ -x "$vacuum_bin" ]] && [[ "$("$vacuum_bin" version)" = "$version" ]]; then
  printf '%s\n' "$vacuum_bin"
  exit 0
fi

case "$(uname -s)" in
  Darwin) os="darwin" ;;
  Linux) os="linux" ;;
  *)
    echo "unsupported vacuum host OS: $(uname -s)" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  arm64 | aarch64) arch="arm64" ;;
  x86_64 | amd64) arch="x86_64" ;;
  i386 | i686) arch="i386" ;;
  *)
    echo "unsupported vacuum host architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

asset="vacuum_${version}_${os}_${arch}.tar.gz"
url="https://github.com/daveshanley/vacuum/releases/download/v${version}/${asset}"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

mkdir -p "$install_dir"
curl -fsSL "$url" -o "$tmp_dir/$asset"
tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"

candidate="$(find "$tmp_dir" -type f -name vacuum -perm -u+x | head -n 1)"
if [[ -z "$candidate" ]]; then
  echo "downloaded $asset did not contain an executable vacuum binary" >&2
  exit 1
fi

install -m 0755 "$candidate" "$vacuum_bin"
installed_vacuum_version="$("$vacuum_bin" version)"
if [[ "$installed_vacuum_version" != "$version" ]]; then
  echo "installed vacuum version mismatch: expected $version, got $installed_vacuum_version" >&2
  exit 1
fi

printf '%s\n' "$vacuum_bin"
