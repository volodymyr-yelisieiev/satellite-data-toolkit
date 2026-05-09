# Production Readiness Audit

Date: 2026-05-09
Project: `satellite-data-toolkit`
Version observed on `main`: `2.1.2`
Scope: Tauri desktop application, frontend, Rust backend, macOS and Windows packaging, repository operations, CI/CD, release readiness, tests, and brief alignment.

## Status

This is an evidence snapshot, not a production release sign-off.

Current status: **not production-complete**.

The repository has a solid production baseline, and the targeted hardening branches listed below have been integrated and pushed to `main`. GitHub-hosted workflows are restored now that the repository is public; Dependabot remains disabled to avoid noisy automated PRs. The app must not be treated as finished until public signing/notarization is configured and native Windows plus live credential-backed QA are completed.

## Evidence Map

| Area | Current evidence | Open work before production sign-off |
| --- | --- | --- |
| Visual and UX consistency | On 2026-05-09, `npm run visual:smoke` passed and captured 27 screenshots in `output/visual-smoke`, covering dashboard, NASA POWER, EUMETSAT, NDVI, PV, saved data, API slots, settings, and about screens at 1024x720, 1280x853, and 1440x900. The integrated hardening tightens clipped-control detection, removes fake settings behavior, stale refresh states, duplicate EUMETSAT actions, misleading download status, and static shell status, and adds click-through browser workflow smoke. | Perform native Windows 10/11 visual QA with actual DPI/scaling. Recheck macOS after signed/notarized packaging. |
| macOS packaging | On 2026-05-09, `./scripts/build-macos.sh` passed locally for `2.1.2`, produced `target/release/bundle/dmg/Satellite Data Toolkit_2.1.2_aarch64.dmg`, refreshed the EUMDAC sidecar manifest, verified codesign, wrote the `.sha256`, and passed `hdiutil verify`. The same release path is wired through hosted `Package artifacts` and `Release` workflows. | Configure Apple Developer ID and notarization secrets. Verify Gatekeeper acceptance, stapled ticket, Intel or universal build behavior, and first-launch behavior from a clean profile. |
| Windows packaging | Earlier CI produced MSI/NSIS artifacts, and the integrated packaging hardening adds pinned EUMDAC sidecar staging and packaged sidecar hash/signature verification. The current gate is `.\scripts\build-windows.ps1` plus `.\scripts\smoke-windows-msi.ps1` on a real Windows machine. | Validate MSI and NSIS on real Windows 10/11 machines, including launch without dev tools, uninstall cleanup, Authenticode signature, and SmartScreen behavior. |
| Backend and functionality | NASA POWER, local PV estimate, PVWatts/NLR call path, NDVI processing, saved datasets, exports, API slots, and EUMETSAT sidecar commands are covered by existing code and tests. Integrated hardening covers PVWatts error redaction and input validation, request timeout settings, saved-data and EUMETSAT flows, NDVI LZW/PackBits/multi-strip TIFF layouts, saved dataset bounds and CSV escaping, PVWatts inverter efficiency, NASA POWER enum and parameter validation, explicit export destinations, stored credential normalization, saved payload validation, non-finite numeric rejection, EUMETSAT time validation, EUMDAC output redaction, and EUMETSAT input validation before sidecar/keychain side effects. | Run live PVWatts with a real `nlr_pvwatts_key`. Run EUMETSAT search/download with real credentials and packaged sidecar. Add real provider NDVI fixture coverage if the product scope requires more GeoTIFF variants. |
| DRY, KISS, and architecture | Existing split between React UI, Tauri command layer, and Rust core is clear enough for production maintenance. Integrated hardening centralizes timeout clamping, sidecar preparation, footer release links, RustSec audit tool lookup, and stored-secret normalization for credential presence checks. | `src/App.tsx` remains large and should be decomposed only where it reduces concrete maintenance risk. Avoid broad refactors until native packaging and live-provider gaps are closed. |
| GitHub, CI, and CD | Public GitHub Actions are restored for CI, RustSec audit, manual package artifacts, and tag-based release publishing. Release tag/version preflight and version consistency guard scripts/tests remain active. Open PR count is 0, and `v2.1.2` is the current release tag for the restored hosted CI/CD path. | After the first green hosted run, enable required status checks on `main`. Release secrets are still optional/missing, so unsigned/ad-hoc assets are suitable for private review, not polished public distribution. |
| Tests and security | On 2026-05-09, the integrated `main` passed local `npm run verify`: version check, Tauri surface guard, TypeScript, Vitest, production Vite build, cargo fmt, cargo test, cargo check, cargo clippy, production npm audit, and build-chain npm audit. Local verification has also covered `npm run visual:smoke`, targeted Rust tests, `./scripts/audit-rust.sh`, and `git diff --check` across the hardening branches. | Keep RustSec allowances narrow and documented. Rerun visual smoke and native packaging checks after any UI or packaging change. Restore hosted/self-hosted protected checks when they can run without quota failures. |

