#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

command -v npm >/dev/null 2>&1 || { echo "npm is required" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
command -v codesign >/dev/null 2>&1 || { echo "codesign is required" >&2; exit 1; }
command -v hdiutil >/dev/null 2>&1 || { echo "hdiutil is required" >&2; exit 1; }
command -v shasum >/dev/null 2>&1 || { echo "shasum is required" >&2; exit 1; }

rm -rf \
  target/release/bundle/macos \
  target/release/bundle/dmg

has_macos_notarization_credentials() {
  [[ -n "${MACOS_NOTARY_KEYCHAIN_PROFILE:-}" ]] ||
    [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]] ||
    [[ -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY_PATH:-}" ]]
}

npm ci
npm run verify
npm run tauri:build -- --bundles app,dmg

APP_PATH="target/release/bundle/macos/Satellite Data Toolkit.app"
DMG_PATH="$(find target/release/bundle/dmg -maxdepth 1 -type f -name '*.dmg' -print -quit)"
STAGING_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGING_DIR"' EXIT

if [[ ! -d "$APP_PATH" ]]; then
  echo "macOS app was not produced: $APP_PATH" >&2
  exit 1
fi

if [[ -z "$DMG_PATH" || ! -f "$DMG_PATH" ]]; then
  echo "macOS DMG was not produced" >&2
  exit 1
fi

if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  codesign --verify --deep --strict --verbose=2 "$APP_PATH"

  require_notarization="${SATELLITE_REQUIRE_MACOS_NOTARIZATION:-}"
  if [[ -z "$require_notarization" ]] && has_macos_notarization_credentials; then
    require_notarization="1"
  fi

  if [[ "$require_notarization" == "1" ]]; then
    command -v xcrun >/dev/null 2>&1 || { echo "xcrun is required for notarization checks" >&2; exit 1; }
    xcrun stapler validate "$APP_PATH"
    xcrun stapler validate "$DMG_PATH"
    spctl --assess --type execute --verbose=4 "$APP_PATH"
    spctl --assess --type open --verbose=4 "$DMG_PATH"
  else
    echo "macOS notarization checks skipped: SATELLITE_REQUIRE_MACOS_NOTARIZATION is not enabled"
  fi
else
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
fi

hdiutil verify "$DMG_PATH"

shasum -a 256 "$DMG_PATH" > "${DMG_PATH}.sha256"
