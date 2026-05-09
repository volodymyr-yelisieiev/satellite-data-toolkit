#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"

command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
command -v cargo-audit >/dev/null 2>&1 || {
  echo "cargo-audit is required. Install with: cargo install cargo-audit --locked --version 0.22.1" >&2
  exit 1
}

cargo audit
