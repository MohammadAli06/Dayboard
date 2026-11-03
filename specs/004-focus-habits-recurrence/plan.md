# Implementation plan

## Constitution check

Local SQLite and filesystem only; typed commands; transactional writes and audit events; ULIDs/UTC/IANA timezone; soft deletion; no network. Pass.

## Files and responsibilities

- `crates/dayboard-core/src/model.rs`: focus, habit, review, stats and export types; task `recurrence_rule`; commands, responses and snapshot fields.
- `crates/dayboard-core/src/schema_v4.sql`: additive focus sessions, habits, habit logs and daily reviews.
- `crates/dayboard-core/src/lib.rs`: migration 4, focus start/stop, habit logs and streak derivation, recurrence parsing/next-occurrence, daily reviews, weekly stats, JSON/Markdown export, JSON import and SQLite backup.
- `crates/dayboard-core/tests/core.rs`: focus actual-minute rollup, single-open-session, streaks/duplicates, each recurrence rule, review upsert, stats, export/import round-trip and migration.
- `src-tauri/src/main.rs`: state data directory; `export_data`, `import_data` commands; rolling backup at startup.
- `src/lib/api.ts`, `src/App.svelte`, `src/style.css`: Insights view (focus timer, habits, stats, review, data), recurrence selector.
- `docs/schemas/*`, `README.md`, `specs/004-focus-habits-recurrence/*`: generated schemas and status.

## Design

Focus sessions are rows with a nullable end; stop computes elapsed minutes and rolls them into the linked task's `actual_min` inside one transaction. Streaks are derived from habit logs (daily consecutive with a one-day grace for today). Recurrence is stored as a normalized rule string on the task; completion builds the next task in the same transaction as the completion events. Reviews are one row per local day, upserted. Stats are computed per snapshot from tasks and completed focus sessions. Export is a versioned JSON bundle built from all rows (including soft-deleted); import inserts only missing IDs in one transaction. Backups use rusqlite's online backup API to a dated file, pruned to seven.

## Validation

Rust tests for focus rollup and transactionality, habit streaks and duplicate rejection, each recurrence rule (including a short month), review upsert, stats, export/import round-trip with ID collisions, and migration. Frontend checks and DOM tests for the Insights view. Native UI Automation smoke: start/stop focus with a task, log a habit, save a review, run an export, and verify the created file plus responsiveness. Backups are verified by launching twice on different dates is impractical; the backup helper and prune logic are unit-tested.
