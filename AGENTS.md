# Daybridge contributor and agent rules

Read `.specify/memory/constitution.md` and the active `specs/*/spec.md`, `plan.md`, and `tasks.md` before changing code. Keep these artifacts current. Use GitHub Spec Kit's installed `.agents/skills/speckit-*` workflow for new features: specify, clarify if needed, plan, tasks, implement, analyze.

Daybridge is the product name; `dayboard-core` preserves the original build plan's crate name. Ship phases 0–2 first. Later phases require their own specification. No cloud, telemetry, embedded LLM, or mandatory bridge.

All business rules and writes belong in dayboard-core. Each public operation has typed serde/schemars contracts. A mutation and its append-only audit event commit in one SQLite transaction. Use ULIDs, UTC timestamps, explicit IANA timezone, soft deletion, and dry runs for destructive operations. Frontend code presents state and invokes commands only.

No polling for reminders. Rearm one native timer after mutations and use a separate midnight wakeup for rollover. Keep secrets, local databases, exports and agent state out of Git. Commit lockfiles and shared Spec Kit templates. Do not claim performance or platform acceptance without measurements.

Checks: `npm ci`, `npm run check`, `npm run lint`, `npm test`, `npm run build`, `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`. Native prerequisites are in README.md.
