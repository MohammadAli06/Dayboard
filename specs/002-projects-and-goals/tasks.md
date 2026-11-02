# Tasks

- [x] T001 Add project, goal, template and saved-filter types, task link fields and command/response contracts.
- [x] T002 Add the additive migration and entity persistence/audit helpers.
- [x] T003 Implement project CRUD, goal CRUD with derived progress, templates and task-from-template.
- [x] T004 Implement saved filters and local GitHub issue URL import.
- [x] T005 Add Tauri `open_in_vscode` command using the OS `vscode://` handler.
- [x] T006 Add the Projects view: project form/list, board, goal progress, templates and saved filters.
- [x] T007 Add project/goal selectors and "save as template" to the task editor.
- [x] T008 Extend tests for CRUD, soft delete, migration, progress, template expansion and issue parsing.
- [x] T009 Regenerate schemas; run Rust/frontend checks and a native smoke flow.

## Manual acceptance still pending

- [ ] A001 Confirm VS Code opens on each supported OS (the Windows handler path compiles and exists, but the editor was not launched during automation).
- [ ] A002 Confirm GitHub issue import behaviour on a large archive (parsing is local and covered by tests).
