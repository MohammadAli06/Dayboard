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

### Interactive validation (2026-11-02)

Native UI Automation drove the debug app against an isolated data directory containing a pre-existing Phase 0–2 database. The app migrated it in place (`user_version` 1 → 2) without losing tasks. The run created a project, a goal, an imported GitHub issue, a template and a saved filter; linked a task to the project and goal; created another task from the template; and rendered the board, goal progress and template rows. The process reported `Responding = true` after every step. A separate stress run added five past-due tasks in a row to exercise the reminder/tray path after the IPC concurrency fix; the app stayed responsive and persisted all of them. The test process and dev server were stopped afterwards; the repository and type checks listed above were re-run after Phase 3.

### Phase 4 native validation (2026-11-03)

The same harness migrated the existing smoke database in place (`user_version` 2 → 3). It added a DSA problem, logged it as solved (stage 1, next review the following day, one linked revision task), then reviewed it (stage 2, a 3-day interval, the old revision completed and a new one created in one transaction). It added a subject and a syllabus item and toggled the item done. It created a deadline; the native date input required keyboard entry (Chromium ignores UIA value writes), and the resulting past date correctly produced a single same-day study task. The process stayed responsive throughout. Multi-day back-scheduling (4 days, 120 minutes → four 30-minute tasks before the deadline) is covered by the Rust integration test.

### Phase 5 native validation (2026-11-03)

The same harness migrated the smoke database in place (`user_version` 3 → 4) and confirmed the rolling startup backup (`backups/daybridge-2026-11-03.sqlite3`). It created a habit, logged it, saved an end-of-day review, started and stopped a focus session linked to a task, exported JSON and Markdown (`exports/`), and imported the JSON export back; the import reported zero inserts and 23 skipped existing records, as designed. A recurring task (daily rule) was completed and its next occurrence was created for the following day with the rule preserved, in the same transaction. The process stayed responsive throughout. A 3-second focus session correctly records zero whole minutes; the minute rollup is covered by the Rust test with a 30-minute session.

## Dependency audit

`npm audit --omit=dev`: zero findings. The complete development dependency graph reports 14 findings (2 low, 12 moderate) propagated from `deepmerge` and `minimatch` in tooling. Critical findings in the initial test-runner dependency were removed by upgrading Vitest. No unsupported major transitive override was forced just to silence the audit. Review upstream fixes through Dependabot before exposing development servers or processing untrusted build inputs.

## Outstanding acceptance

- Cross-platform GitHub Actions runs (workflow added but not run remotely), clean VM install/uninstall, macOS signing/notarization and Windows signing.
- Real native keyboard/tray/notification and display-scaling checks using the feature quickstart.
- Native webview suspension, including platform differences; hidden UI currently avoids periodic JS work.
- Full-process-tree RAM, idle CPU, cold start and query p95 audit. The original <60 MB / <1s / <10ms budgets are unverified targets.
- Large archives: list DOM is bounded, but snapshots still load all tasks. Database pagination remains future performance work.
- Features from phases 3–7, including the actual Jarvis bridge, are not implemented in this release.
