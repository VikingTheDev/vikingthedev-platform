# ASVS L2 checklist

The platform targets [OWASP ASVS 5.0.0](https://github.com/OWASP/ASVS/tree/v5.0.0/5.0/en) Level 2: every level 1 and level 2 requirement (maintainer, 2026-10-24). Each milestone ticks the chapters it touches, with evidence (`plan.md` §4). Requirement IDs follow ASVS's citation form, `v5.0.0-<chapter>.<section>.<requirement>`; the tables use the short form.

**Status:** **Met** has evidence; **Deviation** is unmet and accepted by the maintainer, with the reason below; **Gap** is unmet and waits on the work its decision names; **Later** names the milestone that delivers it; **N/A** gives the reason.

## Decisions

The maintainer answered these on 2026-10-24, after the M1c review. None is expensive to reverse, so none has an ADR.

1. **13.2.1, static backend credentials: accepted deviation.** PostgreSQL and Valkey authenticate the API with long-lived passwords, and `:8081` accepts any caller on the `coolify` network. Rotating credentials automatically needs a vault or mTLS, which is out of scope on one VPS. The databases are never public, and `:8081` never gets a domain (`production-setup.md`).
2. **13.2.2, least privilege: accepted deviation.** The API connects to PostgreSQL as the image's `POSTGRES_USER`, which the official image makes a superuser, and to Valkey as its default all-commands user.
3. **13.2.4 and 13.2.5, egress allowlist: met at the application layer.** Every outbound destination comes from an environment variable, never from input, and no user-supplied URL is ever fetched, so the code can reach only the configured hosts. Nothing filters egress at the network layer, so a compromised dependency could connect anywhere (accepted on one VPS). Revisit when M3 adds SMTP, when an app first uses R2, or if a feature ever fetches a URL it didn't get from configuration.
4. **13.3.1, secrets management: accepted deviation.** Secrets live in Coolify's locked environment variables, injected at runtime and never in images (CLAUDE.md), rather than in a vault that one maintainer would have to run.
5. **7.1.1, 7.3.1 and 7.3.2, session lifetimes: AAL1, plus a 7-day inactivity timeout.** Password-only login is AAL1. At AAL1, [NIST SP 800-63B-4 §2.1.3](https://pages.nist.gov/800-63-4/sp800-63b/aal/) says the overall timeout should be no more than 30 days and makes an inactivity timeout optional. Sessions last 30 days from login. ASVS still requires an inactivity timeout, so a session also ends after 7 days without a request (`plan.md` step 21a). M3 gives the admin role a shorter lifetime and an inactivity timeout. Revisit if decision 8 moves every user to MFA: that's AAL2, where NIST sets 24 hours overall and 1 hour of inactivity.
6. **7.1.2, concurrent sessions: unlimited.** An account can hold any number of sessions, and `user_sess:<user_id>` tracks them all. M3 adds a page to list and end sessions, with "log out everywhere".
7. **3.4.3, CSP as a response header: add `object-src 'none'; base-uri 'none'` to the edge's header.** Prerendered pages carry the baseline policy only in a `<meta>`, and the edge's added header carries only `frame-ancestors 'none'`. The edge's header can't carry a script allowlist: browsers enforce every policy they receive, so a `script-src` without Astro's per-page hashes would block its inline scripts. The hashes stay in the `<meta>` on prerendered pages. Once the edge change is live, 3.4.3 becomes a deviation: the header policy has `object-src` and `base-uri`, but the hash allowlist is in a `<meta>`, because edge-cached HTML can't carry per-page hashes in a header.
8. **6.3.3, multi-factor authentication: decided at the M3 gate.** ASVS 5.0.0 level 2 requires MFA, or a combination of single factors, for every user, not only admins. D1's password login and D1g's admin second factor were decided against 4.0.3. The M3 gate chooses between a second factor for every user and a recorded deviation for ordinary users. It also takes 6.2.11, a context-specific word list for passwords.

## V13 Configuration (M1c)

| ID | Level | Requirement, in short | Status | Evidence |
|---|---|---|---|---|
| 13.1.1 | 2 | Every communication need, including external services, is documented | Met | `production-setup.md` (PostgreSQL, Valkey, the `:8081` alias, GHCR, the Coolify webhook), `edge-config.md` (Cloudflare), ADR 0003 (the tunnel), ADR 0007 (SMTP, from M3), `decisions.md` D6 (UptimeRobot). No user-supplied URL is ever fetched |
| 13.2.1 | 2 | Backend components authenticate without unchanging credentials | Deviation | Decision 1 |
| 13.2.2 | 2 | Backend connections use least-privilege accounts | Deviation | Decision 2. The app containers run as `nonroot` with `--cap-drop=ALL` (ADR 0009) |
| 13.2.3 | 2 | No default credentials | Met | Coolify generates the PostgreSQL and Valkey passwords (`production-setup.md`) |
| 13.2.4 | 2 | An allowlist defines the external systems the app may reach | Met | Decision 3. The containers' only outbound connections are PostgreSQL, Valkey and the API's `:8081`, all from configuration, never from input. SMTP (M3) and R2 join the list when they arrive |
| 13.2.5 | 2 | The server has an allowlist of where it may send requests or load data | Met | Decision 3 |
| 13.3.1 | 2 | A secrets manager holds backend secrets; none in source or build artifacts | Deviation | Decision 4. No secret is in source or images: CLAUDE.md's rules, `gitleaks` on every commit and in CI, runtime-only Coolify variables, no build `ARG` secrets |
| 13.3.2 | 2 | Access to secrets is least-privilege | Met | GitHub holds only `production`-environment secrets, which PR jobs never see; `COOLIFY_TOKEN` can only deploy, and `CLOUDFLARE_API_TOKEN` can only purge one zone's cache (CLAUDE.md, `production-setup.md`) |
| 13.4.1 | 1 | No source control metadata is deployed or reachable | Met | `.dockerignore` excludes `.git`; images copy only named paths (`docker/*.Dockerfile`); `/.git/HEAD` and `/.git/config` are 404 on production (2026-10-24) |
| 13.4.2 | 2 | Debug modes are off in production | Met | The API is a `--release` build without the `e2e` feature, which only the never-pushed CI-stack image enables (`docker/api.Dockerfile`, `scripts/release`); logs default to `info`; Astro runs with `NODE_ENV=production` |
| 13.4.3 | 2 | No directory listings | Met | `/_astro/`, `/api/` and `/node_modules/` are 404 on production (2026-10-24) |
| 13.4.4 | 2 | HTTP `TRACE` isn't supported | Met | `TRACE` on `/` and `/api/health` returns 405 on both hosts (2026-10-24) |
| 13.4.5 | 2 | Internal docs and monitoring endpoints aren't exposed unless intended | Met | `openapi.json` is generated at build time and not served (404 on production); `/health` and `/api/health` are public on purpose, for the deploy wait, and return only the build SHA |

## V3 Web Frontend Security, the parts the skeleton touches (M1c)

V3.7 arrives with real content in M5 and M6. V3.6 has no level 1 or 2 requirement.

| ID | Level | Requirement, in short | Status | Evidence |
|---|---|---|---|---|
| 3.2.1 | 1 | Responses can't be rendered in the wrong context when requested directly | Met | `/api` answers JSON with `Content-Type: application/json` and, zone-wide, `nosniff` (3.4.4); there are no uploads until M7 |
| 3.2.2 | 1 | Text is rendered as text, not HTML | Met | Astro and Svelte escape interpolated values, including the user's `display_name`; no `{@html}` or `set:html` exists (`security-code.md`) |
| 3.3.1 | 1 | Cookies are `Secure`, with a `__Host-` or `__Secure-` prefix | Met | `__Secure-vt_sid; … Secure` (`crates/api/src/sessions/cookie.rs`, its unit tests, the Playwright cookie test) |
| 3.3.2 | 2 | `SameSite` suits each cookie's purpose | Met | `Lax`, because `Strict` breaks arrivals from external links (`rust.md`); CSRF is handled by Fetch Metadata, not `SameSite`, since every subdomain is same-site (3.5.1) |
| 3.3.3 | 2 | `__Host-` unless the cookie is meant for other hosts | Met | The session cookie is shared across `*.vikingthe.dev` by design (ADR 0001 §2), and `__Host-` forbids `Domain` |
| 3.3.4 | 2 | Session cookies are `HttpOnly`, and the token travels only in `Set-Cookie` | Met | `HttpOnly` (`cookie.rs`); no response body carries the token. The maintainer-only `api issue-session` CLI prints one to the operator's terminal, never over HTTP (`plan.md` step 17) |
| 3.4.1 | 1 | HSTS on every response, at least 1 year, including subdomains | Met | `max-age=63072000; includeSubDomains; preload` from the zone-wide Cloudflare rule (`edge-config.md`), asserted by `e2e/tests/edge.spec.ts` after every deploy |
| 3.4.2 | 1 | CORS `Access-Control-Allow-Origin` is fixed or allowlisted | Met | No CORS at all (ADR 0001); a cross-origin request to `/api/me` gets no `Access-Control-*` header (2026-10-24) |
| 3.4.3 | 2 | A CSP response header with `object-src 'none'`, `base-uri 'none'` and an allowlist, nonces or hashes | Gap | Decision 7: waits on the maintainer's change to the edge rule, then becomes a deviation. The shared baseline in `@vikingthedev/config/csp` with Astro's script hashes is a header on on-demand pages but only a `<meta>` on prerendered ones (`security-code.md`, `plan.md` step 5) |
| 3.4.4 | 2 | `X-Content-Type-Options: nosniff` on every response | Met | Zone-wide Cloudflare rule, asserted by `edge.spec.ts` |
| 3.4.5 | 2 | A referrer policy | Met | `strict-origin-when-cross-origin`, zone-wide, asserted by `edge.spec.ts` |
| 3.4.6 | 2 | CSP `frame-ancestors` on every response | Met | `frame-ancestors 'none'` added zone-wide, except Coolify's, Beszel's and SSH's own hosts (`edge-config.md`), asserted by `edge.spec.ts` |
| 3.5.1 | 1 | Without CORS preflight, cross-origin requests to sensitive functions are rejected | Met | The CSRF middleware requires `Sec-Fetch-Site: same-origin`, or else an exact `Origin` allowlist and `Content-Type: application/json` (`crates/api/src/csrf.rs`, `rust.md`); `e2e/tests/csrf.spec.ts` shows `same-site` and `cross-site` get 403 while `same-origin` succeeds |
| 3.5.2 | 1 | If relying on preflight, no request bypasses it | N/A | The API doesn't rely on CORS preflight (3.5.1) |
| 3.5.3 | 1 | Sensitive functions use non-safe methods | Met | The middleware guards every method except GET, HEAD and OPTIONS (`csrf.rs`); logout is `POST` (`api-contract.md`) |
| 3.5.4 | 2 | Separate applications live on separate hostnames | Met | One subdomain per app (ADR 0001 §1) |
| 3.5.5 | 2 | `postMessage` checks origin and syntax | N/A | No code uses `postMessage` |

## V7 Session Management, the parts the skeleton touches (M1c)

Login, re-authentication, account changes and the admin app arrive in M3: 7.2.4 for real logins, 7.4.2, 7.4.3, 7.4.5, 7.5.1, 7.5.2 and 7.6.2. 7.1.3 and 7.6.1 apply only if OAuth sign-in lands (D1).

| ID | Level | Requirement, in short | Status | Evidence |
|---|---|---|---|---|
| 7.1.1 | 2 | Inactivity and absolute lifetimes are documented and justified against NIST SP 800-63B | Met | Decision 5: 30 days absolute, justified as AAL1 (also `rust.md`, step 2 gate), and 7 days of inactivity |
| 7.1.2 | 2 | The number of concurrent sessions per account, and the behavior at the limit, are documented | Met | Decision 6: unlimited, so there's no behavior at a limit |
| 7.2.1 | 1 | Session tokens are verified by a trusted backend | Met | Only Rust validates sessions; Astro asks `:8081/session` (ADR 0001 §2, `packages/session`) |
| 7.2.2 | 1 | Session tokens are dynamically generated, not static secrets | Met | A new random token per session (`crates/api/src/sessions/token.rs`) |
| 7.2.3 | 1 | Reference tokens come from a CSPRNG with at least 128 bits of entropy | Met | 256 bits from `getrandom` (`token.rs`, `TOKEN_BYTES = 32`); Valkey stores only `sha256(token)` (`store.rs`, step 2's tests against real Valkey) |
| 7.2.4 | 1 | Authentication issues a new token and ends the current one | Later (M3) | Login arrives in M3 (D1). The skeleton's only issuers, `api issue-session` and the CI-only test login, always create a new session |
| 7.3.1 | 2 | An inactivity timeout enforces re-authentication | Gap | Decision 5: 7 days, waiting on `plan.md` step 21a. M3 adds a shorter one for the admin role |
| 7.3.2 | 2 | An absolute lifetime enforces re-authentication | Met | Decision 5. Enforced at 30 days from login, never extended, in Valkey's TTL, the stored `created_at` check and the cookie's `Max-Age` (`store.rs`, `cookie.rs`, unit tests) |
| 7.4.1 | 1 | Terminated sessions can't be used again | Met | Logout deletes the Valkey record for every token sent and expires the cookie (`routes.rs`, `store.rs`); after logout, `/api/me` is 401 on both hosts (CI stack, and production in `plan.md` step 17) |
| 7.4.4 | 2 | Pages that require login show a visible logout | N/A | No page requires a login yet. Canary shows a logout button when logged in (`LogoutButton.svelte`); M3's account pages must carry one |
