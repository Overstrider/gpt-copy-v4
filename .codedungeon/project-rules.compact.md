# Project Rules Compact

PROJECT_RULES_STATUS: APPROVED
PROJECT_RULES_SOURCE: .codedungeon/project-rules.md

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
- VERIFY CodeDungeon installation with `./.codex/bin/codedungeon.exe status --human`.
- VERIFY Project Rules with `./.codex/bin/codedungeon.exe rules status --human` and `./.codex/bin/codedungeon.exe rules lint --human` after approval and compaction.
- VERIFY backend work with the Rust formatting, build, and test commands declared by the generated backend.
- VERIFY frontend work with the lint, typecheck, test, and build commands declared by the generated frontend.
- VERIFY each CodeDungeon code-writing run by recording concrete commands through `./.codex/bin/codedungeon.exe qa run`.
- VERIFY GitHub PR readiness with `git remote get-url origin` and `gh auth status` before code-writing workflows.
- MUST NOT commit real OpenRouter API keys or other provider secrets.
- MUST keep `.env.example` limited to placeholders and non-secret defaults.
- MUST load OpenRouter credentials from `OPENROUTER_API_KEY` and model configuration from `OPENROUTER_MODEL`.
- MUST proxy OpenRouter calls server-side only; frontend code must not expose provider secrets.
