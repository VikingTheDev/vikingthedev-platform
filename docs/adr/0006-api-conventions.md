# ADR 0006: API conventions

- **Status:** Proposed
- **Date:** 2026-10-18
- **Decider:** Solo maintainer

## Context

ADR 0001 fixes the tooling: `utoipa` → OpenAPI → `openapi-typescript` → `openapi-fetch`. It doesn't fix the shapes those tools carry. The maintainer wants frontend and backend to agree on data models before code exists. The endpoint draft is `docs/api-contract.md`. This ADR fixes the rules every endpoint follows, and keeps ADR 0001's tooling.

## Decision (proposed)

| Topic | Rule |
|---|---|
| **Errors** | RFC 9457 `application/problem+json`. `type` is a stable URI, `https://vikingthe.dev/problems/<code>`, that clients switch on. `title` is fixed per type. `detail` is safe for users only. `instance` is the request ID. Validation errors add `errors: [{ field, code }]`. Internal text never reaches clients. |
| **IDs** | UUIDv7, as strings in JSON, wrapped in Rust newtypes (`UserId`). |
| **Timestamps** | RFC 3339 in UTC with a `Z` suffix, in fields named `*_at`. |
| **Field names** | snake_case, serde's default (`rust.md`). |
| **Pagination** | Cursor-based. `?limit=` defaults to 20, with a maximum of 100. `?cursor=` is opaque. Responses are `{ items, next_cursor }`, where `next_cursor: null` means the end. No offset pagination. |
| **Idempotency** | An `Idempotency-Key` header (IETF draft-ietf-httpapi-idempotency-key-header) is required on POSTs that create resources or send email. The response is stored in Valkey for 24 h, keyed by caller and key, with a request-body hash. A reused key with a different body gets 422 `idempotency_conflict`. |
| **Caching** | User-dependent responses send `private, no-store` (ADR 0001). |
| **Versioning** | No URL version prefix. Every consumer lives in this repo. |
| **Breaking changes** | `oasdiff breaking` runs between `main`'s and the PR's `openapi.json`, and any breaking change fails CI. The API and the apps deploy separately, so breaking changes are done as expand then contract: add, deploy, migrate consumers, then remove in a later PR. |
| **oasdiff installation** | A pinned release binary with a checksum check in the workflow. Not a third-party GitHub Action. |

## Consequences

- **Positive:**
  - one error shape, which the generated types turn into a discriminated union;
  - IDs that sort by time;
  - safe retries of contact and auth emails;
  - no deploy window where the live frontends break.
- **Negative:**
  - expand-then-contract takes two PRs for every breaking change;
  - idempotency storage adds Valkey keys, which are cheap.
