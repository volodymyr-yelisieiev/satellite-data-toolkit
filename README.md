# Satellite Data Toolkit

Desktop toolkit for NASA POWER data, EUMETSAT product access, NDVI calculation, and PV energy estimates.

This repository is a Tauri 2 application with a React/TypeScript UI and Rust backend/core. It is intended to replace the original Python/Tkinter zip with a one-click desktop app that does not require end users to install Python, Node, Rust, or libraries.

## Reviewer Snapshot

- Current package status: macOS and Windows packaging scripts are configured with checksums; Windows MSI/NSIS CI packaging has been verified, and the release workflow publishes macOS DMG plus Windows MSI/NSIS on `v*` tags.
- Current UI status: implemented desktop shell matching the requested dark toolkit structure with a production-oriented neutral palette, stable workflow tabs, request panels, response tables, logs, saved data, API slots, settings, and about screen.
- Current backend status: NASA POWER live fetch/normalization, SQLite saved datasets, CSV/JSON export, local PV estimate, PVWatts/NLR command, API keychain slots, and pure-Rust TIFF NDVI with common GeoTIFF metadata/nodata preservation are implemented.
- Current release gaps: public macOS signing/notarization, Windows install/uninstall QA, bundled/signed EUMDAC sidecar, live EUMETSAT/PVWatts verification with real credentials, and broader real-world GeoTIFF fixture QA for NDVI.

## Feature Status

| Area | Status | Notes |
| --- | --- | --- |
| macOS app/DMG | Partial release-ready | Builds locally and is ad-hoc signed. Public distribution still needs Developer ID, hardened runtime, notarization, `spctl`, and `stapler` checks. |
| Windows MSI/NSIS | Configured with CI build | Tauri config and PowerShell build script produce MSI/NSIS and checksums. Needs Windows 10/11 install/uninstall QA. |
| NASA POWER | Implemented | Uses JSON API, normalizes daily/hourly records, preserves units metadata, handles fill values. NASA POWER does not need an API key. |
| Saved Data | Implemented | Saves datasets to SQLite, supports preview, delete, and CSV/JSON export. |
| API Slots | Implemented base | Stores credentials in OS keychain under service `Satellite Data Toolkit`; no secrets are written to SQLite or logs. |
| PV Local Estimate | Implemented approximate | Uses normalized NASA POWER irradiance with explicit assumptions and missing-record accounting. |
| PVWatts/NLR | Implemented, needs key QA | Uses `developer.nlr.gov` endpoint and stored `nlr_pvwatts_key`; live validation requires a real key. |
| NDVI | Production baseline | Reads two TIFF rasters and writes Float32 NDVI TIFF. It preserves common GeoTIFF CRS/geotransform tags from the Red band and uses explicit or input `GDAL_NODATA` metadata. |
| EUMETSAT | Partial production | Sidecar discovery/search/download hooks exist. Credentials are read from OS keychain and synced to EUMDAC before CLI calls. EUMDAC binary still must be bundled/signed and live QA requires real credentials. |
| Security posture | Baseline+ | CSP is enabled, Tauri env exposure is limited to `VITE_`, no shell/fs/http plugins are enabled, key slots are whitelisted, EUMDAC process errors redact stored secrets, and RustSec audit is wired into CI/release. Signed sidecars are still required before public release. |

## Included Workflows

### NASA POWER

The NASA POWER screen supports:

- latitude/longitude/date range/community/time format input;
- parameter chips such as `ALLSKY_SFC_SW_DWN`, `T2M`, and `WS2M`;
- quick city examples;
- live fetch through the NASA POWER API;
- status cards, normalized table preview, units metadata, CSV/JSON export, and activity log.

Sample acceptance case:

```text
New York
2024-05-01..2024-05-05
daily
ALLSKY_SFC_SW_DWN,T2M,WS2M
```

Expected result: 5 normalized records. Daily `ALLSKY_SFC_SW_DWN` is treated as `kW-hr/m^2/day`, not `W/m^2`.

### PV Estimate

There are two modes:

- Local quick estimate in Rust. This is approximate and intended for fast screening.
- PVWatts/NLR mode. This calls the current NLR PVWatts V8 API when a `nlr_pvwatts_key` is stored.

Local estimate supports capacity, tilt, azimuth, losses, inverter efficiency, source parameter, used/missing record counts, and daily/hourly irradiance unit handling.

