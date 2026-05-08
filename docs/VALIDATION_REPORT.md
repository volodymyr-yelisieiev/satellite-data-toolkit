# Validation Report

Date: 2026-05-03  
Machine: macOS 26.4.1 arm64, Apple Silicon local workstation  
Project version: 2.1.0  
Scope: final local validation before handoff ZIP

## 2026-05-08 Production Hardening Addendum

Scope: repository hardening pass for the Tauri desktop app on branch `codex/production-hardening`.

Additional changes validated locally:

- Frontend unit tests added with Vitest for shared UI/domain helpers.
- `./scripts/verify.sh` now runs TypeScript typecheck, frontend unit tests, Vite build, Rust fmt, Rust tests, Rust check, Rust clippy, and production npm audit.
- CI now verifies Ubuntu, macOS, and Windows runners.
- Release workflow now builds macOS DMG plus Windows MSI/NSIS on `v*` tags and publishes a consolidated `SHA256SUMS.txt`.
- macOS and Windows packaging scripts now avoid hardcoded artifact versions, emit checksums, and include optional signing/notarization plumbing that is skipped without release secrets.
- EUMETSAT sidecar calls now require a checksum-matching sidecar manifest, sync keychain credentials into EUMDAC before search/download, and redact secret values from process errors.
- UI styling was adjusted toward a more neutral production desktop palette with stable tabs and sticky table headers.

Local commands run successfully on May 8, 2026:

```bash
npm run test
npm run typecheck
npm run build
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm audit --omit=dev
```

macOS packaging was also rerun successfully on May 8, 2026:

```bash
./scripts/build-macos.sh
```

Output:

```text
target/release/bundle/macos/Satellite Data Toolkit.app
target/release/bundle/dmg/Satellite Data Toolkit_2.1.1_aarch64.dmg
target/release/bundle/dmg/Satellite Data Toolkit_2.1.1_aarch64.dmg.sha256
```

Observed DMG SHA256:

```text
54f2a92fd8c85219a6a24e903b24cd51694471cb8bcc9cc9d7353f5d7e9fd2bf
```

Browser visual smoke screenshots were captured for `dashboard`, `power`, `eumetsat`, `ndvi`, `pv`, `saved`, `api`, `settings`, and `about` at 1024x720, 1280x853, and 1440x900 under `output/visual-smoke/`. The 1024x720 pass exposed sidebar/footer density issues; those were fixed with scrollable navigation, active-item scroll alignment, and compact vertical spacing for short windows. This pass is now automated by `npm run visual:smoke` and the CI `Visual smoke` job.

Windows packaging was triggered with the `Windows package` workflow on branch `codex/production-hardening` after the latest hardening commits. Run `25565037772` passed on `652a42a` and uploaded `windows-msi` (6,886,172 bytes), `windows-nsis` (5,485,452 bytes), and `windows-sha256sums` (296 bytes) artifacts after the unified full verify preflight.

Remaining external blockers are unchanged except for the NDVI metadata gap, which is now locally closed for common GeoTIFF tags and Deflate-compressed TIFF inputs: Windows install/uninstall QA, public macOS Developer ID signing/notarization/stapling, signed bundled EUMDAC binaries, live EUMETSAT/PVWatts validation with real credentials, and broader real-world NDVI GeoTIFF fixture QA.

## Executive Status

| Area | Result | Notes |
| --- | --- | --- |
| macOS local app/DMG | Pass for private review | `.app` builds, ad-hoc codesign verifies, DMG verifies and mounts. |
| macOS public release | Not ready | Gatekeeper rejects ad-hoc app/DMG because Developer ID signing and notarization are not configured. |
| Windows packaging | CI build pass | MSI/NSIS/checksum artifacts were produced by the manual Windows package workflow; real install/uninstall QA is still required. |
| Core build/test | Pass | TypeScript build, Rust tests/check/clippy, and production npm audit passed. |
| NASA POWER live sample | Pass | New York 2024-05-01..2024-05-05 returned 5 normalized daily records. |
| UI visual smoke | Pass | Key screens render at target widths through automated Playwright smoke with screenshots uploaded by CI. |
| EUMETSAT | Partial | Sidecar command wiring and checksum-manifest trust gate exist, but no EUMDAC sidecar/credentials were available for live QA. |
| PVWatts/NLR | Partial | Client and validation exist, but no real API key was available for live QA. |
| NDVI | Production baseline | Math/tests exist; common GeoTIFF CRS/geotransform tags, `GDAL_NODATA` metadata, and Deflate-compressed TIFF inputs are covered in the pure-Rust path. |

## Subagent Audit Summary

Five validation passes were run in parallel before final fixes:

- macOS packaging: verified local `.app`/DMG path, ad-hoc signature, DMG structure, and public-release signing gaps.
- Windows packaging: confirmed MSI/NSIS config and listed required Windows 10/11 verification steps.
- Security/reproducibility: confirmed CSP/env/keychain baseline, added RustSec audit gates, and added EUMDAC sidecar checksum-manifest trust hardening.
- Product/API: confirmed NASA/local PV/saved-data/API-slot base and flagged EUMDAC CLI shape, PVWatts validation, and NDVI real-world fixture QA gaps.
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

- Windows MSI/NSIS install, launch, Credential Manager, WebView2, signing, and uninstall checks on real Windows 10/11 machines.
- Public macOS Developer ID signing, hardened runtime, notarization, stapling, and Gatekeeper acceptance.
- EUMETSAT live product search/download because no bundled sidecar and test credentials were available.
- PVWatts/NLR live API result because no API key was available.
- Broader NDVI fixture coverage for tiled and multi-provider GeoTIFFs beyond the local metadata/compression preservation tests.

## Release Recommendation

Use the generated ZIP for colleague/source review and Apple Silicon private macOS smoke only.

Do not present it as a public cross-platform release until:

- Windows QA is run on Windows;
- macOS signing/notarization is configured;
- EUMDAC sidecars are bundled and signed;
- EUMETSAT/PVWatts live credentials are tested;
- Broader NDVI GeoTIFF fixture QA is completed for the target satellite providers.
