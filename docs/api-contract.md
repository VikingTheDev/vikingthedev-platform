# API contract (draft)

- **Status:** Draft. It follows the conventions in ADR 0006.
- **Scope:** platform endpoints for milestones 1–3, plus the homepage endpoints in M5. App endpoints go in each app's spec.
- **Source of truth after implementation:** the `utoipa`-generated `packages/api-types/openapi.json` (ADR 0001). This draft is the target that code is written against. If the two ever disagree, the generated file wins, and this draft is updated in the same commit.

## Shared models

```ts
type UserId = string;     // UUIDv7
type Timestamp = string;  // RFC 3339, UTC, "Z" suffix

interface User {
  id: UserId;
  display_name: string;
  created_at: Timestamp;
}

// RFC 9457, content type application/problem+json
interface Problem {
  type: ProblemType;      // stable; clients switch on this
  title: string;          // fixed per type, never internal text
  status: number;
  detail?: string;        // safe, user-facing only
  instance?: string;      // "urn:uuid:<request-id>" (RFC 9457 wants a URI reference)
}

type ProblemType =
  | "https://vikingthe.dev/problems/unauthenticated"   // 401
  | "https://vikingthe.dev/problems/reauthentication_required" // 401, login older than 15 min
  | "https://vikingthe.dev/problems/forbidden"         // 403, includes CSRF rejection
  | "https://vikingthe.dev/problems/not_found"         // 404
  | "https://vikingthe.dev/problems/validation"        // 422
  | "https://vikingthe.dev/problems/idempotency_conflict" // 422
  | "https://vikingthe.dev/problems/rate_limited"      // 429, with Retry-After
  | "https://vikingthe.dev/problems/unavailable"       // 503
  | "https://vikingthe.dev/problems/internal";         // 500

interface SessionUser {
  user: User;
  role: "user" | "admin";
  grants: string[];       // app names, e.g. "stash"
}

interface Page<T> {
  items: T[];
  next_cursor: string | null;
}
```

- **Validation errors** add an `errors` extension: `{ field: string; code: string }[]`. `code` is a stable snake_case value, for example `too_long`.
- **Every response** carries `X-Request-Id`. On problems, `instance` is the same ID as `urn:uuid:<id>`.
- **User-dependent responses** send `Cache-Control: private, no-store`.

## Public listener (`:8080`, routed at `/api` on every host)

Writes need `Sec-Fetch-Site: same-origin`, or else an allowlisted `Origin` plus `Content-Type: application/json`. Otherwise they get 403 `forbidden` (ADR 0001).

| Method | Path | Request | Response | Auth | Milestone | Notes |
|---|---|---|---|---|---|---|
| GET | `/api/health` | — | 200 `{ status: "ok", sha: string }` | No | M1 | Also used by the deploy wait and the uptime check. `no-store` |
| GET | `/api/me` | — | 200 `User` · 401 | Session | M1 | |
| POST | `/api/auth/logout` | `{}` | 204, plus an expiring `Set-Cookie` with `Domain` | No | M1 | Idempotent. 204 with or without a session |
| POST | `/api/test/login` | `{ user_id: UserId }` | 204, plus `Set-Cookie` | No | M1 | **Exists only in builds with the `e2e` feature** (D11). Production returns 404 |
| GET | `/api/status` | — | 200 `{ services: { name: string; state: "up" \| "degraded" \| "down"; checked_at: Timestamp }[] }` | No | M5 | Cached for 30 s in the API |
| POST | `/api/contact` | `{ name: string; email: string; message: string; website: string }` and an `Idempotency-Key` header | 202 · 422 · 429 | No | M5 | `website` is the honeypot: it must be empty, and a filled value gets a silent 202 |
| POST | `/api/rum` | `{ name: "LCP" \| "INP" \| "CLS" \| "TTFB"; value: number; route: string; host: string }[]` | 204 · 422 | No | M5 | Sent with `fetch` and `keepalive`. No identifiers. Rate-limited. At most 10 entries. `host` must be a known host. `route` is a path only (no query or fragment), at most 200 characters, so no PII from query strings is stored |
| GET | `/api/me/export` | — | 200 JSON with every personal field | Session | M3 | GDPR (D8) |
| DELETE | `/api/me` | `{ password?: string }` | 204, plus an expiring `Set-Cookie` | Session, plus a login within the last 15 minutes | M3 | Revokes every session. Deletes OAuth links too |
| GET | `/api/me/sessions` | — | 200 `{ items: { id: string; created_at: Timestamp; last_seen_at: Timestamp; current: boolean }[] }` | Session | M3 | Not paginated: a user has few sessions. `id` is a short hash prefix, never the token |
| DELETE | `/api/me/sessions/{id}` | — | 204 · 404 | Session | M3 | Only the owner's own sessions (IDOR test) |

