# Tasks

- [x] T001 Add bridge settings and undo types to core, with validation and defaults.
- [x] T002 Implement `Core::undo_last` with a changed-since guard.
- [x] T003 Create the `dayboard-bridge` crate with the JSON-RPC/MCP router and summaries.
- [x] T004 Implement the localhost HTTP server, bearer auth, SSE and stdio mode.
- [x] T005 Add the standalone bridge binary.
- [x] T006 Start the in-process bridge from Tauri when enabled, with a UI notifier.
- [x] T007 Add bridge fields and token generation to Preferences.
- [x] T008 Add Rust tests for RPC, HTTP auth, undo and settings; regenerate schemas.
- [x] T009 Run all checks and a native authenticated bridge smoke flow; update docs.

## Manual acceptance still pending

- [ ] A001 Connect a real MCP client (for example an agent runtime) over `/mcp` and stdio.
- [ ] A002 Exercise the SSE stream in a long-lived client and confirm reconnects.
- [ ] A003 Rotate the token and confirm the old token is rejected after restart.
