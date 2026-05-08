# Packaging Guide

This project is cross-platform by design, but packaging must be performed on native target systems.

Current verified state:

- macOS Apple Silicon: built and locally verified before this hardening pass; current script is checksum-producing and architecture/version agnostic, and the manual `macOS package` workflow produces private-review DMG/checksum artifacts.
- Windows: GitHub Actions packaging has produced MSI/NSIS/checksum artifacts from this branch and runs MSI quiet install/uninstall smoke on pull requests; real install/uninstall QA still requires Windows 10/11 machines.

## Build-Time Requirements

End users do not need these tools. They are only required to build the app from source.

| Tool | Version/Requirement |
| --- | --- |
| Node.js | `.node-version` pins `24.13.0`; `package.json` allows `>=22.12.0` |
| npm | `packageManager` pins `npm@11.6.2`; `package.json` allows `>=11.0.0` |
| Rust | `rust-toolchain.toml` pins `1.95.0` with `clippy` and `rustfmt` |
| Tauri CLI | `@tauri-apps/cli` pinned in `package-lock.json` |
| macOS | Xcode Command Line Tools, `codesign`, `hdiutil`; Developer ID credentials for public release |
| Windows | Windows 10/11, MSVC Rust toolchain, C++ Build Tools/Windows SDK, code-signing certificate |

macOS Homebrew setup used for this workstation:

```bash
brew install node rustup
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
rustup default 1.95.0
rustup component add clippy rustfmt
npm ci
```

## Verification Before Packaging

Run:

```bash
./scripts/verify.sh
./scripts/audit-rust.sh
npm run visual:smoke
```

This fails fast if Node, npm, or Cargo is unavailable, then runs:

```text
npm run version:check
npm run typecheck
npm run test
npm run build
cargo fmt --all -- --check
cargo test --workspace --locked
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm audit --omit=dev
```

`./scripts/audit-rust.sh` requires `cargo-audit`:

```bash
cargo install cargo-audit --locked --version 0.22.1
```

The GitHub RustSec audit workflow and release workflow gate install `cargo-audit` 0.22.1 and run the same script. The current automated npm audit intentionally checks production dependencies only. Dev dependency audit should be reviewed before release as a separate policy decision.

`npm run visual:smoke` builds the app, starts `vite preview`, and captures the `dashboard`, `power`, `eumetsat`, `ndvi`, `pv`, `saved`, `api`, `settings`, and `about` screens at 1024x720, 1280x853, and 1440x900 under `output/visual-smoke/`. CI uploads those screenshots as the `visual-smoke` artifact.

## macOS Local Review Build

Build:

```bash
./scripts/build-macos.sh
```

Expected outputs:

```text
target/release/bundle/macos/Satellite Data Toolkit.app
target/release/bundle/dmg/Satellite Data Toolkit_2.1.1_aarch64.dmg
```

The script:

- installs npm dependencies with `npm ci`;
- runs full verification;
- runs `tauri build --bundles app,dmg`;
- applies ad-hoc signing to the `.app`;
- verifies with `codesign --verify --deep --strict --verbose=2`;
- rebuilds the DMG with the `.app` and an `/Applications` symlink;
- verifies the DMG with `hdiutil verify`;
- writes a `.sha256` checksum next to the DMG.

The `macOS package` GitHub workflow runs this script on `macos-latest` for pull requests and manual dispatches. It uploads `macos-dmg` and `macos-sha256sum` artifacts for private review and does not bypass the public-release signing/notarization requirements below.

Current limitation: without Apple Developer ID secrets, the local build is ad-hoc signed and Apple Silicon only (`aarch64`). It is suitable for private review, not public distribution.

## macOS Public Release Checklist

For a public DMG:

1. Build on a clean macOS runner.
2. Sign the app and all nested binaries with an Apple Developer ID Application certificate.
3. Enable hardened runtime.
4. Sign any sidecars, including EUMDAC.
5. Notarize the app/DMG with Apple.
6. Staple the ticket.
7. Verify:

