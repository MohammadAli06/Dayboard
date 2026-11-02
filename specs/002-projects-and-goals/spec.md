# Feature: Projects, goals, templates and saved filters

Status: implemented and natively smoke-tested | Spec 002 | Phase 3 of the build plan

## Scope

Implement Phase 3 of the build plan without network access in normal operation:

- Projects with a board (backlog / doing / done), kind (coding / college / personal), optional repo path and branch, and a live project status.
- Open a project's repo folder in VS Code through the operating system's registered `vscode://` handler.
- Create a task from a pasted GitHub issue URL. The URL is parsed locally; no HTTP request is made.
- Task templates (for example a bug-fix checklist) and creating a task from a template.
- Goals with medium/long horizon, optional target date and progress derived from linked tasks.
- Saved filters (view + search query) that can be applied from the UI.

Phases 4–7 remain deferred until they have their own specifications.

## User stories and acceptance

1. Projects (P1): create, rename, change kind/status, set repo path and branch, soft-delete. Tasks can be linked to a project from the editor and appear in that project's backlog/doing/done board. Relaunching preserves every project and link.
2. Goals (P1): create, edit and delete goals with a horizon, target date and note. A task can be linked to a goal; the goal shows `done / total` linked tasks and a percentage. Deleting a goal never deletes its tasks.
3. Templates (P2): save a template with title, notes, estimate, priority, energy, tags and project; create a new Inbox task from it in one action. Templates are soft-deleted and recorded in the event log.
4. Repo hand-off (P2): a project with a repo path offers "Open in VS Code". The app asks the OS to open a `vscode://file/<path>` URL; a missing path or handler reports a visible error instead of crashing.
5. Issue import (P2): pasting `https://github.com/<owner>/<repo>/issues/<n>` (with optional `http`, `www`, trailing path or query) creates a task titled `<owner>/<repo>#<n>` with the URL in its notes. Malformed URLs are rejected and leave the input intact.
6. Saved filters (P3): save the current view and search text under a name, apply it from the Projects view, and delete it. Filters contain no personal data beyond what the user typed.

## Edge cases

Reject empty or over-long project, goal and template names. Reject invalid project kinds/statuses and malformed issue URLs. A deleted project/goal/template cannot be edited. Reusing a deleted template is rejected. Task↔goal and task↔project links survive deleting the other side (soft delete only). Goal progress ignores deleted tasks and counts completed tasks separately. Events record `project`, `goal`, `template` and `filter` entities.

## Non-functional requirements

All writes and validation stay in `dayboard-core`; the frontend only invokes commands. One SQLite transaction writes the row plus its audit event. IDs are ULIDs, timestamps UTC, repository paths are stored as entered. No telemetry, account, AI call or HTTP request. Adding projects/goals/templates must migrate existing databases in place without losing tasks.

## Out of scope

Phase 4 (DSA/SRS/college), Phase 5 (focus/habits/recurrence/export), Phase 6 (bridge/MCP), Phase 7 (updater/autostart/release signing). GitHub issue title fetching (would require network).
