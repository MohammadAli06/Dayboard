# Tasks

- [x] T001 Add focus, habit, review, stats and export types; task recurrence field.
- [x] T002 Add migration 4 and persistence/audit helpers.
- [x] T003 Implement focus start/stop with actual-minute rollup.
- [x] T004 Implement habits, logs and derived streaks.
- [x] T005 Implement recurrence parsing and next-occurrence creation on completion.
- [x] T006 Implement daily review upsert and weekly/tag stats.
- [x] T007 Implement JSON/Markdown export, JSON import and SQLite backups.
- [x] T008 Add Tauri export/import commands and rolling startup backup.
- [x] T009 Add the Insights view and recurrence selector.
- [x] T010 Extend Rust/Svelte tests, regenerate schemas, run all checks and a native smoke flow, update docs.

## Manual acceptance still pending

- [ ] A001 Verify restoration into a clean data directory (import is additive; a dedicated restore flow is not implemented).
- [ ] A002 Audit export/backup size and timing on a large archive.