```bash
codesign --verify --deep --strict --verbose=2 "Satellite Data Toolkit.app"
spctl --assess --type execute --verbose=4 "Satellite Data Toolkit.app"
xcrun stapler validate "Satellite Data Toolkit.app"
hdiutil verify "Satellite Data Toolkit_2.1.1_aarch64.dmg"
spctl --assess --type open --verbose=4 "Satellite Data Toolkit_2.1.1_aarch64.dmg"
```

8. Test first launch from a clean user profile with no internet.
9. Test NASA fetch, save, export, API slots, PV local estimate, and NDVI GeoTIFF samples with CRS/geotransform tags, `GDAL_NODATA`, and compressed TIFF input.
10. Test app data removal and uninstall behavior.

Known current macOS public-release gaps:

- Developer ID certificate import is wired for CI, but no certificate secret is configured;
- notarization credential plumbing is wired for Apple ID/app-password or App Store Connect API key, but no credential secrets are configured;
- no stapled ticket;
- no signed EUMDAC sidecar;
- no Intel/universal build validation.

## Windows Build

Run only on Windows 10/11 with the MSVC Rust toolchain:

```powershell
.\scripts\build-windows.ps1
```

Expected outputs:

```text
target\release\bundle\msi\
target\release\bundle\nsis\
target\release\bundle\SHA256SUMS.txt
```

The Tauri config currently enables:

```json
"targets": ["app", "dmg", "msi", "nsis"],
"windows": {
  "webviewInstallMode": {
    "type": "embedBootstrapper"
  }
}
```

This is acceptable for a normal online installer flow. If offline install is required, switch to the appropriate fixed/runtime WebView2 strategy and test on a clean Windows image. Windows signing is skipped unless `WINDOWS_SIGN_COMMAND` is set. When it is set, `scripts/build-windows.ps1` injects a temporary Tauri `signCommand` config that calls `scripts/sign-windows.ps1`. The wrapper sets `WINDOWS_SIGN_FILE` to the file Tauri asked to sign, and also supports `{file}` or `%1` placeholders for signing providers that require positional substitution.

The Windows package workflow runs on pull requests and manual dispatches. It and the release workflow run `scripts\smoke-windows-msi.ps1` after packaging. The smoke script reads ProductCode/ProductName from the MSI, performs a quiet install on the Windows runner, verifies the uninstall registry entry, performs a quiet uninstall, and uploads install/uninstall logs as workflow artifacts.

## Windows Release Checklist

Before sending a Windows build externally:

1. Build MSI and NSIS on Windows 10/11.
2. Install MSI on a clean machine.
3. Launch app without Node, Rust, Python, or npm installed.
4. Verify first-run offline behavior: UI opens; API-dependent actions fail clearly.
5. Verify live NASA POWER fetch.
6. Verify save/preview/delete/export.
7. Verify API Slots use Windows Credential Manager.
8. Verify local PV estimate.
9. Verify PVWatts with a real `nlr_pvwatts_key`.
10. Verify EUMDAC sidecar search/download with real EUMETSAT credentials.
11. Uninstall MSI and confirm app files are removed.
12. Repeat install/uninstall for NSIS.
13. Authenticode sign MSI/NSIS and verify:

```powershell
Get-AuthenticodeSignature .\path\to\installer.exe
Get-AuthenticodeSignature .\path\to\installer.msi
```

Current Windows status remains: CI has produced MSI/NSIS/checksum artifacts, has signing-command plumbing, and includes MSI quiet install/uninstall smoke on the Windows runner. Native Windows 10/11 install/uninstall, NSIS install/uninstall, Authenticode certificate configuration, and SmartScreen QA are still required before public distribution.

## GitHub Release Workflow

The `Release` workflow runs on `v*` tags or manual dispatch with an existing tag. It first runs `scripts/check-release-secrets.sh` and refuses to publish a public release unless Windows Authenticode signing plus macOS Developer ID signing/notarization secrets are configured. After that gate passes, it builds the Windows MSI/NSIS installers and macOS DMG, downloads all build artifacts into a publish job, creates `SHA256SUMS.txt`, and uploads all assets to the matching GitHub release.