### NDVI

The NDVI screen accepts:

- red band TIFF path;
- NIR band TIFF path;
- output TIFF path;
- red/NIR scale factors;
- explicit nodata value.

Current NDVI output is a Float32 TIFF with NDVI values from `(NIR - Red) / (NIR + Red)`. It handles zero denominators, nodata, scale factors, mismatched dimensions, and TIFF read/write tests. The Rust path preserves common GeoTIFF CRS/geotransform tags from the Red band, including model scale/tiepoint/transformation tags, GeoKey directory tags, Geo ASCII/double parameters, and `GDAL_NODATA`.

### EUMETSAT

The EUMETSAT screen is wired for a bundled EUMDAC command-line sidecar:

- `check_eumdac_sidecar`
- `get_eumdac_sidecar_status`
- `fetch_eumetsat_products`
- `download_eumetsat_product`

The app currently looks for executable files named:

```text
eumdac
eumdac.exe
eumdac-cli
eumdac-cli.exe
```

next to the packaged executable. The backend computes the sidecar SHA256 and only trusts it when a matching entry exists in `eumdac-sidecar-manifest.json` or `eumdac-sidecars.json` next to the executable. Production packaging should place platform-specific EUMDAC binaries under `src-tauri/binaries/`, add them to `src-tauri/tauri.conf.json > bundle.externalBin`, record checksum/license/source in the manifest, and sign/notarize them with the app.

EUMDAC 3.x exposes `set-credentials`, `search`, and `download` commands. Before search/download, the app reads `eumetsat_consumer_key` and `eumetsat_consumer_secret` from the OS keychain and syncs them into an app-scoped EUMDAC config environment for the sidecar process. Process errors are redacted before being returned to the UI.

## Credentials

Credential slots are stored in the OS keychain with service name `Satellite Data Toolkit`.

| Slot | Purpose | Required For |
| --- | --- | --- |
| `eumetsat_consumer_key` | EUMETSAT consumer key | EUMETSAT sidecar workflows |
| `eumetsat_consumer_secret` | EUMETSAT consumer secret | EUMETSAT sidecar workflows |
| `nlr_pvwatts_key` | PVWatts/NLR API key | PVWatts mode |

NASA POWER does not require an API key.

Important: EUMETSAT credential sync is implemented for the sidecar workflow, but final authentication/search/download behavior must be validated against the exact bundled EUMDAC distribution and real credentials.

## Local Storage

Runtime data is stored in the Tauri app data directory for identifier `com.satellite.datatoolkit`.

macOS expected location:

```text
~/Library/Application Support/com.satellite.datatoolkit/
```

Files created there:

```text
toolkit.sqlite
exports/
```

Limits:

- maximum saved dataset records: 120,000;
- maximum saved/exported dataset payload: 64 MiB;
- maximum saved dataset name length: 160 bytes.

## Development Prerequisites

These are build-time requirements only. End users of the packaged app do not need them.

- Node.js: `.node-version` pins `24.13.0`; `package.json` allows `>=22.12.0`.
- npm: `packageManager` pins `npm@11.6.2`; `package.json` allows `>=11.0.0`.
- Rust: `rust-toolchain.toml` pins `1.95.0` with `clippy` and `rustfmt`.
- macOS packaging: Xcode Command Line Tools, `codesign`, `hdiutil`.
- Windows packaging: Windows 10/11, MSVC Rust toolchain, Microsoft C++ Build Tools/Windows SDK, WebView2 installer policy, and signing certificate for release builds.

## Install Dependencies

On this macOS workstation the dependencies were installed via Homebrew/rustup. A clean setup is:

```bash
brew install node rustup
rustup default 1.95.0
rustup component add clippy rustfmt
npm ci
```

## Run From Source

Browser/demo UI only. This uses mocked Tauri responses and is useful for frontend inspection:

```bash
npm run dev
```

Native Tauri development. This uses real keychain, SQLite, exports, NASA POWER requests, and Rust commands:

```bash
npm run tauri:dev
```

## Verify

Run all local checks:

```bash
./scripts/verify.sh
```

This currently runs:

- `npm run version:check`
- `npm run typecheck`
- `npm run test`
- `npm run build`
- `cargo fmt --all -- --check`
- `cargo test --workspace --locked`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `npm audit --omit=dev`

