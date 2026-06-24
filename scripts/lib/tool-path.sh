# shellcheck shell=bash
# Standard tool path for trusted public package checks.

tovuk_prepend_tool_path() {
  PATH="/opt/tovuk/native-tools/bin:/opt/tovuk/cargo-tools/bin:/opt/tovuk/cargo/bin:/opt/tovuk/rust/stable/bin:/opt/tovuk/node/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"
  export PATH
}
