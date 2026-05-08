# Security Policy

## Supported Versions

This project is still in pre-release hardening. Security fixes are applied to the current default development line and the latest Tauri desktop release line once a public `v*` release is published.

| Version | Supported |
| --- | --- |
| Latest `main` / active hardening branch | Yes |
| Latest published `v*` Tauri release | Yes, after the first public Tauri release |
| Older tags, archived prototypes, and one-off review builds | No |

## Reporting a Vulnerability

Please do not open a public issue with exploit details, credentials, logs that contain secrets, or proof-of-concept payloads.

Preferred reporting path:

1. Use GitHub private vulnerability reporting from the repository Security page when it is enabled.
2. Include the affected version or commit, platform, reproduction steps, impact, and any relevant logs with secrets redacted.
3. If private vulnerability reporting is not available, open a public issue that only asks for a private security contact and does not disclose technical details.

Expected handling:

- A maintainer should acknowledge a report within 7 days.
- Valid vulnerabilities should receive a remediation plan or status update within 30 days.
- Public disclosure should wait until a fix is available or a coordinated disclosure timeline is agreed.

## Security Scope

In scope:

- Tauri command exposure, local privilege boundaries, and packaging configuration.
- Credential handling for OS keychain slots and EUMDAC/PVWatts integrations.
- Secret leakage through logs, process errors, exports, or CI artifacts.
- Supply-chain issues in runtime dependencies, bundled sidecars, and release workflows.

Out of scope:

- Vulnerabilities in third-party services such as NASA POWER, EUMETSAT, or NLR PVWatts unless this app mishandles their credentials or responses.
- Reports requiring malware, social engineering, physical access, or compromised user devices.
- Denial-of-service claims that only affect a local user's own machine without data exposure or privilege impact.