## Build macOS

```bash
./scripts/build-macos.sh
```

Expected outputs:

```text
target/release/bundle/macos/Satellite Data Toolkit.app
target/release/bundle/dmg/Satellite Data Toolkit_2.1.1_aarch64.dmg
```

Without Apple Developer ID secrets, the script performs a local ad-hoc signature, verifies the `.app` with `codesign --verify --deep --strict`, rebuilds the DMG with an `/Applications` symlink, verifies the DMG with `hdiutil verify`, and writes a `.sha256` checksum next to the DMG. When `APPLE_SIGNING_IDENTITY` is configured, the script preserves the Tauri-signed output and can require notarization/stapling checks through `SATELLITE_REQUIRE_MACOS_NOTARIZATION=1`.

For public distribution, ad-hoc signing is not enough. Use Apple Developer ID signing, hardened runtime, notarization, stapling, and Gatekeeper verification.

## Build Windows

Run on a Windows build machine:

```powershell
.\scripts\build-windows.ps1
```

Expected outputs:

```text
target\release\bundle\msi\
target\release\bundle\nsis\
target\release\bundle\SHA256SUMS.txt
```

Current status: MSI/NSIS packaging is configured and has produced CI artifacts with checksums. `WINDOWS_SIGN_COMMAND` can be set in CI to Authenticode-sign Tauri Windows bundle targets through `scripts/sign-windows.ps1`; without it, signing is explicitly skipped. Before shipping, run MSI and NSIS install/uninstall smoke tests on Windows 10/11, verify WebView2 behavior, Credential Manager storage, first-run offline behavior, code signing, and SmartScreen reputation.

## GitHub CI/CD

The default CI workflow runs local verification on Ubuntu, macOS, and Windows. The release workflow runs on `v*` tags or a manual workflow dispatch with an existing tag, builds:

```text
macOS DMG
Windows MSI
Windows NSIS installer
SHA256SUMS.txt
```

and uploads those assets to the matching GitHub release.

Important repository state as of May 8, 2026: a separate `rust-pro-v3.0.0` release exists from the `codex/rust-pro-windows-exe` branch and points to a portable Rust-only Windows EXE. The Tauri desktop app release line should use `v*` tags; the next Tauri release should be published as latest to avoid confusing end users.

## Review ZIP Contents

The handoff ZIP is intended to contain:

- source code and lockfiles;
- `README.md`;
- `docs/`;
- `scripts/`;
- macOS Apple Silicon DMG under `artifacts/macos/`;
- visual QA screenshots under `artifacts/visual/`;
- `SHA256SUMS.txt` for included artifacts.

It intentionally excludes heavy/generated/local files:

- `node_modules/`;
- `target/`;
- `dist/`;
- `.playwright-cli/`;
- old Python zip;
- local screenshots;
- previous zip files.

## Known Missing Work Before Public Release

- Windows install/uninstall QA on Windows 10/11.
- Apple Developer ID signing, hardened runtime, notarization, stapling, and public Gatekeeper acceptance.
- Bundled, signed, checksum-verified EUMDAC sidecar per platform.
- Live EUMETSAT auth/search/download QA with real credentials and the exact bundled sidecar.
- Live PVWatts/NLR QA with real API key.
- Broader NDVI QA with real-world tiled and multi-provider GeoTIFF fixtures.

## Troubleshooting

If `cargo` is not found on macOS, make sure Homebrew rustup is on PATH:

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
```

If `npm ci` fails, verify Node and npm versions:

```bash
node --version
npm --version
```

If macOS blocks a local review app downloaded from a ZIP/DMG, remove quarantine only for private testing:

```bash
xattr -dr com.apple.quarantine "/Applications/Satellite Data Toolkit.app"
```

Do not use this as a substitute for Developer ID signing and notarization in a public release.

## Reference Links

- NASA POWER API: https://power.larc.nasa.gov/docs/services/api/
- Tauri macOS bundles: https://v2.tauri.app/distribute/macos-application-bundle/
- Tauri Windows installer: https://tauri.app/distribute/windows-installer/
- Tauri sidecars: https://tauri.app/develop/sidecar/
- EUMDAC package: https://pypi.org/project/eumdac/
- PVWatts/NLR docs: https://developer.nlr.gov/docs/solar/pvwatts/
