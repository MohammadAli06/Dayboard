# Data model

`tasks`: ULID primary key, typed JSON record, indexed projections for status/planned_day/due_at/deleted and creation timestamp. A Task has title, notes, priority, energy, estimated/actual minutes, optional due and scheduled UTC timestamps, optional local planned day and parent ID, tags, carried count, completion/deletion timestamps and a persisted reminder-attempt marker.

`tags`: ULID plus unique namespaced name. `task_tags`: foreign-key pair with unique composite key. Core validates parent links and rejects cycles; soft deletion preserves child records.

`task_search`: FTS5 over title, notes and tags, with task ID unindexed. Core maintains the index within the mutation transaction.

`events`: ULID, UTC timestamp, actor enum, entity, entity ID, action and before/after JSON payload. Database triggers disallow editing/deleting existing events. An event insertion failure rolls back the entity/index mutation.

`settings`: preferences JSON with explicit IANA timezone, daily caps and workday hours. No tokens or network credentials.

Schema versioning uses rusqlite_migration. Future data-model changes require migrations and backward-compatible record deserialization or an explicit data migration.
