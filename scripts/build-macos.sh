#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

command -v npm >/dev/null 2>&1 || { echo "npm is required" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
command -v codesign >/dev/null 2>&1 || { echo "codesign is required" >&2; exit 1; }
command -v hdiutil >/dev/null 2>&1 || { echo "hdiutil is required" >&2; exit 1; }

npm ci
npm run build
npm run tauri:build -- --bundles app,dmg

APP_PATH="target/release/bundle/macos/Satellite Data Toolkit.app"
DMG_PATH="target/release/bundle/dmg/Satellite Data Toolkit_2.1.0_aarch64.dmg"
STAGING_DIR="$(mktemp -d)"

codesign --force --deep --sign - "$APP_PATH"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

cp -R "$APP_PATH" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"
hdiutil create \
  -volname "Satellite Data Toolkit" \
  -srcfolder "$STAGING_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH"
rm -rf "$STAGING_DIR"
hdiutil verify "$DMG_PATH"
