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

sign_path() {
  local identity="$1"
  local path="$2"
  local args=(--force --sign "$identity")
  if [[ "$identity" != "-" ]]; then
    args+=(--options runtime)
  fi
  codesign "${args[@]}" "$path"
}

sign_disk_image() {
  local identity="$1"
  local dmg_path="$2"
  codesign --force --sign "$identity" "$dmg_path"
  codesign --verify --verbose=2 "$dmg_path"
}

refresh_eumdac_manifest() {
  local app_path="$1"
  local manifest_path="$app_path/Contents/Resources/eumdac-sidecar-manifest.json"
  local sidecar_path=""

  for candidate in "$app_path/Contents/MacOS"/eumdac*; do
    if [[ -f "$candidate" ]]; then
      sidecar_path="$candidate"
      break
    fi
  done

  if [[ -z "$sidecar_path" ]]; then
    echo "EUMDAC sidecar was not found in $app_path/Contents/MacOS" >&2
    exit 1
  fi
  if [[ ! -f "$manifest_path" ]]; then
    echo "EUMDAC sidecar manifest was not found: $manifest_path" >&2
    exit 1
  fi

  local sidecar_name
  local sidecar_sha256
  sidecar_name="$(basename "$sidecar_path")"
  sidecar_sha256="$(shasum -a 256 "$sidecar_path" | awk '{print $1}')"

  node - "$manifest_path" "$sidecar_name" "$sidecar_sha256" <<'NODE'
const [manifestPath, sidecarName, sidecarSha256] = process.argv.slice(2);
const fs = require("node:fs");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
for (const entry of manifest.binaries ?? []) {
  if (entry.name === sidecarName || entry.name === "eumdac" || entry.name === "eumdac.exe") {
    entry.sha256 = sidecarSha256;
  }
}
fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
NODE
}

sign_macos_app() {
  local identity="$1"
  local app_path="$2"

  while IFS= read -r -d '' executable; do
    sign_path "$identity" "$executable"
  done < <(find "$app_path/Contents/MacOS" -maxdepth 1 -type f -perm -111 -print0)

  refresh_eumdac_manifest "$app_path"
  sign_path "$identity" "$app_path"
  codesign --verify --deep --strict --verbose=2 "$app_path"
}

rebuild_dmg() {
  local app_path="$1"
  local dmg_path="$2"
  local staging_dir="$3"

  rm -rf "$staging_dir"
  mkdir -p "$staging_dir"
  cp -R "$app_path" "$staging_dir/"
  ln -s /Applications "$staging_dir/Applications"
  hdiutil create \
    -volname "Satellite Data Toolkit" \
    -srcfolder "$staging_dir" \
    -ov \
    -format UDZO \
    "$dmg_path"
}

submit_for_notarization() {
  local artifact_path="$1"
  local timeout="${MACOS_NOTARY_TIMEOUT:-30m}"
  local args=(notarytool submit --wait --timeout "$timeout")

  if [[ -n "${MACOS_NOTARY_KEYCHAIN_PROFILE:-}" ]]; then
    args+=(--keychain-profile "$MACOS_NOTARY_KEYCHAIN_PROFILE")
  elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    args+=(--apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID")
  elif [[ -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY_PATH:-}" ]]; then
    args+=(--key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER")
  else
    echo "macOS notarization credentials are required" >&2
    exit 1
  fi

  xcrun "${args[@]}" "$artifact_path"
}

notarize_app_bundle() {
  local app_path="$1"
  local staging_dir="$2"
  local zip_path="$staging_dir/app-notarization.zip"

  command -v ditto >/dev/null 2>&1 || { echo "ditto is required for app notarization" >&2; exit 1; }
  mkdir -p "$staging_dir"
  rm -f "$zip_path"
  ditto -c -k --keepParent "$app_path" "$zip_path"
  submit_for_notarization "$zip_path"
  xcrun stapler staple "$app_path"
  xcrun stapler validate "$app_path"
  spctl --assess --type execute --verbose=4 "$app_path"
}

notarize_dmg() {
  local dmg_path="$1"

  submit_for_notarization "$dmg_path"
  xcrun stapler staple "$dmg_path"
  xcrun stapler validate "$dmg_path"
  spctl --assess --type open --verbose=4 "$dmg_path"
}

npm ci
npm run verify
npm run eumdac:prepare
npm run tauri:build -- --config src-tauri/tauri.eumdac.generated.conf.json --bundles app,dmg

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
  sign_macos_app "$APPLE_SIGNING_IDENTITY" "$APP_PATH"

  require_notarization="${SATELLITE_REQUIRE_MACOS_NOTARIZATION:-}"
  if [[ -z "$require_notarization" ]] && has_macos_notarization_credentials; then
    require_notarization="1"
  fi

  if [[ "$require_notarization" == "1" ]]; then
    command -v xcrun >/dev/null 2>&1 || { echo "xcrun is required for notarization checks" >&2; exit 1; }
    notarize_app_bundle "$APP_PATH" "$STAGING_DIR"
    rebuild_dmg "$APP_PATH" "$DMG_PATH" "$STAGING_DIR"
    sign_disk_image "$APPLE_SIGNING_IDENTITY" "$DMG_PATH"
    notarize_dmg "$DMG_PATH"
  else
    rebuild_dmg "$APP_PATH" "$DMG_PATH" "$STAGING_DIR"
    echo "macOS notarization checks skipped: SATELLITE_REQUIRE_MACOS_NOTARIZATION is not enabled"
  fi
else
  sign_macos_app "-" "$APP_PATH"
  rebuild_dmg "$APP_PATH" "$DMG_PATH" "$STAGING_DIR"
fi

hdiutil verify "$DMG_PATH"

shasum -a 256 "$DMG_PATH" > "${DMG_PATH}.sha256"
