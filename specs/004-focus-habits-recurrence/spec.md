# Feature: Focus, habits, recurrence, review, stats and data safety

Status: implemented and natively smoke-tested | Spec 004 | Phase 5 of the build plan

## Scope

Implement Phase 5 locally:

- Focus sessions (focus/break) linked to an optional task, with a start/stop timer. Stopping a focus session adds its minutes to the task's actual minutes atomically.
- Daily habits with cadence and current/best streaks.
- Recurrence rules on tasks (`daily`, `weekdays`, `weekly`, `monthly`, `every N days`), including quick-add parsing. Completing a recurring task creates its next occurrence atomically.
- End-of-day review notes, one per day, editable.
- Last-7-days stats: completed, created, carried, focus minutes, estimate accuracy and per-tag focus/completion.
- JSON and Markdown export, JSON import (merge, skip existing IDs) and rolling daily SQLite backups (keep the newest 7).

Phases 6–7 remain deferred.

## User stories and acceptance

1. Focus (P1): start a focus or break session from the Insights view with an optional task and planned minutes. Only one session may be open. Stopping records the end time; a focus session adds its elapsed minutes to the linked task's actual minutes in one transaction. Relaunching preserves sessions.
2. Habits (P1): create habits with a cadence (1–30 days), log them for today (once per day), delete them softly. The snapshot reports current streak, best streak and whether the habit is due. A duplicate same-day log is rejected.
3. Recurrence (P1): set a rule on a task from the editor or quick-add. Completing it creates exactly one next occurrence with the same content and a next planned day: daily +1, weekdays skips the weekend, weekly +7, monthly +1 calendar month, every N days +N. The next task is a Todo with a due time and the rule preserved.
4. Review (P2): save a one-line note for today (or a past day); saving again updates that day rather than duplicating.
5. Stats (P2): the Insights view shows last-7-days completed/created/carried counts, focus minutes, estimate accuracy with sample count, and per-tag focus minutes/completed tasks.
6. Data safety (P2): export JSON (complete, versioned) and Markdown to the app-data `exports/` folder and show the resulting path. Import a JSON export by path; existing IDs are skipped and a summary reports inserted counts. On startup, create at most one backup per day under `backups/` and keep the newest 7. No network is used.

## Edge cases

Reject non-positive planned minutes, unknown recurrence text, invalid habit names/cadence, future review days and duplicate habit logs. Stopping an already-closed session or starting a second open session is rejected. Monthly recurrence uses the last valid day when the target month is shorter. Recurrence is applied only when completing (not when reopening). Export includes soft-deleted rows; import never overwrites an existing ID and never imports settings. Backups are plain SQLite copies and are pruned by filename date.

## Non-functional requirements

Validation and writes stay in `dayboard-core`. Focus stop + task actual minutes, completion + next occurrence, and import batches commit in one transaction with audit events. ULIDs, UTC timestamps, explicit IANA timezone. Export/import and backups are local filesystem only. Additive migration preserves all existing data.

## Out of scope

Phase 6 bridge/MCP, Phase 7 updater/autostart, OS-level focus timers while the window is hidden, Markdown import.
