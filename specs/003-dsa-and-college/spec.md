# Feature: DSA spaced repetition and college prep

Status: implemented and natively smoke-tested | Spec 003 | Phase 4 of the build plan

## Scope

Implement Phase 4 locally, with no network access in normal operation:

- DSA problem tracker: title, optional URL, topic, difficulty (easy/medium/hard), platform, notes, confidence (1–5), solved state and review stage.
- Deterministic spaced repetition: solving or reviewing a problem schedules the next revision at 1, 3, 7 and 21 days, adjusted by confidence, and atomically creates a revision task linked to the problem.
- Topic coverage summary (total, solved, average confidence) for a weak-spot heatmap.
- Subjects with a syllabus checklist.
- Deadlines/exams with a countdown and back-scheduled study tasks.

Phases 5–7 remain deferred until they have their own specifications.

## User stories and acceptance

1. Problems (P1): create, edit and soft-delete a DSA problem. Solving it records confidence and solved time, moves it to solved, and creates exactly one open revision task on the next review date. Relaunching preserves both.
2. Spaced repetition (P1): solving at stage 1 schedules +1 day; each review advances the stage and uses the 1/3/7/21-day intervals. Confidence adjusts the interval (lower confidence shortens, higher extends). A review completes the previous open revision task and creates the next one in the same transaction.
3. Coverage (P1): a topic summary reports total, solved and average confidence; topics are derived, never stored.
4. Subjects and syllabus (P2): create subjects, add checklist items, toggle them done, soft-delete items/subjects. Subject progress is `done / total`.
5. Deadlines (P2): create a deadline under an optional subject with a due date and total study minutes. The app creates daily study tasks between today and the due date (window capped at 14 days, minutes split evenly) so that the work is scheduled before the deadline. A deadline due today creates one task today.
6. Validation (P2): reject empty/over-long titles, invalid confidence, empty topics, impossible dates and missing subjects. Every write records an audit event.

## Edge cases

Confidence is clamped to 1–5; an invalid value is rejected. Reviewing an unsolved problem or solving twice is rejected with a clear message. After stage 4 the interval stays at 21 days. Deleting a problem leaves its revision tasks (soft delete only). Back-scheduling never creates more than 14 tasks and never schedules a task after the deadline; a past or same-day deadline yields one task for today. Study/revision tasks carry `!study`/`!revise` tags and are ordinary tasks afterwards.

## Non-functional requirements

All validation and writes stay in `dayboard-core`; the UI only invokes typed commands. Revision and study task creation, the problem/deadline update and their audit events commit atomically. ULIDs, UTC timestamps, explicit IANA timezone. No telemetry, accounts, AI call or HTTP request. The additive migration preserves every existing task and Phase 3 record.

## Out of scope

Phase 5 (focus/habits/recurrence/export), Phase 6 (bridge), Phase 7 (updater/autostart). Fetching problem metadata or LeetCode data (would require network).
