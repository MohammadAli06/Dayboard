# Dayboard

A quiet, local-first desktop task planner. Capture a thought, give it a place in your day, and get back to work.

**Early development release:** the foundation and daily-planning slice of the [original Dayboard build plan](<Dayboard — Build Plan.md>). Built with Tauri 2, Svelte 5, Rust and SQLite. No account, cloud service, telemetry, API key or AI subscription is required to run it.

## Run your own copy

Fork this repository on GitHub, then clone **your fork's URL** from its Code menu. In the cloned directory:

```sh
npm ci
npm run desktop
```

Install these prerequisites first:

| Platform     | Prerequisites                                                                                                                           |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| All          | Git, Node.js 24.15+ LTS or 26+, npm, Rust stable via rustup, a supported desktop session                                                |
| Windows      | Visual Studio Build Tools with **Desktop development with C++**, Windows SDK, Microsoft Edge WebView2 Runtime                           |
| macOS        | Xcode command-line tools (`xcode-select --install`)                                                                                     |
| Ubuntu 24.04 | `sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev` |

See [Tauri's platform prerequisites](https://v2.tauri.app/start/prerequisites/) for other distributions. Restart your terminal after installing Rust so `cargo` is available. First native builds take several minutes.

`npm run dev` opens the frontend development surface only; it deliberately does not simulate or save tasks in a browser. Use `npm run desktop` for the working app.

On first use, open **Preferences → Use this device's timezone**, then save. The initial timezone is UTC. Set your daily task/minute caps and work hours there too.

## What works

- Inbox, Today, seven-day planner, completed/dropped tasks and global FTS5 search.
- Quick capture with `#area`, `@context`, `!type`, `p1`–`p4`, `45m`, `1h30m`, `today`, `tomorrow`, weekdays, `next week`, ISO dates, 12/24-hour times, `repeat daily|weekdays|weekly|monthly` and `every 3 days`.
- Task editing, notes, parent/subtask links, energy, estimate/actual minutes, soft delete and immediate restore.
- Projects with a backlog/doing/done board, repo path and branch, **Open in VS Code**, local GitHub issue URL import, task templates, goals with derived progress, and saved filters.
- DSA problem tracker with confidence-based spaced repetition (1/3/7/21 days), linked revision tasks, topic coverage, subjects with syllabus checklists, and deadlines that back-schedule study tasks.
- Focus sessions (focus/break) that roll elapsed minutes into the linked task, habits with streaks, daily review notes, last-7-days stats, recurrence rules, JSON/Markdown export, JSON import and rolling daily backups.
- Optional localhost Jarvis bridge: JSON-RPC, MCP over HTTP or stdio, bearer-token auth, compact summaries, SSE change events and undo (off by default).
- One-time welcome card and opt-in start-on-login for packaged builds.
- Drag-to-hour time blocking, accessible date/time fields, capacity warnings and rule-based plan suggestions.
- Calendar rollover with carried counts, preserving original due deadlines.
- Tray menu, single instance, global capture and event-driven native reminders.
- Typed command/response schemas, transactional append-only event history and destructive dry-runs, ready for a future adapter.

```text
Solve two sliding window problems #dsa @deep p1 45m today 7pm
Read a chapter #college p2 1h30m tomorrow
Review notes !revision fri 09:30
```

Recognized metadata tokens are removed from the title. Date-only capture means 17:00 in your configured timezone; time-only means today. Recurrence uses `repeat daily`, `repeat weekdays`, `repeat weekly`, `repeat monthly` or `every N days`, and is also selectable in the editor. Unsupported future syntax (`^goal` capture) remains literal title text; link a goal from the task editor instead. Ambiguous/nonexistent daylight-saving times are rejected so you can choose another time.

| Shortcut                 | Action                                                                 |
| ------------------------ | ---------------------------------------------------------------------- |
| Ctrl/Cmd + Shift + Space | Global quick capture; tray menu is the fallback if another app owns it |
| Ctrl/Cmd + K             | Navigation palette                                                     |
| N / `/`                  | Focus capture / search                                                 |
| J / K                    | Select next / previous task (when not in a field or button)            |
| X / E                    | Complete or reopen / edit selected task                                |
| Escape                   | Close editor, palette or capture                                       |

Closing the main window **hides it to the tray**. Choose **Quit** in the tray menu to exit. Enable OS notifications for Dayboard. Reminders run in Rust even when the window is hidden; the app must remain running. Install a packaged build for the most reliable Windows notification identity. Autostart is opt-in in Preferences and is applied by packaged builds.

## Data and privacy

SQLite lives in Tauri's app-data directory under `dev.dayboard.desktop`, never in the Git checkout:

- Windows: `%APPDATA%\dev.dayboard.desktop\dayboard.sqlite3`
- macOS: `~/Library/Application Support/dev.dayboard.desktop/dayboard.sqlite3`
- Linux: `~/.local/share/dev.dayboard.desktop/dayboard.sqlite3`

For an isolated testing or portable data directory, set `DAYBOARD_DATA_DIR` to an absolute directory before starting the app. The default remains the OS app-data directory.

The DB uses WAL, migrations and foreign keys. Mutations and audit events commit together. Deleted tasks are retained; task history can contain previous titles/notes. The database is **not encrypted**. To manually back up, quit through the tray and copy the database; while running, SQLite may also need its WAL file. Exports and rolling backups (newest seven) are written under `exports/` and `backups/` in the same app-data directory. Import is additive and skips IDs that already exist.

The app makes no runtime network requests. Installing dependencies and building may download packages/tooling. `.gitignore` excludes databases, WALs, backups, exports, environment files, keys, build output and private agent state. Review `git status` before publishing; ignore rules cannot undo a secret already committed.

## Jarvis bridge (optional, off by default)

The bridge exposes the same typed commands to a local MCP/JSON-RPC client. It is disabled unless you enable it:

1. **Preferences → Jarvis bridge**, tick **Enable local bridge**, choose a port (default 8765), click **Generate token**, then **Save preferences**.
2. Restart Dayboard. The log prints `Dayboard bridge listening on 127.0.0.1:8765`.

It binds loopback only and requires `Authorization: Bearer <token>` on every route except `/health`. The token is stored in the local settings row; the database is not encrypted.

| Route        | Purpose                                                |
| ------------ | ------------------------------------------------------ |
| `POST /rpc`  | JSON-RPC 2.0 methods                                   |
| `POST /mcp`  | MCP over HTTP (`initialize`, `tools/list`, `tools/call`) |
| `GET /events`| Server-sent change events (authorized)                 |
| `GET /health`| Unauthenticated liveness                               |

```sh
curl -s http://127.0.0.1:8765/rpc \
  -H "Authorization: Bearer $DAYBOARD_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"dayboard.today_summary"}'
```

Methods: `dayboard.command` (any typed command), `dayboard.today_summary`, `dayboard.overdue`, `dayboard.weekly_stats`, `dayboard.dsa_weak_topics`, `dayboard.search`, `dayboard.undo` and `dayboard.schema`. MCP tools mirror these as `dayboard_*`. Mutations refresh the open UI and rearm reminders.

For a stdio MCP client, run the standalone bridge against the same database:

```sh
cargo run -p dayboard-bridge -- --db "%APPDATA%\dev.dayboard.desktop\dayboard.sqlite3"
```

Add `--port 8765 --token <token>` to serve HTTP instead. Do not run two HTTP bridges on one port. The in-app listener is recommended while the UI is open because it pushes change notifications. The bridge never starts by default and makes no outbound network requests.

## Spec-driven development

This project was initialized using the official **GitHub Spec Kit / Specify CLI**, not a hand-written imitation:

```sh
uv tool install specify-cli --from git+https://github.com/github/spec-kit.git@7370e592f3cd9c06e5b698983957e227c75b30e2
specify check
```

The pinned revision identifies the initialization tooling used here. `uv` is needed only for Spec Kit, not for running Dayboard. Shared infrastructure is in `.specify/` and reusable Codex skills in `.agents/skills/`. No local agent credentials are committed.

Read the [constitution](.specify/memory/constitution.md), [feature spec](specs/001-desktop-foundation/spec.md), [plan](specs/001-desktop-foundation/plan.md) and [tasks](specs/001-desktop-foundation/tasks.md). In a Codex session, use the installed workflow:

```text
$speckit-specify <describe one feature and its acceptance criteria>
$speckit-plan
$speckit-tasks
$speckit-implement
$speckit-analyze
```

The CLI scaffolds the workflow; your coding agent performs the reasoning and implementation. Do not expect a `specify implement` shell command. Other agent integrations can be installed with `specify integration install <agent>`. See [official Spec Kit documentation](https://github.github.com/spec-kit/) and [contributing](CONTRIBUTING.md).

## Validate and package

```sh
npm run check
npm run lint
npm test
npm run build
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run schemas
npm run bench:core
npm run tauri build
```

Native installers appear under `target/release/bundle/`. A Windows x64 NSIS installer was built locally during implementation; see the validation record. GitHub Actions runs the quality checks on Windows, macOS and Linux. A manual **Package desktop app** workflow builds installers as downloadable workflow artifacts. It does not publish a release or require signing secrets. Fork maintainers should add their own signing/notarization before distributing publicly; unsigned installers may produce OS trust prompts. See [releasing](docs/releasing.md) for signing, notarization, autostart and updater setup.

## Roadmap and limitations

Phase 3 is implemented: projects with a board, repo/VS Code hand-off, local GitHub issue import, templates, goals and saved filters. Phase 4 is implemented: DSA spaced repetition, topic coverage, subjects/syllabus and deadline study tasks. Phase 5 is implemented: focus sessions, habits/streaks, recurrence, daily review, last-7-days stats, export/import and rolling backups. Phase 6 is implemented: the optional localhost bridge (JSON-RPC/MCP, token auth, summaries, events and undo). Phase 7 is implemented except for hosted auto-update, which needs your signing key and update endpoint: packaging metadata, autostart, onboarding, the release guide and a core benchmark are in place. The bridge is off by default; there is no listening server unless you enable it.

Native webview suspension, fresh-machine installation on all three OSes, notification permissions, accessibility with a screen reader, and the plan's performance budgets need manual validation. Hidden UI avoids interval work, but this is not a claim that every OS suspends its webview. Task DOM rendering is bounded; snapshots currently load all task data, so very large archives need database pagination in a later performance pass. See [validation notes](docs/validation.md).

MIT licensed. You can fork, modify and share it.
