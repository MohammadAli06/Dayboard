# Implementation plan

## Constitution check

Local-only; no new network or cloud dependency; settings and writes stay in core; platform integration is limited to an explicit opt-in autostart entry. Pass.

## Files and responsibilities

- `crates/dayboard-core/src/model.rs`: `autostart` and `onboarded` settings defaults.
- `crates/dayboard-core/examples/bench.rs`: isolated benchmark for open + snapshot latency.
- `src-tauri/src/main.rs`: idempotent platform autostart writers and application on settings save/startup.
- `src/App.svelte`, `src/style.css`: autostart checkbox and first-run welcome card.
- `src-tauri/tauri.conf.json`: installer metadata.
- `docs/releasing.md`, `README.md`, `specs/006-packaging-polish/*`: release/signing/updater guidance and status.

## Design

Autostart is implemented without a new dependency: `reg.exe` for the Windows per-user Run key, a LaunchAgent plist on macOS and an XDG desktop entry on Linux. `apply_autostart` is called after a successful `save_settings` and at startup; development builds log and return without touching the machine. The welcome card uses `settings.onboarded` and the existing `save_settings` command. The benchmark uses a temporary SQLite file, creates a fixed number of tasks through the real command layer, then time-boxes snapshots with `std::time::Instant`.

## Validation

`cargo fmt`, `cargo clippy -D warnings`, workspace tests; frontend checks/tests/build; run `npm run bench:core` and record p50/p95/open on this machine; native launch checks the welcome card and that enabling autostart in a dev build writes no registry entry. Installer/signing/updater claims remain deferred until credentials and a clean VM are available.
