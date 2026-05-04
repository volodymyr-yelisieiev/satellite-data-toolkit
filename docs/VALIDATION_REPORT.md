# Validation Report

Date: 2026-05-03  
Machine: macOS 26.4.1 arm64, Apple Silicon local workstation  
Project version: 2.1.0  
Scope: final local validation before handoff ZIP

## Executive Status

| Area | Result | Notes |
| --- | --- | --- |
| macOS local app/DMG | Pass for private review | `.app` builds, ad-hoc codesign verifies, DMG verifies and mounts. |
| macOS public release | Not ready | Gatekeeper rejects ad-hoc app/DMG because Developer ID signing and notarization are not configured. |
| Windows packaging | Configured, not verified | MSI/NSIS targets and script exist, but no Windows VM/runner was available in this workspace. |
| Core build/test | Pass | TypeScript build, Rust tests/check/clippy, and production npm audit passed. |
| NASA POWER live sample | Pass | New York 2024-05-01..2024-05-05 returned 5 normalized daily records. |
| UI visual smoke | Pass with caveat | Key screens rendered at target widths with no persistent clipped text/controls detected. |
| EUMETSAT | Partial | Sidecar command wiring exists, but no EUMDAC sidecar/credentials were available for live QA. |
| PVWatts/NLR | Partial | Client and validation exist, but no real API key was available for live QA. |
| NDVI | Partial production | Math/tests exist; production GeoTIFF CRS/tag preservation remains missing. |

## Subagent Audit Summary

Five validation passes were run in parallel before final fixes:

- macOS packaging: verified local `.app`/DMG path, ad-hoc signature, DMG structure, and public-release signing gaps.
- Windows packaging: confirmed MSI/NSIS config and listed required Windows 10/11 verification steps.
- Security/reproducibility: confirmed CSP/env/keychain baseline and flagged missing cargo advisory policy plus sidecar trust hardening.
- Product/API: confirmed NASA/local PV/saved-data/API-slot base and flagged EUMDAC CLI shape, PVWatts validation, NDVI GeoTIFF gaps.
- Docs/handoff: flagged README/PACKAGING as too thin and required explicit credentials, storage, release gaps, and ZIP contents.

Fixes applied after those audits:

- EUMDAC search now passes bbox as four CLI arguments and uses `-s`/`-e` style dates.
- EUMDAC download now includes `collectionId`.
- PVWatts validation now checks module type, array type, timeframe, azimuth `< 360`, and allows limited negative losses.
- README and packaging docs were rewritten for handoff use.

## Commands Run

### Local Verification

```bash
./scripts/verify.sh
```

Result: pass.

Checks included:

- `npm run build`
- `cargo test --workspace --locked`
- `cargo check --workspace --locked`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `npm audit --omit=dev`

Rust test result:

```text
satellite-core: 10 passed
satellite-data-toolkit lib: 3 passed
```

Production npm audit:

```text
0 vulnerabilities
```

### Tauri Environment

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
npm run tauri -- info
```

Result: pass with known environment note.

Observed:

- Xcode Command Line Tools: installed
- Xcode.app: not installed
- rustc: 1.95.0
- cargo: 1.95.0
- Rust toolchain override: `rust-toolchain.toml`
- Node: 24.13.0
- npm: 11.6.2
- Tauri Rust crate: 2.11.0
- `@tauri-apps/api`: 2.11.0
- `@tauri-apps/cli`: 2.11.0
- CSP enabled

Full Xcode is not required for the local build performed here, but public macOS release workflows may need additional Apple tooling depending on the signing/notarization setup.

### NASA POWER Live Sample

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo run -p satellite-cli --locked
```

Result: pass.

Request:

```text
latitude=40.7128
longitude=-74.006
date=2024-05-01..2024-05-05
temporal=daily
parameters=ALLSKY_SFC_SW_DWN,T2M,WS2M
```

Observed:

- records: 5
- statusCode: 200
- apiVersion: v2.8.11
- `ALLSKY_SFC_SW_DWN`: `kW-hr/m^2/day`
- `T2M`: `C`
- `WS2M`: `m/s`

### macOS Build

```bash
./scripts/build-macos.sh
```

Result: pass.

Outputs:

```text
target/release/bundle/macos/Satellite Data Toolkit.app
target/release/bundle/dmg/Satellite Data Toolkit_2.1.0_aarch64.dmg
```

Post-build checks:

```bash
codesign --verify --deep --strict --verbose=2 "target/release/bundle/macos/Satellite Data Toolkit.app"
hdiutil verify "target/release/bundle/dmg/Satellite Data Toolkit_2.1.0_aarch64.dmg"
```

Result: pass.

DMG mount check:

```text
Applications -> /Applications
Satellite Data Toolkit.app
```

Artifact details:

```text
SHA256: 4438708b3a527a4ce548e9d4b6d1ea07abd7f8777eb2bdbec4f7983851a0cdfc
Size: 6.0M
Binary: Mach-O 64-bit executable arm64
Signature: adhoc
TeamIdentifier: not set
```

Gatekeeper checks:

```bash
spctl --assess --type execute --verbose=4 "target/release/bundle/macos/Satellite Data Toolkit.app"
spctl --assess --type open --verbose=4 "target/release/bundle/dmg/Satellite Data Toolkit_2.1.0_aarch64.dmg"
```

Result: rejected, expected for unsigned/not-notarized public distribution.

## UI Visual Smoke

Browser/demo mode was used only for visual layout smoke. Native commands are mocked in browser mode.

Screenshots saved under:

```text
output/playwright/
```

Viewport checks:

| Viewport | Screen | Result |
| --- | --- | --- |
| 1280x853 | NASA POWER | No clipped text/controls detected |
| 1024x720 | NASA POWER | No clipped text/controls detected |
| 1440x900 | NASA POWER | No clipped text/controls detected |
| 1280x853 | EUMETSAT | No clipped text/controls detected |
| 1280x853 | NDVI Calculator | No clipped text/controls detected |
| 1280x853 | PV Estimate | One transient input scroll-width warning; focused recheck returned no clipped inputs |
| 1280x853 | Saved Data | No clipped text/controls detected |
| 1280x853 | API Slots | No clipped text/controls detected |
| 1280x853 | Settings | No clipped text/controls detected |
| 1280x853 | About | No clipped text/controls detected |

## Not Fully Verifiable In This Workspace

- Windows MSI/NSIS build, install, launch, Credential Manager, WebView2, signing, and uninstall checks.
- Public macOS Developer ID signing, hardened runtime, notarization, stapling, and Gatekeeper acceptance.
- EUMETSAT live product search/download because no bundled sidecar and test credentials were available.
- PVWatts/NLR live API result because no API key was available.
- Production NDVI georeferencing preservation because current implementation uses pure TIFF output rather than GDAL-backed GeoTIFF metadata preservation.

## Release Recommendation

Use the generated ZIP for colleague/source review and Apple Silicon private macOS smoke only.

Do not present it as a public cross-platform release until:

- Windows QA is run on Windows;
- macOS signing/notarization is configured;
- EUMDAC sidecars are bundled and signed;
- EUMETSAT/PVWatts live credentials are tested;
- NDVI GeoTIFF metadata preservation is implemented if production geospatial output is required.
