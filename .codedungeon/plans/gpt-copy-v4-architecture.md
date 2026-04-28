# gpt-copy-v4 Architecture Plan

PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: 3cfc7da06128fb69b4457625bdb2ea6a46298c99ef604ed165dae89855c63cbd
PROJECT_RULES_READ: yes

## Scope

- Bootstrap a monorepo with `backend/` and `frontend/`.
- Backend: Rust 2024 Axum API, SQLite persistence through `sqlx`, OpenRouter server-side proxy, structured errors, tracing, CORS for `http://localhost:3000`.
- Frontend: Next.js App Router, TypeScript, Tailwind, ChatGPT-style UI with conversation sidebar, transcript, composer, markdown rendering, API validation, component tests, and one Playwright send-message smoke test.
- Root docs and `.env.example` must contain only placeholders and non-secret defaults.

## Backend Shape

- `backend/Cargo.toml` defines a binary crate with `edition = "2024"`.
- Runtime state is owned by `AppState` with:
  - `SqlitePool` for conversations/messages.
  - `Arc<dyn ChatProvider>` for OpenRouter or test mock.
  - `AppConfig` for API key, model, frontend origin, database URL, bind address.
- SQLite tables:
  - `conversations(id TEXT PRIMARY KEY, title TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)`.
  - `messages(id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE)`.
- Startup runs idempotent migrations from embedded SQL.

## Backend API

- `GET /health` returns `{ "status": "ok" }`.
- `GET /api/conversations` lists recent conversations.
- `POST /api/conversations` creates a conversation with optional validated title.
- `GET /api/conversations/:id/messages` loads a transcript.
- `POST /api/conversations/:id/messages` validates user content, persists user and assistant messages, and returns both.
- `POST /api/conversations/:id/messages/stream` validates user content, persists the user message, proxies OpenRouter streaming, persists the final assistant content, and emits newline-delimited JSON events.

## OpenRouter Boundary

- `OPENROUTER_API_KEY` is required only for real provider calls.
- `OPENROUTER_MODEL` defaults to `nvidia/nemotron-3-super-120b-a12b:free`.
- Frontend never receives provider credentials.
- Tests use a mock `ChatProvider`; no network calls are required.

## Frontend Shape

- `frontend/app/page.tsx` renders the chat application as the first screen.
- API logic lives under `frontend/lib/api.ts` with `zod` response validation.
- UI components live under `frontend/components/`.
- TanStack Query handles conversation/message reads and invalidation.
- Native `fetch` streaming handles the chat send flow.
- `react-markdown` + `remark-gfm` renders assistant markdown.

## Verification Plan

- Backend: `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`.
- Frontend: `npm run lint`, `npm run typecheck`, `npm run test`, `npm run build`, `npm run test:e2e`.
- All concrete verification commands are recorded through `./.codex/bin/codedungeon qa run --phase 6 --cmd "<cmd>"`.

## Risks

- Local cargo is installed under `C:/Users/loldi/.cargo/bin` but not ambient PATH; verification commands should use a PATH prefix or full binary path.
- Playwright browser availability may require `npx playwright install chromium`; if missing, record the exact failure and install only the needed browser.
- Streaming tests should avoid real OpenRouter calls by mocking frontend fetch or backend provider behavior.
