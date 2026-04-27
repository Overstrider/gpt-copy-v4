# Project Rules

Status: APPROVED
Generated: 2026-04-27

## Sources Reviewed
- AGENTS.md
- .codedungeon/README.md
- .codedungeon/commands/codedungeon.md
- .codedungeon/commands/main-quest.md
- .codedungeon/commands/side-quest.md
- .codedungeon/commands/one-shot.md
- .codex/config.toml
- prompts/full-v4.txt

## Architecture And Boundaries
- The repository is a CodeDungeon Codex test repository under examples/v4.
- The target application is named gpt-copy-v4 and uses a monorepo layout with backend/ and frontend/.
- backend/ owns the Rust 2024 Axum API, SQLite persistence, OpenRouter proxying, health checks, validation, and backend tests.
- frontend/ owns the Next.js App Router UI, chat transcript, sidebar, composer, streaming UX, health/status UI, and frontend tests.
- .codedungeon/ stores shared CodeDungeon state, commands, phases, plans, tasks, reviews, QA records, and final reports.
- .codex/ and .agents/ contain Codex provider configuration, agents, skills, and the project-local CodeDungeon binary.

## Project Rules
- MUST use the promoted Codex workflow surface: `$codedungeon --full|--lite|--oneshot|--auto|--rules <prompt>`.
- MUST run `$codedungeon --full` before any lite or oneshot workflow for this example.
- MUST run `$codedungeon --lite` only after a prior plan exists under `.codedungeon/plans/*.md`.
- MUST run `$codedungeon --oneshot` only after the full and lite changes are integrated into main.
- MUST keep generated application source in backend/ and frontend/ unless the task explicitly requires root-level config or documentation.
- MUST preserve the Project Rules envelope in plans, task files, reviews, phase handoffs, and final reports.
- MUST NOT report COMPLETE unless concrete build/check/test commands are recorded with `codedungeon qa run` and final output shows `Verification: PASS`.
- MUST NOT write review reports or final reports manually; use `codedungeon review run`, `codedungeon review post`, and `codedungeon report render`.
- MUST keep the workflow PR-centered; do not use a local-only completion path.
- MUST NOT merge CodeDungeon PRs from the parent operator session.

## Commands And Verification
- VERIFY CodeDungeon installation with `./.codex/bin/codedungeon.exe status --human`.
- VERIFY Project Rules with `./.codex/bin/codedungeon.exe rules status --human` and `./.codex/bin/codedungeon.exe rules lint --human` after approval and compaction.
- VERIFY backend work with the Rust formatting, build, and test commands declared by the generated backend.
- VERIFY frontend work with the lint, typecheck, test, and build commands declared by the generated frontend.
- VERIFY each CodeDungeon code-writing run by recording concrete commands through `./.codex/bin/codedungeon.exe qa run`.
- VERIFY GitHub PR readiness with `git remote get-url origin` and `gh auth status` before code-writing workflows.

## Security And Data Rules
- MUST NOT commit real OpenRouter API keys or other provider secrets.
- MUST keep `.env.example` limited to placeholders and non-secret defaults.
- MUST load OpenRouter credentials from `OPENROUTER_API_KEY` and model configuration from `OPENROUTER_MODEL`.
- MUST proxy OpenRouter calls server-side only; frontend code must not expose provider secrets.
- MUST mock or fake provider calls in tests instead of requiring live secrets.

## Agent Operating Rules
- MUST run `./.codex/bin/codedungeon.exe rules status --human` before planning, execution, review, or final reporting.
- MUST read `.codedungeon/project-rules.compact.md` when status is approved.
- MUST include `PROJECT_RULES_STATUS`, `PROJECT_RULES_DIGEST`, and `PROJECT_RULES_READ` in workflow artifacts.
- MUST inspect `git status --short` before and after CodeDungeon-generated commits or phase transitions.
- ASK WHEN Project Rules are draft or stale before continuing with `--full` or `--lite`.
- ASK WHEN a remote GitHub operation is required but `gh` or remote credentials are unavailable.

## Open Questions
- None.
