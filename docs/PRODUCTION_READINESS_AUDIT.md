# Production Readiness Audit

Date: 2026-05-09
Project: `satellite-data-toolkit`
Version observed on `main`: `2.1.1`
Scope: Tauri desktop application, frontend, Rust backend, macOS and Windows packaging, repository operations, CI/CD, release readiness, tests, and brief alignment.

## Status

This is an evidence snapshot, not a production release sign-off.

Current status: **not production-complete**.

The repository has a solid production baseline and a queue of targeted hardening pull requests, but the app must not be treated as finished until the pull-request queue is merged through required checks, public signing/notarization is configured, and native Windows plus live credential-backed QA are completed.

## Evidence Map

| Area | Current evidence | Open work before production sign-off |
| --- | --- | --- |
| Visual and UX consistency | `npm run visual:smoke` covers dashboard, NASA POWER, EUMETSAT, NDVI, PV, saved data, API slots, settings, and about screens at 1024x720, 1280x853, and 1440x900. PR #17 tightens clipped-control detection. PRs #19-#23 remove fake settings behavior, stale refresh states, duplicate EUMETSAT actions, misleading download status, and static shell status. PR #32 adds click-through browser workflow smoke and makes workflow activity feedback visible outside NASA POWER. | Merge and rerun required CI. Perform native Windows 10/11 visual QA with actual DPI/scaling. Recheck macOS after signed/notarized packaging. |
| macOS packaging | `./scripts/build-macos.sh` builds and verifies a local `.app` and DMG. PR #16 cleans stale artifacts, stages pinned EUMDAC sidecars, verifies sidecar hashes, signs nested sidecars when configured, refreshes manifests after signing, and wires notarization/stapling flow. | Configure Apple Developer ID and notarization secrets. Verify Gatekeeper acceptance, stapled ticket, Intel or universal build behavior, and first-launch behavior from a clean profile. |
| Windows packaging | Existing Windows workflows build MSI/NSIS artifacts and run MSI quiet install/uninstall smoke. PR #16 adds pinned EUMDAC sidecar staging and packaged sidecar hash/signature verification. | GitHub Actions billing currently prevents current Windows jobs from starting. Validate MSI and NSIS on real Windows 10/11 machines, including launch without dev tools, uninstall cleanup, Authenticode signature, and SmartScreen behavior. |
| Backend and functionality | NASA POWER, local PV estimate, PVWatts/NLR call path, NDVI processing, saved datasets, exports, API slots, and EUMETSAT sidecar commands are covered by existing code and tests. PR #15 hardens PVWatts error redaction and input validation. PR #19 applies request timeout settings to live calls. PRs #20-#22 harden saved-data and EUMETSAT flows. PR #30 adds NDVI coverage for LZW, PackBits, and multi-strip TIFF layouts. PR #35 covers saved dataset storage bounds, saved-name validation, and CSV escaping for quotes, newlines, and missing values. PR #37 sends UI inverter efficiency through the PVWatts/NLR `inv_eff` request parameter and validates the documented range. PR #38 validates NASA POWER community and time-standard enums before HTTP requests. PR #39 fixes explicit export destinations such as bare filenames and covers path resolution boundaries. PR #41 treats whitespace-only stored credentials as missing across PVWatts and EUMETSAT flows. PR #42 validates saved dataset payloads after loading from SQLite. PR #43 rejects negative saved dataset record counts from local storage. PR #44 rejects non-finite local PV numeric inputs. PR #45 rejects non-finite PVWatts numeric inputs. PR #46 rejects untrimmed NASA POWER parameter names before URL construction. PR #47 validates EUMETSAT query time formats and ordering before sidecar calls. PR #48 redacts EUMETSAT credential values from EUMDAC command output returned to the UI. | Run live PVWatts with a real `nlr_pvwatts_key`. Run EUMETSAT search/download with real credentials and packaged sidecar. Add real provider NDVI fixture coverage if the product scope requires more GeoTIFF variants. |
| DRY, KISS, and architecture | Existing split between React UI, Tauri command layer, and Rust core is clear enough for production maintenance. PR #19 centralizes timeout clamping. PR #16 centralizes sidecar preparation. PR #14 centralizes footer release links. PR #18 hardens the RustSec audit tool lookup. PR #41 centralizes stored-secret normalization for credential presence checks. | `src/App.tsx` remains large and should be decomposed only where it reduces concrete maintenance risk. Avoid broad refactors until the current hardening PR queue is merged. |
| GitHub, CI, and CD | Branch protection requires Ubuntu/macOS/Windows verify jobs, visual smoke, RustSec audit, macOS DMG, and Windows MSI/NSIS packaging. Workflows exist for CI, macOS package, Windows package, Rust audit, and release publishing. Dependabot and security policy are present. PR #33 adds test coverage for the release signing/notarization secret preflight. PR #34 adds release tag/version preflight before publishing. PR #40 adds contract tests for the version consistency guard used by `npm run verify`. | Required checks are blocked before runner startup by GitHub billing/spending-limit state. Release secrets are not configured. Merge queue cannot prove the current hardening changes until those external blockers are removed. |
| Tests and security | Local verification has covered `npm run verify`, `npm run visual:smoke`, targeted Rust tests, `./scripts/audit-rust.sh`, and `git diff --check` across the hardening branches. Current Dependabot alerts observed during this pass were fixed or dismissed. PR #29 adds an explicit npm build-chain audit gate for high and critical dev/build-tooling findings. PR #31 adds browser/demo IPC fallback contract tests for the commands used by visual smoke. PR #35 adds Rust unit coverage for backend storage validation and CSV export edge cases. PR #36 adds fail-closed contract tests for the Tauri API surface guard. PR #41 adds Rust coverage for stored credential normalization. PR #42 adds Rust coverage for saved dataset load validation. PR #43 adds Rust coverage for saved dataset record-count conversion. PR #44 adds Rust coverage for non-finite PV inputs. PR #45 adds Rust coverage for non-finite PVWatts inputs. PR #46 adds Rust coverage for NASA POWER parameter-name normalization. PR #47 adds Rust coverage for EUMETSAT query time validation. PR #48 adds Rust coverage for EUMDAC command-output secret redaction. | Rerun the full protected-check set after billing is fixed. Merge and rerun the npm build-chain audit policy from PR #29. Keep RustSec allowances narrow and documented. |