Optional release signing secrets:

| Secret | Purpose |
| --- | --- |
| `WINDOWS_SIGN_COMMAND` | PowerShell command used by `scripts/sign-windows.ps1`; reference the target file through `$env:WINDOWS_SIGN_FILE`, `{file}`, or `%1`. |
| `APPLE_CERTIFICATE` | Base64-encoded Developer ID `.p12` certificate. |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the exported `.p12`. |
| `KEYCHAIN_PASSWORD` | Temporary CI keychain password. |
| `APPLE_SIGNING_IDENTITY` | Optional explicit signing identity; if omitted, CI chooses an imported Developer ID identity. |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | Apple ID notarization credential path. |
| `APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_P8_BASE64` | App Store Connect API notarization credential path. |

When `APPLE_SIGNING_IDENTITY` is set and notarization credentials are present, `./scripts/build-macos.sh` requires `xcrun stapler` and Gatekeeper checks to pass. Without signing secrets, use `./scripts/build-macos.sh` locally, the `macOS package` workflow, or the manual `Windows package` workflow for private-review artifacts; the public `Release` workflow is intentionally blocked.

As of May 8, 2026, GitHub also contains a separate `rust-pro-v3.0.0` release from the `codex/rust-pro-windows-exe` branch. Treat that as a separate portable Rust-only artifact line. Public Tauri app releases should use `v*` tags and be promoted as latest after macOS and Windows artifacts are attached.

## EUMDAC Sidecar Packaging

The app currently detects sidecars named:

```text
eumdac
eumdac.exe
eumdac-cli
eumdac-cli.exe
```

next to the packaged executable.

For production:

1. Choose the exact EUMDAC release and source.
2. Record version, checksum, source URL, and license.
3. Place platform binaries under `src-tauri/binaries/` using Tauri's target-triple naming, for example `src-tauri/binaries/eumdac-aarch64-apple-darwin` or `src-tauri/binaries/eumdac-x86_64-pc-windows-msvc.exe`.
4. Add the base sidecar path to `src-tauri/tauri.conf.json > bundle.externalBin`, for example `"binaries/eumdac"`.
5. Sign/notarize the sidecar with the app.
6. Validate auth flow with real `eumetsat_consumer_key` and `eumetsat_consumer_secret`.
7. Validate these CLI shapes against the bundled EUMDAC version:

```text
eumdac set-credentials <consumer_key> <consumer_secret>
eumdac search -c <collection> -s <start> -e <end> --bbox <west> <south> <east> <north> --limit <n>
eumdac download -c <collection> -p <product> -o <output_dir>
```

Important: the app reads both EUMETSAT keychain slots and syncs them to EUMDAC immediately before search/download using an app-scoped EUMDAC config environment. Confirm this behavior against the exact sidecar binary. Keep secrets out of logs and review whether EUMDAC persists credentials in any sidecar-managed config file.

Sidecar checksum trust is enforced by a manifest placed next to the packaged executable:

```json
{
  "binaries": [
    {
      "name": "eumdac",
      "sha256": "<platform-binary-sha256>",
      "version": "3.x.y",
      "source": "https://pypi.org/project/eumdac/",
      "license": "<license>"
    }
  ]
}
```

Use `eumdac.exe` as the `name` on Windows when that is the packaged file name. Search/download commands reject an unmanifested or checksum-mismatched sidecar. Local development can bypass that gate only with `SATELLITE_ALLOW_UNVERIFIED_EUMDAC=1`.

## Artifact Handoff

The review ZIP should include:

```text
README.md
docs/
scripts/
src/
src-tauri/
crates/
Cargo.toml
Cargo.lock
package.json
package-lock.json
tsconfig*.json
vite.config.ts
index.html
rust-toolchain.toml
.node-version
artifacts/macos/Satellite Data Toolkit_2.1.1_aarch64.dmg
artifacts/macos/SHA256SUMS.txt
artifacts/visual/
```

It should exclude:

```text
node_modules/
target/
dist/
.playwright-cli/
*.zip from older handoffs
local screenshots
```
