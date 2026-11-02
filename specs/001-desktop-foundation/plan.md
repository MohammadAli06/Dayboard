# Implementation plan

## Constitution check

Local SQLite; typed Rust command enum and response schema; transactional event log; no runtime network; safe public defaults. Pass by design; verify using tests and source review.

## Files and responsibilities

- Cargo.toml, crates/dayboard-core/{src,tests,examples}: migrations, models/contracts, quick-add parser, command dispatcher, planner and tests.
- src-tauri/{src,Cargo.toml,tauri.conf.json,capabilities}: app-data DB, command adapter, tray, single instance, global shortcut, one reminder task and calendar wakeup.
- src/{App.svelte,lib,style.css}: accessible task list, editor, settings, week timeline, keyboard palette and capture surface.
- .github/workflows, .gitignore, README, CONTRIBUTING, SECURITY, LICENSE: reproducible public-repo setup and CI.
- .specify, .agents/skills, specs/001-desktop-foundation: actual Specify CLI integration and tracked decisions.

## Design

Store typed task JSON in tasks alongside indexed projection columns for planned_day, due_at, status and deleted state; tags in normalized tables and FTS titles/notes. Projection updates, FTS changes and audit entries share a transaction. Settings include IANA timezone and caps. UI sends a discriminated command and receives a typed response. A deterministic Planner trait leaves room for a future separate assistant.

## Validation

At least 30 parser cases, transactional rollback, dry-run/delete/restore, search, persistence, timezone/day rollover, planning/capacity and reminder marker tests. Svelte type checking, lint, frontend utility tests and production build. Native workspace check/test/clippy locally where toolchain permits. CI provides other-OS checks; fresh-machine notification and performance acceptance remain manual.
