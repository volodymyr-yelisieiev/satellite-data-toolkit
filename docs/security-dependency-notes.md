# Security Dependency Notes

## RustSec Warning Policy

`scripts/audit-rust.sh` runs `cargo audit` and treats vulnerability findings as
failures while allowing RustSec warnings from upstream transitive dependencies.
Do not switch this repository to `cargo audit --deny warnings` until the upstream
Tauri dependency chain below can resolve those warning-only advisories without a
local fork or risky `[patch.crates-io]` override.

## Linux-only GTK3 alerts

GitHub and RustSec may report `glib`, `proc-macro-error`, or unmaintained GTK3
bindings in `Cargo.lock` because Tauri's Linux webview stack pulls
`gtk`/`webkit2gtk` through `wry`. The production release targets for this app are
macOS and Windows, and those bundles do not load the Linux GTK stack.

As of Tauri `2.11.1` and Wry `0.55.1`, the latest compatible dependency chain still
requires `gtk = ^0.18`, which requires `glib = ^0.18`. Attempting to force `glib`
to `0.20.0` fails dependency resolution. Keep these alerts classified as
transitive Linux-only dependencies until Tauri/Wry publish a compatible GTK stack
using `glib >= 0.20`.

## Tauri URLPattern alerts

RustSec may also report unmaintained `unic-*` crates through:

```text
tauri-utils 2.9.1 -> urlpattern 0.3.0 -> unic-ucd-ident 0.9.0
```

`cargo update -p urlpattern --dry-run` does not move the lockfile because the
current Tauri constraint still selects `urlpattern 0.3.0`; crates.io has newer
`urlpattern` releases, but they are outside Tauri's accepted range. Keep these as
upstream Tauri transitive warnings and recheck after every Tauri runtime update.
