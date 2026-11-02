# Dayboard — Build Plan

A personal, local-first, cross-platform desktop task manager. Built to be tiny on RAM/CPU, keyboard-driven, and later plugged into Jarvis as its task brain.

## 1. Vision and principles

- **Answers "what do I do right now?"** instead of being an endless list.
- **Local-first:** one SQLite file, no account, no cloud, no network calls by default.
- **Light:** idle RAM under 60 MB, idle CPU \~0%, cold start under 1 s.
- **Keyboard-first:** global quick-add hotkey, command palette, inline text tags.
- **Quiet UI:** minimal and professional, one accent colour, dark mode default, no theatrics.
- **Jarvis-ready from day one:** every action is a typed command that a human, the UI, or an AI agent can call the same way.

## 2. Jarvis integration note (design constraint for every phase)

Dayboard will later be integrated into a Jarvis assistant whose LLM is the smart, intelligent brain. So Dayboard must be a good *tool provider* and *data source* for it, without ever depending on it.

**Rules to follow while building:**

1. **Single command layer.** All mutations go through `dayboard-core` commands (`create_task`, `complete_task`, `reschedule_task`, `plan_day`, `log_dsa_problem`, ...). The UI, quick-add, and Jarvis all call these. No business logic in the frontend.
2. **Every command has a JSON schema** (input and output). This lets the same definitions become LLM tool-calling specs and MCP tools.
3. **Append-only event log** (`events` table): every change is recorded with timestamp, actor (`user`, `ui`, `jarvis`), and payload. Jarvis can learn habits and explain what happened.
4. **Bridge is a separate optional process** (sidecar): local-only server exposing the commands over MCP and JSON-RPC/HTTP on `127.0.0.1` with a bearer token. Off by default, so the main app stays light.
5. **Deterministic first, AI optional.** The quick-add parser and day-planner are rule-based. A `Planner` trait/interface lets Jarvis (LLM) override or augment them later.
6. **Stable IDs and UTC times.** ULIDs for IDs, UTC timestamps plus user timezone, so sync or agent access never breaks.
7. **Read APIs shaped for LLMs:** `get_today_summary`, `get_overdue`, `get_weekly_stats`, `get_dsa_weak_topics`, each returning compact JSON.
8. **Event push:** the bridge can emit events (task due, task carried 3x, streak at risk) so Jarvis can nudge proactively.
9. **Safety:** destructive commands (delete, bulk reschedule) support a `dry_run` flag and are logged, so an AI mistake is reversible.

## 3. Tech stack

- **Shell:** Tauri 2 (Rust + system webview). Windows, macOS, Linux from one codebase. Idles around 30–60 MB vs 150 MB+ for Electron.
- **Frontend:** Svelte 5 + TypeScript + Vite. Hand-written CSS with CSS variables (no heavy UI library). Small virtualised list for long lists.
- **Core (Rust):** a Cargo workspace crate `dayboard-core` holding domain models, DB access, parser, scheduler, spaced repetition, stats. No Tauri dependency, so it is reusable by the bridge and tests.
- **Database:** SQLite (WAL mode) via `rusqlite` + `rusqlite_migration`. FTS5 for search. Optional `sqlite-vec` later for semantic search by Jarvis.
- **Tauri plugins:** global-shortcut, notification, autostart, tray (built-in), single-instance, updater, dialog, fs.
- **Time/recurrence:** `chrono` + `chrono-tz`, `rrule` crate for recurring rules.
- **Bridge (Phase 6):** crate `dayboard-bridge`, `axum` for localhost JSON-RPC, MCP server over stdio and HTTP, `schemars` to generate JSON schemas from Rust types, `serde`.
- **Testing:** `cargo test` for core, Vitest for frontend, Playwright or `tauri-driver` for a few end-to-end flows.
- **CI and release:** GitHub Actions building Win/macOS/Linux installers with `tauri-action`.

## 4. Architecture and flow

**Repo layout**

