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

version="${VACUUM_VERSION:-0.26.6}"
install_dir="${TOVUK_VACUUM_DIR:-$repo_root/target/tools/vacuum-$version}"
vacuum_bin="$install_dir/vacuum"

vacuum_asset_sha256() {
  case "$1:$2:$3" in
    0.26.6:darwin:arm64) printf '%s\n' "36e540617b960dc822eec1f65b5e8e6b5a10107c7bca27bf09d8c9afec6fdde2" ;;
    0.26.6:darwin:x86_64) printf '%s\n' "839c66424af0bfc4357ddea7b46e9c4830923bb7ac95597163df358b7f33425a" ;;
    0.26.6:linux:arm64) printf '%s\n' "2d57aa941495f970e6093a2b557ce919b02659fc913d13a6a7a8e2deea594b0b" ;;
    0.26.6:linux:x86_64) printf '%s\n' "e81288a3d1f6eb03431b6f8e817b9a8071d2ee800eb0ada3213e4f00805e00e6" ;;
    0.26.6:linux:i386) printf '%s\n' "76b90ed6b5bbef1fa1c4adc2d2ccfa8716cfe1df9fd8480573424653f0c42800" ;;
    *)
      echo "unsupported vacuum asset checksum for version=$1 os=$2 arch=$3" >&2
      exit 1
      ;;
  esac
}

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
expected_sha256="$(vacuum_asset_sha256 "$version" "$os" "$arch")"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

mkdir -p "$install_dir"
curl -fsSL "$url" -o "$tmp_dir/$asset"
actual_sha256="$(shasum -a 256 "$tmp_dir/$asset" | awk '{print $1}')"
if [[ "$actual_sha256" != "$expected_sha256" ]]; then
  echo "downloaded $asset checksum mismatch: expected $expected_sha256, got $actual_sha256" >&2
  exit 1
fi
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
