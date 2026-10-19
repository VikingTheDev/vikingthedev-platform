---
paths:
  - "crates/**"
  - "Cargo.toml"
  - "Cargo.lock"
---

# Rust rules: backend and API

## Toolchain and lints

Use edition 2024 with `rustfmt` defaults. Configure lints once, in the workspace:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
unwrap_used = "warn"
expect_used = "warn"
dbg_macro = "warn"
todo = "warn"
print_stdout = "warn"
```

Every `#[allow]` is local and gives a reason, for example `#[allow(clippy::cast_possible_truncation, reason = "ids < 2^32")]`.

## Errors

- **Domain errors** are `thiserror` enums.
- **The API crate** has one `AppError` that implements `IntoResponse`. Handlers return `Result<T, AppError>` and use `?`.
- **What clients see:** log the internal error text and return a stable error code. Internal text never reaches clients.
- **`anyhow`** is allowed only in startup code and in tooling (`openapi-gen`).
- **No `unwrap` or `expect` in request paths.** `expect("reason")` is fine at startup.

## Naming and types

These follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html).

- **Casing:** RFC 430 (C-CASE).
- **Conversion names:** `as_`, `to_`, `into_` (C-CONV).
- **Getters:** no `get_` prefix (C-GETTER).
- **Conversions:** implement `From`, `TryFrom` and `AsRef` rather than ad-hoc constructors (C-CONV-TRAITS).
- **Domain values:** wrap them in newtypes (`UserId`, `SessionToken`, `EmailAddress`) that validate in their constructor (C-NEWTYPE).
- **Unclear arguments:** use an enum instead of a `bool` or `Option` whose meaning isn't obvious at the call site (C-CUSTOM-TYPE).

## Code shape

- Prefer iterator chains when they read better than loops.
- Borrow instead of cloning. Take `&str` and `&[T]`, and return owned types.
- Match exhaustively on your own enums. No `_ =>` catch-all that would hide new variants.

## axum

- **Thin handlers:** extract, call a domain function, and map the result to a response. Business logic never lives in a handler.
- **Routes:** each feature module exposes `pub fn routes() -> Router<AppState>`, and `main` merges them.
- **State:** `AppState` holds clients that are cheap to clone (pools, plus config behind an `Arc`). Pass it with `State<AppState>`.
- **DTOs:**
  - Keep them separate from database rows and domain types.
  - They derive `serde` and `utoipa::ToSchema`.
  - Request DTOs use `#[serde(deny_unknown_fields)]`.
  - JSON fields stay in serde's default snake_case.

## Docs

Doc comments go only on public items whose behavior the signature doesn't show: one sentence, plus an `# Errors` line where it applies.

## Performance budget

A budget regression is a failing test, not a judgment call.

| Metric | Target |
|---|---|
| Reads | p50 ≤ 10 ms, p99 ≤ 50 ms |
| Session check | p99 ≤ 5 ms |

Measure with `oha` from the maintainer's machine against production `/api`, and from inside the VPS against `:8081`, never during backups (ADR 0002). Server-side timings come from `tracing`.

## Caching on API responses

- User-dependent responses send `Cache-Control: private, no-store`.
- Never set `Set-Cookie` on a response meant to be cached. That includes login and logout.
- Private R2 objects go through short-lived presigned URLs and are never cached.

## Async, logging and tests

- **Blocking work:** never block in async code. CPU-heavy work such as password hashing goes in `tokio::task::spawn_blocking`.
- **Locks:** never hold a lock across an `.await`.
- **Logging:**
  - Use `tracing` only, never `println!`.
  - Put `#[tracing::instrument(skip_all, fields(…))]` on handlers and domain entry points.
- **Unit tests** go in `#[cfg(test)] mod tests`, next to the code.
- **HTTP tests** live in `crates/api/tests/` and drive the `Router` directly with `tower::ServiceExt::oneshot`.
- **Database:** Diesel through `diesel-async` with a pool in `AppState` (ADR 0005). Map query errors to domain `thiserror` enums. Migrations run at startup under an advisory lock, and are expand-only; destructive changes wait for a later release (D16).
- **Session store:** hand-rolled on the `redis` crate, sharing one `ConnectionManager`. `tower-sessions` was checked on 2026-10-19 and rejected: it stores raw IDs and has no per-user index (M1 gate).

