#!/usr/bin/env bash
set -euo pipefail

PATH="/opt/tovuk/native-tools/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
export PATH

shell_entrypoints=(scripts/*.sh)

shellcheck -x "${shell_entrypoints[@]}"
shfmt -i 2 -ci -d scripts/*.sh
