# Releasing Dayboard

Dayboard ships as a Tauri desktop app. This guide covers local builds, installer metadata, signing/notarization, the optional updater and current limitations.

## Build

Prerequisites are listed in [README.md](../README.md). In the repository root:

```sh
npm ci
npm run check
npm run lint
npm test
npm run build
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run tauri build
```

Installers appear under `target/release/bundle/`:

- Windows: NSIS `.exe` and MSI (MSI requires WiX).
- macOS: `.app` and `.dmg`.
- Linux: `.AppImage`, `.deb`, `.rpm` where the packaging dependencies are installed.

The GitHub workflows (`.github/workflows/ci.yml` and `package.yml`) run checks and build artifacts. They do not publish releases or require signing secrets.

## Signing and notarization

Unsigned installers trigger OS warnings. For public distribution:

- **Windows:** sign with an Authenticode certificate and a timestamp URL, either through Tauri's `bundle.windows` certificate options or by signing the generated installer in CI.
- **macOS:** sign with a Developer ID Application identity and notarize with `notarytool`; configure `bundle.macOS.signingIdentity` and the notarization credentials.
- **Linux:** sign packages through the distribution channel you publish to.

Keep keys and passwords in CI secrets, never in the repository.

## Autostart

Enabling **Start Dayboard on login** in Preferences writes, in packaged builds only:

- Windows: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Dayboard`
- macOS: `~/Library/LaunchAgents/dev.dayboard.desktop.plist`
- Linux: `~/.config/autostart/dayboard.desktop`

Development builds skip the OS entry by design, so `npm run desktop` never registers a dev binary. Disabling removes only the Dayboard entry.

## Auto-update (intentionally off by default)

Dayboard does not contact a server. Enabling the Tauri updater requires an owner-provided signing key and update endpoint:

1. Generate keys: `npm run tauri signer generate -- -w ~/.tauri/dayboard.key`; keep the private key in CI.
2. Add `tauri-plugin-updater` (Rust and JS), set `bundle.createUpdaterArtifacts: true`, and add `plugins.updater` with `endpoints` and `pubkey` in `tauri.conf.json`.
3. Publish signed update artifacts (for example to GitHub Releases) and point the endpoint manifest at them.
4. Check for updates from an explicit user action. Do not add startup polling or telemetry.

Without those credentials and the endpoint, update checking stays absent, preserving the local-first and no-runtime-network defaults.

## Current limitations

- The installer has not been validated on a clean VM or on all three operating systems.
- The RAM, cold-start and query budgets are targets, not acceptance claims. See [validation.md](validation.md) for measurements.
- Cross-platform signing/notarization is not configured in this repository.
