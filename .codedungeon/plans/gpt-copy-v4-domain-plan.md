# gpt-copy-v4 Domain Plan

PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: 3cfc7da06128fb69b4457625bdb2ea6a46298c99ef604ed165dae89855c63cbd
PROJECT_RULES_READ: yes

## Repository Map

- Root:
  - `.env.example`
  - `README.md`
  - `.gitignore`
  - CodeDungeon runtime artifacts under `.codedungeon/`
- `backend/`:
  - Rust 2024 Axum API.
  - SQLite persistence and embedded migrations.
  - OpenRouter HTTP client and mockable provider trait.
  - Backend tests.
- `frontend/`:
  - Next.js App Router app.
  - Tailwind styling.
  - Component and Playwright tests.

## Domain Boundaries

- Backend is the only OpenRouter trust boundary and must never expose provider secrets.
- Frontend talks only to backend API URLs configured with `NEXT_PUBLIC_API_BASE_URL`.
- SQLite schema is backend-owned; frontend receives DTOs only.
- Error shape is backend-owned and frontend validates it with `zod`.
- Streaming protocol is newline-delimited JSON events emitted by backend and consumed by native `fetch`.

## Specialist Roles

- Backend planner/reviewer:
  - Axum routing, structured errors, sqlx persistence, OpenRouter client, tests.
- Frontend planner/reviewer:
  - Chat UI behavior, responsive layout, TanStack Query, markdown safety, browser tests.
- QA planner:
  - Defines minimal reliable verification set and records environment assumptions.
- Review personas:
  - Spec reviewer checks prompt coverage.
  - Security reviewer checks secret handling and server-side proxy boundary.
  - Saboteur reviewer checks persistence/streaming edge cases.
  - New-hire reviewer checks maintainability and run instructions.

## Implementation Interfaces

- Backend response DTOs:
  - `ConversationDto { id, title, created_at, updated_at }`
  - `MessageDto { id, conversation_id, role, content, created_at }`
  - `SendMessageResponse { conversation, user_message, assistant_message }`
  - `ApiErrorBody { error: { code, message, details? } }`
- Frontend API functions:
  - `listConversations()`
  - `createConversation(title?)`
  - `listMessages(conversationId)`
  - `sendMessageStream(conversationId, content, callbacks)`

## Review Evidence Plan

- Persona findings will be written as JSON files under `.codedungeon/reviews/adv-review/`.
- `review-manifest.json` will declare the persona files.
- `./.codex/bin/codedungeon review run` will aggregate and render review evidence.
- `./.codex/bin/codedungeon review post` will post the generated review evidence to the PR.
