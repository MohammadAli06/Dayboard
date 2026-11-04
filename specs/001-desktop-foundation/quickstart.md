# Acceptance walkthrough

1. Install README prerequisites; run `npm ci` then `npm run desktop`.
2. Set device timezone in Preferences. Capture `Read a chapter #college p2 30m today 7pm`. Confirm the title, tag, estimate, Today placement and local due time.
3. Edit notes, estimate and actual time. Complete using the checkbox or keyboard; reopen from Completed. Capture a child task and choose its parent in the editor.
4. Open Planner, drag a task onto an hour, then edit its time using the accessible field. Cancel an edit and confirm no time change; save and confirm estimate-duration/end time.
5. Set a small cap, plan enough tasks to exceed it, and verify the warning. Review a task carried three days and choose do, split or drop.
6. Search for part of a title or tag, including punctuation. Soft-delete and immediately restore a task.
7. Press Ctrl/Cmd+Shift+Space, capture from the small window, then reopen the main app from the tray and confirm the change.
8. Schedule a reminder a minute ahead, close the main window to the tray, and verify native delivery. Quit/reopen and verify it is not redelivered. Test blocked notification permissions separately.
9. Quit from tray and reopen; verify persistence. Use a temporary `DAYBOARD_DATA_DIR` when testing so real data is not affected.

Automated tests cover the deterministic core and DOM interactions. Native permission behavior, actual hotkey/tray interaction, display scaling and performance need real-OS verification; see docs/validation.md for what was actually run.
