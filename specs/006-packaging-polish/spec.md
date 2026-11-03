# Feature: packaging, autostart, onboarding and performance

Status: implemented except hosted auto-update/signing (needs owner credentials) | Spec 006 | Phase 7 of the build plan

## Scope

Finish the local release polish that does not require a third-party service or signing key:

- Opt-in autostart on Windows, macOS and Linux. Packaged builds write the OS entry; development builds skip it so they never register a broken dev binary.
- A one-time welcome card with the shortest useful capture/planning steps, persisted through the existing settings command.
- Bundle metadata for installers (publisher, copyright, long description, licence).
- A release guide covering installers, signing/notarization and the exact steps to enable the Tauri updater later.
- A repeatable core benchmark for `Core::open` and snapshot query latency, with measured results recorded.

The hosted auto-updater, code signing and fresh-machine installer acceptance are intentionally deferred: they require the owner's signing identity, update endpoint and a clean VM.

## User stories and acceptance

1. Autostart (P2): a Preferences checkbox requests start-on-login. In a packaged build, enabling writes the platform entry (Windows HKCU Run value, macOS LaunchAgent, Linux XDG autostart file) pointing at the installed executable, and disabling removes it. In a dev build the flag is stored but no OS entry is written, with a visible log note.
2. Onboarding (P3): a new database shows a welcome card on Today with capture, metadata and Preferences tips. Dismissing it stores `onboarded` in settings and it never returns.
3. Packaging (P1): `npm run tauri build` produces installers with publisher/copyright/licence/long description metadata. `docs/releasing.md` documents build, signing, notarization, updater enablement and the fact that auto-update is off by default.
4. Performance (P1): `npm run bench:core` opens a database, inserts a fixed sample and reports min/p50/p95 snapshot latency plus open time. Results are recorded as measurements, not as acceptance of the plan's budgets.
5. Safety (P1): autostart changes are idempotent; removing it does not touch unrelated entries; debug builds never write machine state.

## Edge cases

Registry/file errors surface as a UI error and do not crash the app. The dev-build skip is explicit. Onboarding is shown only on Today and only until dismissed. Existing databases receive the new settings defaults without migration. Autostart removal tolerates a missing entry. Benchmark uses an isolated temporary database.

## Out of scope

Updater network calls, signing keys, notarization credentials, a hosted update manifest, clean-VM install/uninstall, and claiming the original cold-start/RAM/query budgets are met.
