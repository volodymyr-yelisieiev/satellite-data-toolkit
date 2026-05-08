#!/usr/bin/env bash
set -euo pipefail

missing=()

require_secret() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    missing+=("$name")
  fi
}

has_apple_id_notary_secrets() {
  [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]
}

has_app_store_connect_notary_secrets() {
  [[ -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY_P8_BASE64:-}" ]]
}

require_secret WINDOWS_SIGN_COMMAND
require_secret APPLE_CERTIFICATE
require_secret APPLE_CERTIFICATE_PASSWORD
require_secret KEYCHAIN_PASSWORD

if ! has_apple_id_notary_secrets && ! has_app_store_connect_notary_secrets; then
  missing+=("APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID or APPLE_API_KEY/APPLE_API_ISSUER/APPLE_API_KEY_P8_BASE64")
fi

if (( ${#missing[@]} > 0 )); then
  echo "Refusing to publish a public release without production signing/notarization secrets." >&2
  printf 'Missing: %s\n' "${missing[@]}" >&2
  exit 1
fi

echo "Release signing/notarization secret preflight passed."