```
dayboard/
  crates/
    dayboard-core/     # domain, db, parser, scheduler, srs, stats, commands
    dayboard-bridge/   # optional local server: MCP + JSON-RPC (Phase 6)
  src-tauri/           # app shell: tray, hotkey, notifications, tauri commands
  src/                 # Svelte UI (views, stores, components)
  docs/                # this plan, command schemas, ADRs
```

**Normal flow (UI)**

`User input → Svelte UI → Tauri invoke → dayboard-core command → SQLite → event row → Tauri event → UI store update → tray badge / notification scheduler re-arms`

**Quick-add flow**

`Global hotkey → small capture window → text → core parser (tags, priority, estimate, time) → create_task → window closes`

**Jarvis flow (Phase 6)**

`Voice/text → Jarvis LLM → tool call (e.g. dayboard.create_task) → Bridge (MCP/JSON-RPC, token auth) → dayboard-core command → SQLite → event → UI updates live`

**Reverse flow:** `Dayboard event (task due, carried 3x, streak at risk) → Bridge push/webhook → Jarvis decides whether and how to nudge you.`

**Scheduler:** no polling. One timer is armed for the *next* upcoming reminder and re-armed on any change. The window is suspended when hidden to the tray.

## 5. Data model (SQLite)

- `tasks`: id, title, notes, status (inbox/todo/doing/done/dropped), priority (1–4), energy (low/high), est\_min, actual\_min, due\_at, scheduled\_start, scheduled\_end, project\_id, goal\_id, parent\_id, recurrence\_rule, carried\_count, created\_at, completed\_at
- `projects`: id, name, kind (coding/college/personal), repo\_path, status
- `goals`: id, title, horizon (long/medium), target\_date, progress\_note
- `tags` and `task_tags`: tags by namespace (`#area`, `@context`, `!type`)
- `dsa_problems`: id, title, url, topic, difficulty, platform, status, confidence (1–5), solved\_at, next\_review\_at, review\_stage, notes
- `subjects` and `syllabus_items`: college prep checklists
- `deadlines`: exams, submissions, with target dates
- `habits` and `habit_logs`: recurrence, streak, last\_done
- `focus_sessions`: task\_id, started\_at, ended\_at, planned\_min, kind (focus/break)
- `templates`: name, JSON subtask list
- `events`: id, ts, actor, entity, entity\_id, action, payload JSON (the Jarvis-friendly log)
- `settings`: key/value

All tables use ULID text IDs, UTC timestamps, soft-delete where useful.

## 6. Complete feature list

**Capture and organisation**

- Inbox for quick-capture, triaged later
- Smart quick-add with inline tags (see section 7)
- Subtasks, notes (Markdown), search (FTS5), saved filters

**Horizons (long term / short term)**

- **Today:** daily list with a capacity cap
- **This Week:** short-term pool
- **Goals:** long-term items that tasks link up to, with progress derived from linked tasks
- **Inbox:** unsorted capture

**Priority and energy**

- P1–P4 priority, high/low energy field to match tasks to your mood
- **Today cap:** warns when planned minutes or task count exceed your limit (anti-overplanning)
- Auto-rollover with a "carried Nx" badge; at 3x it asks: do, split, or drop

**Daily planning**

- Time-blocking: drag tasks onto a day timeline, estimate vs actual
- Time-based tasks with reminders and notifications
- Focus mode: only the current task, Pomodoro-style timer, logs focus sessions
- End-of-day review: done, carried, one-line note (30 seconds)
- Morning plan suggestion: rule-based "suggested today" from priority, due dates, energy

**Coding projects**

- Project boards (backlog / doing / done) per project
- Task templates (e.g. Bug fix = reproduce, fix, test, PR)
- Link task to repo path or branch; open in VS Code from the task
- Paste a GitHub issue URL to create a task

**DSA and college prep**

- DSA problem tracker: link, topic, difficulty, confidence
- Spaced repetition: solved problems resurface at 1, 3, 7, 21 days as revise tasks, adjusted by confidence
- Topic coverage heatmap (arrays, graphs, DP, ...) to expose weak spots
- Deadline and exam countdowns that back-schedule study tasks
- Syllabus checklist per subject