## Sessions

The cookie is exactly:

```
__Secure-vt_sid=<base64url 256-bit random>; Domain=vikingthe.dev; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=2592000
```

- **Prefix:** not `__Host-`, which forbids `Domain`.
- **SameSite:** not `Strict`, which breaks arrivals from external links.
- **Token:** an opaque token. No JWTs.

**Valkey storage**

- **Session record:** `sess:{sha256(token)}` is a hash of `{user_id, created_at, last_seen_at, last_auth_at}`, with timestamps in Unix seconds. `last_auth_at` feeds ADR 0004's recent-login check. No `ua_hash`: nothing uses it, and user agents change on every browser update (M1 gate, 2026-10-19).
  - It expires 30 days after login, matching the cookie's `Max-Age`, and is never extended (step 2 gate, 2026-10-19).
  - `last_seen_at` is written at most once an hour.
- **Per-user index:** `user_sess:{user_id}` is a set used to revoke all of a user's sessions.
- **Hashing:** never store the raw token.
- **Persistence:** Valkey runs with AOF on. It holds state, not a disposable cache.

**Logout:** `POST /api/auth/logout` deletes the Valkey key and expires the cookie with the same `Domain`.

**Several session cookies** on one request mean one was planted from a sibling subdomain or through XSS. Logout revokes every well-formed token sent. Session loading treats more than one distinct token as unauthenticated, so a planted cookie can't take over a login (step 2 review, 2026-10-19).

## API routing and CSRF

**Listeners**

- **`:8080`** is public and sits behind Traefik. Each app host has an `/api` path domain pointing to it.
- **`:8081`** is internal. It is reachable on the Docker network only, never gets a domain, and serves SSR and server islands.

**Known risks** (ADR 0001):
- Coolify might not route `/api` and `/` on the same host to different resources. The fallback is `api.vikingthe.dev` with CORS and an exact `Origin` allowlist. Ask first.
- How a Coolify Docker Image resource reaches `:8081` is unconfirmed. The fallback is a shared Docker network, attached manually.

**CSRF middleware** covers POST, PUT, PATCH and DELETE on `:8080`, and runs **before** auth.

- Require `Sec-Fetch-Site: same-origin`.
- If that header is absent, require both an exact `Origin` allowlist and `Content-Type: application/json`.
- Reject `same-site` with a 403. Every subdomain is same-site, so `SameSite` cookies don't protect against this.
- `:8081` skips Fetch Metadata checks.

## API conventions (ADR 0006)

- **Errors:** RFC 9457 `application/problem+json`. `type` is a stable URI, `https://vikingthe.dev/problems/<code>`; `title` is fixed per type; `detail` is user-safe only; `instance` is `urn:uuid:<request-id>`. Validation errors add `errors: [{ field, code }]`.
- **IDs:** UUIDv7 in newtypes, strings in JSON.
- **Timestamps:** RFC 3339 in UTC with `Z`, in fields named `*_at`.
- **Pagination:** cursor-based, `?limit=` (default 20, max 100) and an opaque `?cursor=`, returning `{ items, next_cursor }`. No offsets.
- **Idempotency:** POSTs that create resources or send email require `Idempotency-Key`. Responses are kept in Valkey for 24 h, keyed by caller and key, with a body hash; a reused key with a different body gets 422 `idempotency_conflict`.
- **Versioning:** no URL prefix.
- **Breaking changes:** `oasdiff breaking` fails CI. Do expand then contract across separate PRs, because the API and the apps deploy separately.

## API contract

- **Generation:** `utoipa` derives OpenAPI from the handlers, and `openapi-typescript` generates `packages/api-types`. The frontend client is `openapi-fetch`.
- **Changing a request or response shape:** change the Rust code, regenerate the types, fix the frontends, and commit all three together.
- **The contract check:**

  ```sh
  cargo run -p openapi-gen > packages/api-types/openapi.json
  pnpm --filter api-types generate
  git diff --exit-code packages/api-types
  pnpm -r check
  ```
