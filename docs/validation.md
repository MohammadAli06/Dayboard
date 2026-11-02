# Validation record

Validation performed on Windows on 2026-11-02. This records observed results, not an assertion that every platform acceptance criterion is complete.

## Automated checks

- Rust core: 15 integration tests, including a 36-case valid parser table and invalid/date/DST cases. Covers atomic rollback when audit insertion fails, append-only event protection, soft-delete/dry-run/restore, persistence/reopen, FTS search, parent cycles, timezone rollover, capacity, planner and reminders.
- Frontend: 11 tests (7 DOM interaction tests and 4 date tests). DOM tests use a mocked Tauri transport; they verify capture, command errors, completion and task-edit interactions. Date-format tests cover local/UTC boundaries. These are not native end-to-end tests.
- Svelte/TypeScript checking: no errors or warnings. ESLint passes.
- Production frontend bundle builds. No remote fonts/assets or browser persistence fallback.
- `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and Rust formatting pass on Windows.
- JSON command/response schemas generated from the Rust types.
- Official `specify check` succeeds; Spec Kit prerequisite checker recognizes the active feature's plan, data model, quickstart and tasks.
- `npm ci --dry-run --ignore-scripts` confirms package/lockfile consistency for a clean install.
- Ignore-rule probes exclude example env files, private agent state and SQLite/WAL files while retaining shared Spec Kit skills. A credential-pattern scan of publishable text files found zero matches; this is a basic check, not a guarantee against every kind of sensitive content.

## Native validation

A Windows debug build with embedded frontend assets compiled and launched. Its process exposed a `Daybridge` window, reported responsive, and created the SQLite database in an isolated temporary data directory. That test process was stopped afterward.

Interactive native verification could not run: both computer-use interfaces reported `Computer Use native pipe is unavailable ... The system cannot find the file specified. (os error 2)`. Therefore actual visual appearance, native quick-add, drag/drop, tray badge and OS notification delivery remain manual acceptance items. DOM tests cover frontend behavior but do not substitute for these checks.

A Windows x64 NSIS release installer was built successfully with `npm run tauri build -- --bundles nsis` at `target/release/bundle/nsis/Daybridge_0.1.0_x64-setup.exe` (about 2.08 MiB). This verifies packaging and the installer-size target for this build; the installer has not been installed on a clean VM. It is unsigned.

## Dependency audit

`npm audit --omit=dev`: zero findings. The complete development dependency graph reports 14 findings (2 low, 12 moderate) propagated from `deepmerge` and `minimatch` in tooling. Critical findings in the initial test-runner dependency were removed by upgrading Vitest. No unsupported major transitive override was forced just to silence the audit. Review upstream fixes through Dependabot before exposing development servers or processing untrusted build inputs.

## Outstanding acceptance

- Cross-platform GitHub Actions runs (workflow added but not run remotely), clean VM install/uninstall, macOS signing/notarization and Windows signing.
- Real native keyboard/tray/notification and display-scaling checks using the feature quickstart.
- Native webview suspension, including platform differences; hidden UI currently avoids periodic JS work.
- Full-process-tree RAM, idle CPU, cold start and query p95 audit. The original <60 MB / <1s / <10ms budgets are unverified targets.
- Large archives: list DOM is bounded, but snapshots still load all tasks. Database pagination remains future performance work.
- Features from phases 3–7, including the actual Jarvis bridge, are not implemented in this release.
