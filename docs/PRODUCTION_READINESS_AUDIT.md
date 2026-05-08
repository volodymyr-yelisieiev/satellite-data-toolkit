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
| Visual and UX consistency | `npm run visual:smoke` covers dashboard, NASA POWER, EUMETSAT, NDVI, PV, saved data, API slots, settings, and about screens at 1024x720, 1280x853, and 1440x900. PR #17 tightens clipped-control detection. PRs #19-#23 remove fake settings behavior, stale refresh states, duplicate EUMETSAT actions, misleading download status, and static shell status. | Merge and rerun required CI. Perform native Windows 10/11 visual QA with actual DPI/scaling. Recheck macOS after signed/notarized packaging. |
| macOS packaging | `./scripts/build-macos.sh` builds and verifies a local `.app` and DMG. PR #16 cleans stale artifacts, stages pinned EUMDAC sidecars, verifies sidecar hashes, signs nested sidecars when configured, refreshes manifests after signing, and wires notarization/stapling flow. | Configure Apple Developer ID and notarization secrets. Verify Gatekeeper acceptance, stapled ticket, Intel or universal build behavior, and first-launch behavior from a clean profile. |
| Windows packaging | Existing Windows workflows build MSI/NSIS artifacts and run MSI quiet install/uninstall smoke. PR #16 adds pinned EUMDAC sidecar staging and packaged sidecar hash/signature verification. | GitHub Actions billing currently prevents current Windows jobs from starting. Validate MSI and NSIS on real Windows 10/11 machines, including launch without dev tools, uninstall cleanup, Authenticode signature, and SmartScreen behavior. |
| Backend and functionality | NASA POWER, local PV estimate, PVWatts/NLR call path, NDVI processing, saved datasets, exports, API slots, and EUMETSAT sidecar commands are covered by existing code and tests. PR #15 hardens PVWatts error redaction and input validation. PR #19 applies request timeout settings to live calls. PRs #20-#22 harden saved-data and EUMETSAT flows. | Run live PVWatts with a real `nlr_pvwatts_key`. Run EUMETSAT search/download with real credentials and packaged sidecar. Add broader real-world NDVI fixture coverage if the product scope requires more GeoTIFF variants. |
| DRY, KISS, and architecture | Existing split between React UI, Tauri command layer, and Rust core is clear enough for production maintenance. PR #19 centralizes timeout clamping. PR #16 centralizes sidecar preparation. PR #14 centralizes footer release links. PR #18 hardens the RustSec audit tool lookup. | `src/App.tsx` remains large and should be decomposed only where it reduces concrete maintenance risk. Avoid broad refactors until the current hardening PR queue is merged. |
| GitHub, CI, and CD | Branch protection requires Ubuntu/macOS/Windows verify jobs, visual smoke, RustSec audit, macOS DMG, and Windows MSI/NSIS packaging. Workflows exist for CI, macOS package, Windows package, Rust audit, and release publishing. Dependabot and security policy are present. | Required checks are blocked before runner startup by GitHub billing/spending-limit state. Release secrets are not configured. Merge queue cannot prove the current hardening changes until those external blockers are removed. |
| Tests and security | Local verification has covered `npm run verify`, `npm run visual:smoke`, targeted Rust tests, `./scripts/audit-rust.sh`, and `git diff --check` across the hardening branches. Current Dependabot alerts observed during this pass were fixed or dismissed. | Rerun the full protected-check set after billing is fixed. Decide and document dev-dependency audit policy before public release. Keep RustSec allowances narrow and documented. |

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
