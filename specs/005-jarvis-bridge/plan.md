# Implementation plan

## Constitution check

Local-only and optional; typed command contracts; audit events for every bridge mutation; no runtime cloud/network dependency (loopback socket only, disabled by default). Pass.

## Files and responsibilities

- `crates/dayboard-bridge/`: JSON-RPC/MCP router, summaries, SSE hub, minimal HTTP/1.1 server, stdio loop and binary.
- `crates/dayboard-core/src/model.rs`: bridge settings fields and `UndoResult`.
- `crates/dayboard-core/src/lib.rs`: settings validation/defaults and `undo_last`.
- `src-tauri/src/main.rs`: start the bridge in-process when enabled, with its own SQLite connection and a notifier that refreshes the UI/rearms reminders.
- `src/lib/api.ts`, `src/App.svelte`: bridge settings in Preferences.
- `docs/`, `README.md`, `specs/005-jarvis-bridge/*`: usage, security and status.

## Design

The bridge crate owns a `Bridge` with a `std::sync::Mutex<Core>` connection to the same SQLite file and a `Notifier` callback. `handle_rpc` maps JSON-RPC methods to typed core commands and compact summaries; no business logic is duplicated. HTTP is implemented directly on `std::net::TcpListener` (no new HTTP dependency): `Content-Length` requests, `Connection: close` responses and an SSE stream for events. Authentication compares `Authorization: Bearer <token>` in constant time; the server refuses an empty token and binds only to loopback. The Tauri app starts the bridge at setup when enabled, passing a notifier that emits `dayboard:changed` and wakes the reminder loop. The standalone binary serves newline-delimited MCP on stdio, or HTTP with `--port`/`--token`.

## Validation

Unit tests for the RPC router (initialize, tools, command, summary, undo, errors), notifier firing, HTTP 401/200 with a real loopback socket, and core undo guards/settings defaults. Rust fmt/clippy/tests, frontend checks, schema regeneration and a native launch with a token, an authenticated `curl`-style request and a bridge-created task visible in the UI.
