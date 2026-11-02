# Daybridge Constitution

Version 1.0.0 | Ratified 2026-11-02

1. Local first: SQLite is authoritative. No accounts, telemetry, network calls or AI dependency in normal operation.
2. One command layer: Rust owns validation, parser, scheduling, persistence and capacity. UI and future adapters share typed JSON schema contracts.
3. Durable and explainable: mutations and append-only events are atomic. IDs are ULIDs, timestamps UTC, calendar decisions use an explicit IANA timezone. Destructive operations offer dry-run and soft deletion.
4. Quiet and accessible: keyboard capture and actions, visible focus, dark default, one accent. Errors never silently discard input.
5. Resource discipline: event driven reminders, no idle polling, separate optional future bridge. Performance budgets remain targets until measured.
6. Public by design: fork-and-run documentation, MIT license, lockfiles, safe ignore rules, no personal data or required credentials.
7. Spec before code: acceptance scenarios, implementation plan and tracked tasks accompany every feature. No false completion claims.

Amendments must explain rationale and update affected specs and tests.
