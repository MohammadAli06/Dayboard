# Architecture

The Svelte frontend calls one Tauri `dispatch` adapter. The adapter fixes the actor to `ui`, serializes access to `Core`, and emits a change event after a successful mutation. The core has no Tauri dependency. It can be tested in memory or against a temporary SQLite file.

`Command` is a tagged enum (`command` plus `input`); `Response` is a tagged enum (`type` plus `data`). Generated schemas in `docs/schemas` cover every variant. Task edits are full editable-field replacements; use explicit nulls to clear dates/parents. The native UI is trusted local code; future external adapters must authenticate and validate the actor separately.

Task JSON contains the complete typed domain record. Indexed SQL columns project date, status and deletion state. FTS5 and normalized tag relations are maintained in the same transaction. A failed validation/SQL write rolls everything back. Event rows contain before/after snapshots and are guarded against UPDATE/DELETE. Dry-run operations return proposed changes and create no audit event because no change occurred.

`planned_day` is a local calendar date, distinct from the absolute UTC `due_at` deadline. Rollover moves old planned days to today, increases carried_count by calendar days missed and clears obsolete time blocks, without moving the deadline. This runs at startup and local midnight. RulePlanner produces suggestions without mutating data.

The native reminder task sleeps until the earliest undelivered reminder or the next local midnight. A Notify signal cancels/recomputes that sleep after mutations. No JS interval, polling loop or background network connection is used. A persisted marker suppresses subsequent attempts after either delivery or an OS error. Failed attempts record `reminder_failed`; rescheduling clears the marker and retries. OS presentation itself cannot be guaranteed. A database failure pauses the reminder task until a mutation wakes it, preventing a busy retry loop. Restart after an app crash between delivery and recording may duplicate a notification.

The optional bridge, its auth, MCP/JSON-RPC, undo and event-stream semantics are intentionally deferred. The typed core and audit log provide the foundation, not a claim that the bridge exists.
