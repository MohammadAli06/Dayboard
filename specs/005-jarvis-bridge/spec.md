# Feature: optional Jarvis bridge (MCP + JSON-RPC)

Status: implemented and natively smoke-tested | Spec 005 | Phase 6 of the build plan

## Scope

Add an optional, local-only bridge that exposes the typed command layer to an AI assistant:

- A new `dayboard-bridge` crate with a JSON-RPC 2.0 router and an MCP surface (`initialize`, `tools/list`, `tools/call`).
- HTTP on `127.0.0.1` only, with bearer-token authentication. Endpoints: `/rpc` (JSON-RPC), `/mcp` (MCP over HTTP), `/events` (SSE change stream) and `/health`.
- Stdio MCP for clients that launch a process.
- Compact read summaries shaped for an LLM: today, overdue, weekly stats, DSA weak topics, search.
- Undo of the latest task mutation, with a guard against clobbering later edits.
- Event push so the UI refreshes when the bridge mutates data.
- Bridge settings (enabled, port, token) in Preferences. Off by default; changing them requires an app restart.

## User stories and acceptance

1. Default off (P1): with the bridge disabled, the app opens no listening socket and behaves exactly as before. Enabling requires a token of at least 16 non-whitespace characters and a valid port; the server starts on the next launch.
2. Auth (P1): `/rpc`, `/mcp` and `/events` reject requests without the correct `Authorization: Bearer <token>` (401). `/health` is unauthenticated. The server binds only to 127.0.0.1.
3. MCP (P1): `initialize` returns protocol/server info; `tools/list` advertises the command tool, summaries, search and undo; `tools/call` runs them and returns MCP text content.
4. Commands (P1): `dayboard.command` accepts the same typed `Command` JSON as the UI, executes it as `jarvis`, and returns the typed `Response`. Mutations notify the app (UI refresh, reminder rearm) and publish an SSE event.
5. Summaries (P1): today summary (tasks, capacity, counts), overdue tasks, weekly stats and DSA topics flagged as weak.
6. Undo (P2): `dayboard.undo` reverts the newest task event if the task still matches the after-state; it refuses otherwise. A created task is soft-deleted; an edit/complete/delete is restored.
7. Stdio (P2): `dayboard-bridge --db <path>` serves MCP over newline-delimited JSON on stdin/stdout; `--port`/`--token` enable HTTP instead.
8. Schemas (P2): `dayboard.schema` returns the generated command/response JSON schemas.

## Security and edge cases

The token is stored in the local settings row (the database is not encrypted; documented). Empty or short tokens are rejected when the bridge is enabled. Payloads are capped at 1 MiB. Malformed JSON returns JSON-RPC -32700; unknown methods -32601; invalid params -32602; internal errors -32603. A corrupted bridge database connection reports a JSON-RPC error instead of panicking. Duplicate bridge instances fail to bind and are reported without crashing the app. Undo of a recurring completion may first remove the generated next occurrence (the newest event); a second undo restores the completed task.

## Non-functional requirements

No cloud, telemetry or external network client. The bridge is a separate optional process / in-app listener, off by default, using only the local SQLite file. It does not add a mandatory dependency to normal app startup. Mutations still go through `dayboard-core` commands and audit events.

## Out of scope

Phase 7 (updater/autostart/release polish), remote access, TLS, multi-user auth, OAuth, and any embedded LLM.
