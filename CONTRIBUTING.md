# Contributing

Start with README.md for setup, then AGENTS.md and the constitution. No account beyond GitHub is needed to contribute; Spec Kit is optional for running the app but is the development workflow for features.

1. Fork and clone, create a feature branch, and describe the user-visible problem in `specs/NNN-feature/spec.md` using the installed Spec Kit workflow.
2. Review the plan and acceptance criteria before implementation. Keep business rules in `dayboard-core` and UI presentation in Svelte.
3. Implement with meaningful parser/domain/regression tests. Update schemas with `npm run schemas` when contracts change.
4. Run the validation commands in README. Record any platform-dependent checks you could not run. Never claim green CI before it has run.
5. Open a pull request with the problem, behavior change, relevant spec and validation results. Avoid personal database files, exports, screenshots with personal data and signing material.

Keep runtime dependencies small. No telemetry, analytics, remote fonts or network fetches in normal app flows. Do not silently introduce new services or require API keys.

Use `cargo fmt --all` and `npm run format`. Lockfiles are committed for reproducible application builds. Existing Spec Kit skill/template instructions are upstream content; project decisions belong in the constitution and feature specs.
