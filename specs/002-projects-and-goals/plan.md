# Implementation plan

## Constitution check

Local SQLite only; typed command enum and snapshot response; transactional append-only events; ULIDs and UTC; soft deletion with dry-run where destructive; no runtime network. Pass.

## Files and responsibilities

- `crates/dayboard-core/src/model.rs`: `Project`, `Goal`, `Template`, `SavedFilter`, `GoalProgress`, new task link fields, commands and responses.
- `crates/dayboard-core/src/schema_v2.sql`: additive migration for the four new JSON tables.
- `crates/dayboard-core/src/lib.rs`: migration registration, entity persistence helpers, project/goal/template/filter commands, issue-URL parser, goal progress, snapshot loading.
- `crates/dayboard-core/tests/core.rs`: CRUD, soft delete, progress, template expansion, issue import, migration/reopen.
- `src-tauri/src/main.rs`: `open_in_vscode` command using the OS URL handler (no shell interpolation).
- `src/lib/api.ts`, `src/App.svelte`, `src/style.css`: typed contracts, Projects view, board, goal progress, templates, saved filters, editor link fields.
- `docs/schemas/*`, `README.md`, `specs/002-projects-and-goals/*`: generated schemas and status.

## Design

Projects/goals/templates/filters are typed JSON rows (`data` column) with a `deleted` projection, mirroring tasks. Task JSON gains optional `project_id` and `goal_id` with `#[serde(default)]`, so existing rows deserialize unchanged. Goal progress is derived at snapshot time from live tasks, never stored. New commands follow the existing pattern: validate → load/construct → one transaction (row + event) → typed response. Issue URLs are parsed with a literal regex in core; no HTTP client is added. VS Code is opened with `vscode://file/<absolute path>` through the platform handler (`rundll32`, `open`, `xdg-open`) as a separate process argument, avoiding shell injection.

## Validation

`cargo fmt --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -D warnings`; frontend `npm test`, `npm run check`, `npm run lint`, `npm run build`; schema generation; native launch plus project/template/goal/filter smoke flow with UI Automation. A pre-existing database must reopen and gain the new tables without data loss.
