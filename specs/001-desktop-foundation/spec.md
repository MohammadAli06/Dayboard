# Feature: Dayboard desktop foundation

Status: implemented foundation slice; manual platform acceptance pending | Spec 001 | Product name: Dayboard

## Scope

Implement phases 0–2 as the first usable release, following the build plan's shipping recommendation. Phases 3–7 are explicitly deferred. No remote bridge, AI calls, updater endpoint or owner-specific service is enabled.

## User stories and acceptance

1. Capture (P1): enter a title with tags, p1–p4, estimates and local dates; save as Inbox or scheduled Todo. Restarting preserves it. Invalid input leaves the capture text intact and shows a useful error.
2. Manage (P1): edit title, notes, priority, energy, estimate, actual minutes, tags and parent; complete, reopen, drop, soft-delete and restore. Every mutation records actor, timestamp, before and after atomically. Destructive dry-run does not mutate.
3. Plan (P1): see Today, Inbox, Week and completed views; schedule by local date/time or drag onto a timeline; see count/minute caps, due and carried badges. Rollover advances unfinished past planned days exactly once per missed calendar day without changing the original due deadline.
4. Capture anywhere (P2): Ctrl/Cmd+Shift+Space opens a compact capture window. Ctrl/Cmd+K opens a palette; j/k select, x completes, e edits, / searches. Native single-instance prevents competing app processes.
5. Reminders (P2): one native timer wakes for the next due reminder; edits rearm it; persisted delivery markers prevent restart duplicates. Close hides to tray; tray opens or quits. Native scheduling continues while the webview is hidden.
6. Fork and run (P1): install documented platform prerequisites, npm ci, npm run desktop. No account or secrets required. CI checks frontend/core and packages on all three desktop OSes.

## Edge cases

Reject empty titles, invalid priorities/durations/timezones, nonexistent local DST times and ambiguous local DST times (ask for another time). Bound list rendering; use literal search terms, never interpolate SQL. A failure rolls back both data and audit. Date-only due values map to 17:00 local; time-only means today. Calendar rollover uses the configured timezone, including missed days. Deleting a parent preserves its children.

## Non-functional requirements

SQLite WAL and foreign keys, FTS5, migrations, ULID identity, UTC timestamps. All domain writes in Rust. No polling or runtime network use. The plan's <60 MB RAM, <1s cold start, <10ms query p95 and <15 MB installer are measurement targets, not acceptance claims until audited.

## Deferred

Projects/goals/templates/saved filters moved to [Spec 002](../002-projects-and-goals/spec.md) and are implemented there. DSA/college moved to [Spec 003](../003-dsa-and-college/spec.md) and is implemented there. Focus/habits/recurrence/review/stats/export/backups moved to [Spec 004](../004-focus-habits-recurrence/spec.md) and is implemented there. The optional bridge/MCP/undo moved to [Spec 005](../005-jarvis-bridge/spec.md) and is implemented there (off by default). Phase 7 polish moved to [Spec 006](../006-packaging-polish/spec.md); autostart, onboarding, packaging metadata and the benchmark are implemented, while hosted auto-update, signing/notarization and clean-VM acceptance remain deferred. Native webview suspension is platform-specific; hidden UI performs no interval work, but OS-level suspension needs a separate measured validation.
