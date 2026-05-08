#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

command -v npm >/dev/null 2>&1 || { echo "npm is required" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
command -v codesign >/dev/null 2>&1 || { echo "codesign is required" >&2; exit 1; }
command -v hdiutil >/dev/null 2>&1 || { echo "hdiutil is required" >&2; exit 1; }
command -v shasum >/dev/null 2>&1 || { echo "shasum is required" >&2; exit 1; }

npm ci
npm run verify
npm run tauri:build -- --bundles app,dmg

APP_PATH="target/release/bundle/macos/Satellite Data Toolkit.app"
DMG_PATH="$(find target/release/bundle/dmg -maxdepth 1 -type f -name '*.dmg' -print -quit)"
STAGING_DIR="$(mktemp -d)"

if [[ ! -d "$APP_PATH" ]]; then
  echo "macOS app was not produced: $APP_PATH" >&2
  exit 1
fi

if [[ -z "$DMG_PATH" || ! -f "$DMG_PATH" ]]; then
  echo "macOS DMG was not produced" >&2
  exit 1
fi

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

shasum -a 256 "$DMG_PATH" > "${DMG_PATH}.sha256"