### Auth endpoints (depend on D1)

These are sketched for the recommendation in ADR 0004: email and password, then GitHub and Discord OAuth, with the magic link used as the password reset. They change if D1 lands elsewhere.

| Method | Path | Request | Response | Notes |
|---|---|---|---|---|
| POST | `/api/auth/signup` | `{ email: string; password: string; display_name: string }` with an `Idempotency-Key` | 202 | Always 202. Creates only a pending record (ADR 0004); no account exists until verification. If the email already has an account, its owner gets a notice instead. Emails are sent in the background, so timing reveals nothing |
| POST | `/api/auth/verify-email` | `{ token: string; password: string }` | 204, plus a new session cookie · 401 | The password must match the one entered at that sign-up, which blocks pre-account hijacking. Single-use token, valid 24 hours, stored hashed |
| POST | `/api/auth/login` | `{ email: string; password: string }` | 204, plus a new session cookie · 401 `unauthenticated` | Same 401 for an unknown email, a wrong password or a disabled account. An unknown email still runs a dummy hash |
| POST | `/api/auth/password-reset` | `{ email: string }` with an `Idempotency-Key` | 202 | Always 202. Rate-limited per IP and per email |
| POST | `/api/auth/password-reset/confirm` | `{ token: string; password: string }` | 204, plus a new session cookie · 401 · 422 | Single-use token, valid 15 minutes. Revokes all other sessions |
| GET | `/api/auth/oauth/{provider}/start` | `?return_to=` | 302 to the provider | `provider` is `"github"` or `"discord"`. Stores `state`, the PKCE verifier and `return_to` in Valkey for 10 minutes, and sets a short-lived `HttpOnly` pre-auth cookie bound to that `state`. `return_to` must be a relative path or an allowlisted `https://*.vikingthe.dev` URL. Served only on the apex, the one registered callback host |
| GET | `/api/auth/oauth/{provider}/callback` | `?code=&state=` | 302 to the stored `return_to`, plus a new session cookie · 403 if `state` or the pre-auth cookie don't match | A top-level GET navigation, so Fetch Metadata CSRF doesn't apply; `state` plus the browser-bound cookie stop login CSRF. Never links to an existing account by email. If the provider's email belongs to another account, the redirect carries a generic "log in with your existing method, then link" message, the same whether or not the address exists |
| POST | `/api/me/identities/{provider}/link` | `{}` | 200 `{ redirect_url: string }` | Session required. Starts linking from a logged-in session |
| DELETE | `/api/me/identities/{provider}` | — | 204 · 422 if it's the last login method | |

### Admin endpoints (`admin` role required; D18, D19)

Every admin endpoint checks the role in the domain layer. It accepts requests only on `Host: admin.vikingthe.dev` with a valid `Cf-Access-Jwt-Assertion` (403 `forbidden` otherwise). Every non-GET also requires a login within the last 15 minutes (401 `reauthentication_required` otherwise) and writes an `admin_audit` row.

| Method | Path | Request | Response | Notes |
|---|---|---|---|---|
| GET | `/api/admin/users` | `?q=&limit=&cursor=` | 200 `Page<AdminUser>` | `AdminUser` extends `User` with `email`, `role`, `disabled_at` and `grants` |
| PATCH | `/api/admin/users/{id}` | `{ disabled: boolean }` | 200 `AdminUser` | Disabling revokes every session |
| PUT | `/api/admin/users/{id}/grants/{app}` | `{}` | 204 | Idempotent |
| DELETE | `/api/admin/users/{id}/grants/{app}` | — | 204 | |
| GET | `/api/admin/audit` | `?limit=&cursor=` | 200 `Page<AuditEntry>` | |

## Internal listener (`:8081`, Docker network only)

| Method | Path | Request | Response | Notes |
|---|---|---|---|---|
| GET | `/health` | — | 200 `{ status: "ok", sha: string }` | |
| GET | `/session` | The forwarded `Cookie` header | 200 `SessionUser` · 401 | Astro middleware calls it with a 50 ms timeout. Includes role and grants, so gated apps can authorize their server-rendered pages without a second call. Budget: p99 ≤ 5 ms |
