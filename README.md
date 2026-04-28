# gpt-copy-v4

ChatGPT-style monorepo with a Rust 2024 Axum backend and a Next.js App Router frontend.

PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: 3cfc7da06128fb69b4457625bdb2ea6a46298c99ef604ed165dae89855c63cbd
PROJECT_RULES_READ: yes

## Project Layout

- `backend/` - Axum API, SQLite persistence through `sqlx`, server-side OpenRouter proxy.
- `frontend/` - Next.js App Router UI, TypeScript, Tailwind, TanStack Query, safe markdown rendering.

## Environment

Create a local `.env` from `.env.example` and keep it untracked.

```powershell
Copy-Item .env.example .env
```

Required for real model calls:

```text
OPENROUTER_API_KEY=replace-with-your-openrouter-api-key
```

Default non-secret model:

```text
OPENROUTER_MODEL=nvidia/nemotron-3-super-120b-a12b:free
```

## Backend

Use Rust 1.95 or newer. If Cargo is installed but not on `PATH` in PowerShell:

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
```

Install and run:

```powershell
cd backend
cargo run
```

The backend listens on `http://127.0.0.1:8080` by default.
It is intentionally local-only: startup rejects non-loopback `BIND_ADDR` values because the API stores conversations and proxies OpenRouter with a server-side key.

Backend checks:

```powershell
cd backend
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Frontend

Install and run:

```powershell
cd frontend
npm install
npm run dev
```

The frontend listens on `http://localhost:3000` by default and calls `NEXT_PUBLIC_API_BASE_URL`.

Frontend checks:

```powershell
cd frontend
npm run lint
npm run typecheck
npm run test
npm run build
npm run test:e2e
```

If Playwright browsers are not installed:

```powershell
cd frontend
npx playwright install chromium
```

## API

- `GET /health`
- `GET /api/conversations`
- `POST /api/conversations`
- `GET /api/conversations/{conversation_id}/messages`
- `POST /api/conversations/{conversation_id}/messages`
- `POST /api/conversations/{conversation_id}/messages/stream`

Errors use:

```json
{
  "error": {
    "code": "validation_error",
    "message": "message content cannot be blank"
  }
}
```

## Troubleshooting

- `OPENROUTER_API_KEY is not configured`: add a real key to local `.env`; never commit it.
- SQLite file errors: run backend commands from `backend/` or point `DATABASE_URL` to a writable `sqlite://` path relative to the backend process cwd.
- CORS errors: keep `FRONTEND_ORIGIN=http://localhost:3000` for local frontend dev.
- Playwright browser missing: run `npx playwright install chromium` from `frontend/`.
