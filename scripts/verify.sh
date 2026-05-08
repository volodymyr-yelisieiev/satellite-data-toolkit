#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

command -v node >/dev/null 2>&1 || { echo "node is required" >&2; exit 1; }

node scripts/verify.mjs
