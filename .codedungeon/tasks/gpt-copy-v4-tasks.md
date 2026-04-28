# gpt-copy-v4 Task Decomposition

PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: 3cfc7da06128fb69b4457625bdb2ea6a46298c99ef604ed165dae89855c63cbd
PROJECT_RULES_READ: yes

## Task 1: Backend Bootstrap

- Create `backend/Cargo.toml`.
- Add Rust 2024 Axum app structure under `backend/src/`.
- Add config loading from environment with safe defaults.
- Add tracing and CORS for `http://localhost:3000`.
- Verification: backend compiles, `GET /health` test passes.

## Task 2: Backend Persistence And API

- Add SQLite schema and repository functions with `sqlx`.
- Add conversation list/create and message load endpoints.
- Add structured JSON error type and request validation.
- Verification: validation and persistence tests pass with in-memory SQLite.

## Task 3: Backend OpenRouter Boundary

- Add mockable `ChatProvider` trait.
- Add OpenRouter HTTP client using `OPENROUTER_API_KEY` and `OPENROUTER_MODEL`.
- Add send-message and streaming-message endpoints.
- Verification: mocked provider tests prove user/assistant persistence without network.

## Task 4: Frontend Bootstrap And API Client

- Create Next.js App Router project in `frontend/`.
- Configure TypeScript, Tailwind, ESLint, Vitest, Testing Library, and Playwright.
- Add `zod` schemas and API client for backend DTOs.
- Verification: typecheck and API validation component tests.

## Task 5: Frontend Chat UI

- Build ChatGPT-style layout: sidebar, transcript, composer, user/assistant bubbles, loading/error states, mobile behavior.
- Use `lucide-react` for common actions.
- Use TanStack Query for conversation/message reads and native streaming fetch for sends.
- Render assistant markdown with `react-markdown` and `remark-gfm`.
- Verification: component tests for render/send/error behavior.

## Task 6: Docs And Environment

- Add root `README.md` with setup, env, backend run, frontend run, tests, and troubleshooting.
- Add root `.env.example` with placeholders only and default non-secret model.
- Confirm `.gitignore` excludes real env files and build outputs.
- Verification: docs include exact commands.

## Task 7: Review And QA Gates

- Run formatting, linting, build, and test commands through CodeDungeon QA.
- Create adversarial review persona JSON files and manifest.
- Aggregate with `codedungeon review run`, post with `codedungeon review post`.
- Push branch and open PR for human review.
- Verification: `codedungeon git verify` and `codedungeon report render`.
