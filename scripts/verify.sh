#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" == "Darwin" ]]; then
  export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
fi

command -v node >/dev/null 2>&1 || { echo "node is required" >&2; exit 1; }

node scripts/verify.mjs
