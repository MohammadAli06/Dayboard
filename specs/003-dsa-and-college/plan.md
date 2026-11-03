# Implementation plan

## Constitution check

Local SQLite only; typed command/response contracts; one transaction per mutation (including generated tasks and audit events); ULIDs, UTC, explicit IANA timezone; soft deletion; no runtime network. Pass.

## Files and responsibilities

- `crates/dayboard-core/src/model.rs`: `DsaProblem`, `DsaStatus`, `DsaDifficulty`, `TopicCoverage`, `Subject`, `SyllabusItem`, `Deadline`; task `dsa_problem_id`; commands, responses and snapshot fields.
- `crates/dayboard-core/src/schema_v3.sql`: additive tables and indexes.
- `crates/dayboard-core/src/lib.rs`: migration registration; entity loaders/persistence; solve/review SRS engine; topic derivation; subject/syllabus/deadline commands; single-transaction revision and back-scheduling writes.
- `crates/dayboard-core/tests/core.rs`: SRS intervals, task supersession, atomic writes, topic coverage, syllabus, back-scheduling, migration.
- `src/lib/api.ts`, `src/App.svelte`, `src/style.css`: Study view, review controls, heatmap, subjects/syllabus, deadlines.
- `docs/schemas/*`, `README.md`, `specs/003-dsa-and-college/*`: generated schemas and status.

## Design

New entities are typed JSON rows with a `deleted` projection, matching projects/goals. `Task.dsa_problem_id` is optional with `#[serde(default)]`, so existing rows keep deserializing. SRS is a pure function: stage selects the base interval (1/3/7/21) and confidence adjusts it deterministically (1–2 shorter, 3 unchanged, 4–5 longer). Solving/reviewing loads the problem and its open revision tasks, builds the next revision task, and writes problem + superseded tasks + new task + audit events in one transaction. Back-scheduling is a pure function of the deadline, today and timezone; the deadline row and generated tasks commit together. Topic coverage is computed at snapshot time from live problems.

## Validation

Rust unit/integration tests for intervals, confidence adjustment, double-solve rejection, revision supersession, topic coverage, syllabus toggles, back-scheduling windows (including past deadlines), and migration/reopen. Frontend type check, lint, DOM tests and production build. Native UI Automation smoke flow for creating/solving a problem, adding a subject/syllabus item and a deadline, checking the generated tasks and responsiveness. Schemas regenerated from types.
