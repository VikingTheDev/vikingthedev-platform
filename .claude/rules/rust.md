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

Measure with `oha` against staging, nightly rather than per PR.

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
- **Database:** the PostgreSQL library isn't decided yet. Ask before choosing one.
- **Session store:** whether `tower-sessions` hashes session IDs at rest is unverified. Check before adopting it. The fallback is a hand-rolled store on `fred`.

## Sessions

The cookie is exactly:

```
__Secure-vt_sid=<base64url 256-bit random>; Domain=vikingthe.dev; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=2592000
```

- **Prefix:** not `__Host-`, which forbids `Domain`.
- **SameSite:** not `Strict`, which breaks arrivals from external links.
- **Token:** an opaque token. No JWTs.

**Valkey storage**

- **Session record:** `sess:{sha256(token)}` is a hash of `{user_id, created, last_seen, ua_hash}`.
  - It expires after 30 days.
  - Refresh the expiry at most once an hour.
- **Per-user index:** `user_sess:{user_id}` is a set used to revoke all of a user's sessions.
- **Hashing:** never store the raw token.
- **Persistence:** Valkey runs with AOF on. It holds state, not a disposable cache.

**Logout:** `POST /api/auth/logout` deletes the Valkey key and expires the cookie with the same `Domain`.

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