**Habits and recurrence**

- Daily habits with streaks (DSA daily, revise, gym)
- Flexible recurrence ("every 3 days after completion", weekdays, monthly)

**Quality of life**

- Global hotkey quick-add, command palette (`Ctrl/Cmd+K`)
- Tray icon showing today's remaining count, autostart, single instance
- Weekly stats: focus hours per tag, completion rate, carry-over rate, estimate accuracy
- Export/import JSON and Markdown, automatic rolling DB backups
- Light/dark theme, configurable work hours and daily cap

**Jarvis bridge (Phase 6)**

- MCP server + JSON-RPC exposing all commands and read summaries
- Event stream for proactive nudges
- Dry-run and undo for AI-driven changes

## 7. Quick-add syntax

```
Solve 2 sliding window problems #dsa @deep p1 45m today 7pm
```

- `#area` project/area, `@context` energy or place, `!type` task type
- `p1`–`p4` priority, `45m` / `1h30m` estimate
- Dates: `today`, `tomorrow`, `fri`, `next week`, `2026-11-05`, times like `7pm`
- `every 3 days` for recurrence, `^goal-name` to link a goal

Parser is deterministic with unit tests (table-driven). LLM parsing is an optional later override via the `Planner` interface.

## 8. Performance budget

- Idle RAM < 60 MB, idle CPU \~0%, cold start < 1 s, DB query p95 < 10 ms
- No polling anywhere; event-driven timers only
- Webview suspended when hidden to tray
- Bridge and any AI features run out-of-process and are off by default
- Release build size target < 15 MB installer

## 9. UI direction

Minimal and professional: neutral palette, one accent, subtle transitions only, dark by default, generous spacing, no gradients or flashy motion. Three main views:

1. **Today** (list + current focus task)
2. **Planner** (week + day timeline)
3. **Library** (projects, goals, DSA tracker, college, habits, stats)

## 10. Build phases

**Phase 0 — Setup (day 1–2)**

- Cargo workspace, Tauri 2 + Svelte + TS scaffold, lint/format, CI skeleton
- Core crate with migrations framework and command/event pattern
- *Done when:* app launches on all three OSes and a test command round-trips through core to SQLite.

**Phase 1 — Foundation (week 1)**

- Task CRUD, statuses, subtasks, notes, tags with autocomplete
- Quick-add parser with tests, Inbox and Today views
- Event log written on every mutation
- *Done when:* you can capture and complete tasks entirely by keyboard.

**Phase 2 — Planning (week 2)**

- Priority/energy, Today cap, auto-rollover with carried badge
- Week view, day timeline with drag time-blocking, estimates vs actual
- Tray + badge, global hotkey capture window, notification scheduler
- *Done when:* you can plan a full day and get reminded on time with the app hidden.

**Phase 3 — Projects and goals (week 3)**

- Project boards, templates, goal linking and progress
- Repo path/branch link, open in VS Code, GitHub issue URL import
- Command palette, search (FTS5), saved filters
- *Done when:* a coding project can be run from a board with templates.

**Phase 4 — DSA and college prep (week 4)**

- DSA tracker, spaced repetition engine, topic heatmap
- Subjects, syllabus checklists, deadlines with back-scheduling
- *Done when:* solving a problem auto-creates its revise tasks and deadlines generate study tasks.

**Phase 5 — Focus, habits, stats, safety (week 5)**

- Focus mode + sessions, habits and streaks, recurrence rules
- End-of-day review, weekly stats, export/import, rolling backups
- *Done when:* a full week of real use produces meaningful stats.

**Phase 6 — Jarvis bridge (week 6)**

- Generate JSON schemas for all commands, MCP server (stdio + HTTP), JSON-RPC on localhost with token auth
- LLM-shaped read summaries, event push, dry-run + undo
- `Planner` interface and a mock Jarvis client for testing
- *Done when:* an MCP client can create, reschedule, and summarise tasks and see changes live in the UI.

**Phase 7 — Packaging and polish (week 7)**

