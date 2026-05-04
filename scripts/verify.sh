#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

command -v node >/dev/null 2>&1 || { echo "node is required" >&2; exit 1; }
command -v npm >/dev/null 2>&1 || { echo "npm is required" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }

npm run build

cargo test --workspace --locked
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm audit --omit=dev
