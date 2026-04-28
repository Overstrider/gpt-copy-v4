# gpt-copy-v4 QA Trap Plan

PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: 3cfc7da06128fb69b4457625bdb2ea6a46298c99ef604ed165dae89855c63cbd
PROJECT_RULES_READ: yes

## Backend Traps

- `/health` must not require OpenRouter credentials or database writes.
- Empty/blank titles and messages must return structured JSON errors.
- Unknown conversations must return structured `not_found` errors.
- Message send must persist both user and assistant messages in order.
- Mocked provider tests must prove backend behavior without network or secrets.
- Streaming must emit parseable NDJSON chunks and persist the final assistant response.
- CORS must allow the frontend dev origin and not require permissive credentials.

## Frontend Traps

- API responses must be parsed with `zod`; malformed backend responses must surface a user-visible error.
- Sending a message must show loading state and append assistant output without exposing provider details.
- Assistant markdown must render through `react-markdown` + `remark-gfm`, not raw HTML injection.
- Sidebar conversations and mobile sidebar toggling must remain usable at narrow widths.
- Component tests should mock API calls and avoid a real backend.
- Playwright smoke test should intercept backend API routes and validate send-message UX.

## Verification Commands

- Backend:
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- Frontend:
  - `npm run lint`
  - `npm run typecheck`
  - `npm run test`
  - `npm run build`
  - `npm run test:e2e`

## Environment Assumptions

- Rust binaries may require `C:/Users/loldi/.cargo/bin` in `PATH` for CodeDungeon QA commands.
- Playwright may need Chromium installed; record installation or failure explicitly if needed.
- No verification command may depend on a real `OPENROUTER_API_KEY`.