## Open Pull Request Queue

These pull requests represented the production-hardening queue. Their branch changes were integrated into `main`, and the superseded GitHub pull requests were closed without updating them further:

| PR | Branch | Purpose |
| --- | --- | --- |
| #12 | `codex/dependency-compatibility-hardening` | Update Rust dependency compatibility set. |
| #13 | `codex/frontend-patch-dependencies` | Update frontend tooling patch dependencies. |
| #14 | `codex/footer-link-polish` | Polish footer release links. |
| #15 | `codex/backend-api-hardening` | Harden PVWatts and EUMDAC query handling. |
| #16 | `codex/package-clean-builds` | Harden package artifacts and EUMDAC sidecar staging. |
| #17 | `codex/ui-consistency-polish` | Guard visual smoke against clipped controls. |
| #18 | `codex/audit-path-hardening` | Resolve `cargo-audit` PATH lookup. |
| #19 | `codex/settings-ux-hardening` | Make settings persist real UI defaults. |
| #20 | `codex/saved-data-refresh-hardening` | Handle saved dataset refresh failures. |
| #21 | `codex/eumetsat-action-hardening` | Harden EUMETSAT action states. |
| #22 | `codex/eumetsat-download-result-log` | Log completed EUMETSAT downloads. |
| #23 | `codex/shell-status-polish` | Reflect workflow state in shell status. |
| #29 | `codex/npm-dev-audit-policy` | Gate npm build-chain audit before release. |
| #30 | `codex/ndvi-compression-fixture-coverage` | Broaden NDVI TIFF compression coverage. |
| #31 | `codex/browser-demo-contract-tests` | Cover browser demo IPC fallback. |
| #32 | `codex/browser-workflow-smoke` | Add browser workflow smoke coverage. |
| #33 | `codex/release-preflight-tests` | Cover release secret preflight. |
| #34 | `codex/release-tag-preflight` | Validate release tag before publishing. |
| #35 | `codex/storage-validation-tests` | Cover storage validation boundaries. |
| #36 | `codex/tauri-surface-guard-tests` | Cover Tauri surface guard. |
| #37 | `codex/pvwatts-inverter-efficiency` | Send inverter efficiency to PVWatts. |
| #38 | `codex/nasa-power-request-validation` | Validate NASA POWER request enums. |
| #39 | `codex/export-destination-paths` | Fix explicit export destination paths. |
| #40 | `codex/version-guard-tests` | Cover version consistency guard. |
| #41 | `codex/credential-presence-hardening` | Normalize stored credential presence checks. |
| #42 | `codex/saved-dataset-load-validation` | Validate saved dataset payloads on load. |
| #43 | `codex/saved-record-count-validation` | Validate saved dataset record counts. |
| #44 | `codex/pv-finite-input-validation` | Reject non-finite local PV inputs. |
| #45 | `codex/pvwatts-finite-input-validation` | Reject non-finite PVWatts inputs. |
| #46 | `codex/nasa-parameter-name-normalization` | Reject untrimmed NASA POWER parameters. |
| #47 | `codex/eumetsat-time-validation` | Validate EUMETSAT query times. |
| #48 | `codex/eumdac-output-redaction` | Redact EUMDAC command output secrets. |
| #49 | `codex/eumetsat-validate-before-sidecar` | Validate EUMETSAT inputs before sidecar work. |

## External Blockers

Production completion is currently blocked by issues outside the local source tree:

- Dependabot config remains removed to avoid noisy automated PRs. GitHub Actions hosted workflows are restored for the now-public repository.
- Apple Developer ID signing and notarization secrets are not configured: #27.
- Windows Authenticode signing is not configured, and native Windows 10/11 install, uninstall, visual, SmartScreen, and first-launch QA has not been completed: #25.
- Live EUMETSAT and PVWatts credentials are not available for packaged-app validation: #26.

## Production Sign-Off Gate

Do not mark the goal complete until all of these are true:

- Integrated `main` is pushed and passes local `npm run verify`, visual smoke, and native packaging checks.
- Hosted GitHub checks pass on `main`/release tag: Ubuntu verify, macOS verify, Windows verify, visual smoke, RustSec audit, macOS DMG build, and Windows MSI/NSIS build.
- A signed and notarized macOS DMG passes `codesign`, `spctl`, `stapler`, and first-launch checks.
- Signed Windows MSI and NSIS installers pass clean Windows 10/11 install, launch, uninstall, Authenticode, and SmartScreen checks.
- Packaged EUMDAC sidecars are checksum-manifested, signed where required, and validated with real credentials.
- Live NASA POWER, PVWatts, EUMETSAT, NDVI sample, saved-data, export, API-slot, and settings workflows pass on packaged builds.
- Release publishing uploads the expected assets and `SHA256SUMS.txt` from a protected tag after required signing/notarization secrets are configured.