## Open Pull Request Queue

These pull requests represent the current production-hardening queue and should be reviewed, merged, and revalidated rather than duplicated on `main`:

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

## External Blockers

Production completion is currently blocked by issues outside the local source tree:

- GitHub Actions jobs cannot start because account billing or spending-limit state blocks runners: #28.
- Apple Developer ID signing and notarization secrets are not configured: #27.
- Windows Authenticode signing is not configured, and native Windows 10/11 install, uninstall, visual, SmartScreen, and first-launch QA has not been completed: #25.
- Live EUMETSAT and PVWatts credentials are not available for packaged-app validation: #26.

## Production Sign-Off Gate

Do not mark the goal complete until all of these are true:

- All hardening PRs that are still relevant are merged through protected `main` checks.
- Required checks pass on `main`: Ubuntu verify, macOS verify, Windows verify, visual smoke, RustSec audit, macOS DMG build, and Windows MSI/NSIS build.
- A signed and notarized macOS DMG passes `codesign`, `spctl`, `stapler`, and first-launch checks.
- Signed Windows MSI and NSIS installers pass clean Windows 10/11 install, launch, uninstall, Authenticode, and SmartScreen checks.
- Packaged EUMDAC sidecars are checksum-manifested, signed where required, and validated with real credentials.
- Live NASA POWER, PVWatts, EUMETSAT, NDVI sample, saved-data, export, API-slot, and settings workflows pass on packaged builds.
- Release workflow publishes the expected assets and `SHA256SUMS.txt` from a protected tag with required secrets configured.
