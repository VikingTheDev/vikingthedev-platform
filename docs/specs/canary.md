# Spec: canary

- **Status:** Ready
- **Host:** `canary.vikingthe.dev`
- **Related ADRs:** 0001, 0002, 0003, 0006

## Purpose

A second app host that exists only to prove the platform works across subdomains: the shared session, `/api` routing per host, the internal `:8081` session check, CSRF between sibling hosts, and logout everywhere. It stays as the target of the synthetic checks (smoke suite, uptime) after M1.

## Users

The maintainer and automated checks. Anyone can open it; there's no login method until M3. Sessions come from `POST /api/test/login` in the CI stack (`e2e` builds only) and from `api issue-session` in production (human only, plan step 17).

## User stories

- As a logged-in user arriving from `vikingthe.dev`, I see my display name in canary's server-rendered HTML, so the session survives across subdomains.
  - [ ] With a session cookie set on the homepage's host, the HTML of `/` contains the user's `display_name` without any client-side fetch.
  - [ ] Without a cookie, or with an invalid or expired one, the HTML says "anonymous".
  - [ ] If the internal session check takes longer than 50 ms or fails, the page renders as anonymous instead of erroring.
  - [ ] The session cookie has exactly the attributes in ADR 0001 (`rust.md` "Sessions"), with `Domain` from config (`vikingthe.test` in the CI stack).
- As a logged-in user, I can log out from canary, and the session ends on every host.
  - [ ] Pressing "Log out" calls `POST /api/auth/logout` and reloads the page, which then says "anonymous".
  - [ ] On failure (403 or a network error), the page stays as it is and shows an error message.
  - [ ] Afterwards, `/api/me` returns 401 on both hosts, and the homepage's server island renders "anonymous".
- As the platform, I reject cross-host writes.
  - [ ] Sent with Playwright's `APIRequestContext` and explicit `Sec-Fetch-Site` and `Origin` headers, a `POST /api/auth/logout` to canary's `/api` gets 403 `forbidden` when marked `same-site` (from the homepage's origin) or `cross-site`; the same request marked `same-origin` gets 204.
  - [ ] In a real browser, a logged-in homepage page sends a `no-cors` `text/plain` POST to canary's `/api/auth/logout`; afterwards the session still works on both hosts.
- As a visitor, I can go back to the homepage.
  - [ ] A link to the homepage exists, and following it fires no view transition (`pagereveal` with no `event.viewTransition`).
- As a visitor using assistive technology, I can use the page.
  - [ ] axe reports no serious or critical violations.

## Pages and routes

| Route | Purpose | Rendering | Auth required |
|---|---|---|---|
| `/` | Shows "anonymous" or the user's display name, a "Log out" button when logged in, and a link to the homepage | On demand | No |

## Data

None owned. It reads `SessionUser` (`api-contract.md`) from the internal `:8081` `/session`. The display name is personal data; it's rendered only into a `private, no-store` response.

## API

No endpoints of its own. It uses:

| Method | Path | Request | Response | Auth | Notes |
|---|---|---|---|---|---|
| GET | `:8081/session` | The forwarded `Cookie` header | 200 `SessionUser` · 401 | Session | Called by Astro middleware, 50 ms timeout, anonymous on failure |
| POST | `/api/auth/logout` | `{}` | 204 | No | Called from the logout island |

## Auth and security

- `/` sends `Cache-Control: private, no-store` and never `Set-Cookie`.
  - [ ] CI stack: those headers are present on `/`.
  - [ ] Production (M1b step 12): `cf-cache-status` on `/` is `DYNAMIC` or `BYPASS`.
- The logout island sends `Content-Type: application/json`; the browser adds `Sec-Fetch-Site: same-origin`.
- CSP: no `{@html}` or `set:html`.
  - [ ] The built HTML carries a CSP `<meta>` with `script-src` hashes and no `unsafe-inline`, and the logout island still hydrates under it (M1a step 5).
- `security.checkOrigin` on, and `security.allowedDomains` lists canary's host and `canary.vikingthe.test`, so the CI stack tests the image that ships; `.test` is reserved (RFC 6761) (the maintainer, 2026-10-19).
- Hidden from search engines with `<meta name="robots" content="noindex">` (the maintainer, 2026-10-19). No `robots.txt` blocks crawling, because a crawler that can't fetch a page never sees its `noindex`.
  - [ ] CI stack: the HTML of `/` contains that tag.

## Performance

All default budgets for a content route in `frontend.md` apply; the server-rendered TTFB budget is ≤ 400 ms. The largest JS payload is `/`, which carries only the logout island.

- [ ] `size-limit` and Lighthouse CI (lab: LCP, CLS, TBT, script size) pass in the CI stack.

## Out of scope

Any content, styling beyond the defaults, a login form, navigation to other apps, or localization (English only, `lang="en"`; the maintainer, 2026-10-19).

## Open questions

None.