- Installers for Win/macOS/Linux, auto-update, autostart, icons, onboarding
- Performance audit against the budget, crash-safe DB handling
- *Done when:* clean install works on a fresh machine and the budget is met.

Ship Phases 1–2 and start using it daily; your own usage decides what to prioritise next.

## 11. Agent prompts for your vibecoding agent

Use the **Master prompt** once at the start of every session (or put it in the agent's rules file, e.g. `AGENTS.md` / `CLAUDE.md`), then feed one phase prompt at a time.

### Master prompt

```
You are building "Dayboard", a personal, local-first, cross-platform desktop task manager (Windows, macOS, Linux). It will later be integrated into a Jarvis AI assistant as a tool provider.

Stack: Tauri 2 (Rust), Svelte 5 + TypeScript + Vite, SQLite (rusqlite, WAL, rusqlite_migration, FTS5), chrono/chrono-tz, rrule. Cargo workspace: crates/dayboard-core (domain, db, parser, scheduler, srs, stats, commands), crates/dayboard-bridge (optional MCP/JSON-RPC server, Phase 6), src-tauri (shell), src (UI).

Non-negotiable rules:
1. All business logic and mutations live in dayboard-core as typed commands. The UI only calls them via Tauri invoke. No logic in the frontend.
2. Every command has serde input/output types with schemars JSON schemas, so they can become LLM tool specs and MCP tools later.
3. Every mutation appends a row to the `events` table (actor: user|ui|jarvis, entity, action, payload).
4. IDs are ULIDs; timestamps are UTC ISO-8601 plus user timezone setting.
5. Performance budget: idle RAM < 60 MB, idle CPU ~0%, cold start < 1s. No polling; use a single timer armed for the next reminder. Suspend work when the window is hidden to tray.
6. No network calls by default. No telemetry.
7. UI: minimal, professional, dark by default, one accent colour, no heavy UI libraries, no flashy animation. Keyboard-first (Ctrl/Cmd+K palette, global quick-add hotkey).
8. Write unit tests for core logic (parser, scheduler, SRS, rollover) with table-driven cases.
9. Work in small steps. Before coding each phase, list the files you will create or change and any assumptions, then implement. Do not add features outside the current phase. Ask me if something is ambiguous.
```

### Phase 0 prompt

```
Phase 0: scaffold Dayboard. Create the Cargo workspace with crates/dayboard-core and src-tauri, a Tauri 2 + Svelte 5 + TS + Vite frontend, ESLint/Prettier/rustfmt/clippy config, and a GitHub Actions workflow that builds on windows, macos, ubuntu. In dayboard-core set up the SQLite connection (WAL, foreign keys on), a migrations runner, a `Command` pattern (typed input/output + schemars), and an `events` table writer. Add one demo command `ping_db` and call it from the UI. Acceptance: app launches on all three OSes, ping round-trips UI -> Tauri -> core -> SQLite, CI is green.
```

### Phase 1 prompt

```
Phase 1: tasks foundation. Add migrations for tasks, tags, task_tags, events. Implement core commands: create_task, update_task, complete_task, delete_task (soft), list_tasks(filter), add_subtask, set_tags. Implement the quick-add parser in core: `#area`, `@context`, `!type`, `p1-p4`, estimates (45m, 1h30m), natural dates/times (today, tomorrow, fri, 7pm, 2026-11-05). Table-driven tests with at least 30 cases. UI: Inbox and Today views, task row component, inline tag autocomplete, quick-add input, keyboard navigation (j/k, x to complete, e to edit, / to search). Every mutation writes an event. Acceptance: capture, edit and complete tasks by keyboard only; parser tests pass.
```

### Phase 2 prompt

```
Phase 2: planning. Add priority (1-4), energy (low/high), est_min/actual_min, scheduled_start/end, carried_count. Implement: Today cap (max tasks and max planned minutes from settings, warning in UI when exceeded), end-of-day rollover (unfinished Today tasks move to tomorrow, carried_count++, at 3 show do/split/drop prompt), Week view, a day timeline with drag-to-time-block, estimate vs actual display. Add tray icon with remaining-today badge, global hotkey that opens a small borderless quick-add window, and a notification scheduler that arms ONE timer for the next upcoming reminder and re-arms on any change (no polling). Hide-to-tray should suspend the webview. Acceptance: plan a day, get a notification on time with the window hidden, idle CPU ~0%.
```

### Phase 3 prompt

```
Phase 3: projects and goals. Add projects (kind: coding/college/personal, repo_path), goals (long/medium horizon, target_date), templates (named subtask lists). Implement project boards (backlog/doing/done) with drag between columns, goal linking with progress computed from linked tasks, task templates (e.g. Bug fix = reproduce, fix, test, PR), open-in-VS-Code from a task (`code <repo_path>`), GitHub issue URL -> task (parse URL, fetch only when the user explicitly triggers it), command palette (Ctrl/Cmd+K) covering every command, FTS5 search and saved filters. Acceptance: run a coding project from a board using templates and goals.
```

### Phase 4 prompt

```
Phase 4: DSA and college prep. Add dsa_problems, subjects, syllabus_items, deadlines. Implement: problem tracker (title, url, topic, difficulty, platform, confidence 1-5); spaced repetition engine in core at 1/3/7/21 days, intervals adjusted by confidence, which auto-creates revise tasks; topic coverage heatmap view; syllabus checklist per subject; deadline/exam countdown that back-schedules study tasks across available days respecting the Today cap. Unit test the SRS and back-scheduling. Acceptance: logging a solved problem creates its revise tasks; adding an exam date generates a study plan.
```

### Phase 5 prompt

```
Phase 5: focus, habits, stats, safety. Add focus mode (current task only, Pomodoro timer, focus_sessions log), habits with streaks and logs, recurrence via rrule plus "N days after completion", 30-second end-of-day review (done, carried, one-line note), weekly stats (focus hours per tag, completion rate, carry-over rate, estimate accuracy), JSON and Markdown export/import, automatic rolling DB backups (keep last 14). Acceptance: a week of use yields meaningful stats; export then import into a fresh DB reproduces the data.
```

### Phase 6 prompt (Jarvis bridge)

```
Phase 6: Jarvis bridge. Create crates/dayboard-bridge as a separate optional binary/sidecar, off by default, bound to 127.0.0.1 with a bearer token stored in settings. Expose every dayboard-core command as (a) MCP tools (stdio and HTTP) and (b) JSON-RPC, with schemas generated by schemars. Add LLM-friendly read tools returning compact JSON: get_today_summary, get_overdue, get_weekly_stats, get_dsa_weak_topics, find_free_slots. Add an event stream (SSE or webhook) for: task_due, task_carried_3x, streak_at_risk, deadline_near. Add `dry_run` for destructive commands and an `undo_last(actor)` command driven by the events table. Define a `Planner` trait in core (default rule-based) so Jarvis can supply plans. Build a small mock Jarvis client script and integration tests. Acceptance: an MCP client can create, reschedule and summarise tasks; the UI updates live; main app memory is unchanged when the bridge is off.
```

### Phase 7 prompt

```
Phase 7: packaging and polish. Configure tauri-action to build Windows (msi/nsis), macOS (dmg), Linux (AppImage/deb). Add autostart toggle, single-instance, auto-updater, app icons, first-run onboarding (timezone, work hours, daily cap). Audit against the budget (idle RAM < 60 MB, CPU ~0%, cold start < 1s, installer < 15 MB) and fix regressions. Make DB handling crash-safe (WAL checkpoint on exit, integrity check on start, restore from last backup prompt). Acceptance: clean install on a fresh VM per OS, budget met.
```

## 12. Risks and notes

- **Webview differences:** test on Linux (WebKitGTK) early; keep CSS conservative.
- **Notification reliability:** OS permission prompts differ; handle denied state gracefully.
- **Scope creep:** finish Phase 2 and use the app for a week before starting Phase 3.
- **AI safety for Jarvis:** keep dry-run, undo, and the event log mandatory for every AI-callable command.
- **Future (not now):** optional encrypted sync, semantic search with `sqlite-vec`, voice quick-add via Jarvis.
