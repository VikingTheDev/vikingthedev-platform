# Spec: homepage (M1 skeleton)

- **Status:** Ready
- **Host:** `vikingthe.dev`
- **Related ADRs:** 0001, 0002, 0003, 0006

This is the minimal M1 version: just enough to exercise every rendering mode. M5 replaces it with the real homepage (`design-sequence.md`), and this spec is rewritten then.

## Purpose

Prove the homepage's rendering modes and edge caching: a prerendered page cached at the edge, a server island that personalizes it, and one on-demand route, with view transitions between them.

## Users

Anyone. There's no login method until M3. Sessions come from `POST /api/test/login` in the CI stack (`e2e` builds only) and from `api issue-session` in production (human only, plan step 17).

## User stories

- As a visitor, I get the front page from the Cloudflare cache.
  - [ ] CI stack: the HTML of `/` contains no user data, and its response has no `Set-Cookie`.
  - [ ] Production (M1b step 12): the second request to `/` shows `cf-cache-status: HIT`.
- As a logged-in user, I see that I'm logged in, even on the cached page.
  - [ ] A server island on `/` shows the user's display name, or "anonymous" without a valid session.
  - [ ] CI stack: the server-island response carries `Cache-Control: private, no-store` and no `Set-Cookie`.
  - [ ] Production (M1b step 12): `/_server-islands/*` shows `cf-cache-status: DYNAMIC` or `BYPASS`.
  - [ ] The island's `slot="fallback"` reserves the same fixed size as the rendered island, so swapping it in causes no layout shift.
- As a logged-in user, I can see my account page.
  - [ ] `/account` renders on demand and shows the display name, or a "not logged in" message without redirecting (there's no login page yet).
  - [ ] CI stack: `/account` carries `Cache-Control: private, no-store`; in production it shows `DYNAMIC` or `BYPASS`.
- As a visitor, I can move between pages with a view transition.
  - [ ] Navigating from `/` to `/account` fires a view transition (`pagereveal` with `event.viewTransition` set).
  - [ ] A link to canary exists, and following it fires none.
- As the deploy pipeline, I can see which build is live.
  - [ ] `GET /health` returns `{ "status": "ok", "sha": "<git sha>" }` with `Cache-Control: no-store`; the SHA is the image's build argument, and `dev` outside CI builds (step 4 gate, 2026-10-19).
- As a visitor using assistive technology, I can use every page.
  - [ ] axe reports no serious or critical violations on `/` and `/account`.

## Pages and routes

| Route | Purpose | Rendering | Auth required |
|---|---|---|---|
| `/` | Placeholder front page: "vikingthe.dev" and "Under construction.", with a session server island | Prerendered, plus a server island | No |
| `/account` | Shows the session user, or "not logged in" | On demand | No |
| `/health` | The build's git SHA, for the deploy wait (`ci-deploy.md`) | On demand | No |

## Data

None owned. The server island and `/account` read `SessionUser` from `:8081` `/session`. The display name is personal data and only appears in `private, no-store` responses.

## API

No endpoints of its own. It uses:

| Method | Path | Request | Response | Auth | Notes |
|---|---|---|---|---|---|
| GET | `:8081/session` | The forwarded `Cookie` header | 200 `SessionUser` · 401 | Session | Called by Astro middleware, 50 ms timeout, anonymous on failure |

## Auth and security

- `/` is public and cacheable. The server-island response and `/account` send `Cache-Control: private, no-store`.
- CSP: no `{@html}` or `set:html`.
  - [ ] The built HTML carries a CSP `<meta>` with `script-src` hashes and no `unsafe-inline`, and the server island still loads under it (M1a step 5).
- `security.checkOrigin` on, and `security.allowedDomains` lists `vikingthe.dev` and `vikingthe.test`, so the CI stack tests the image that ships; `.test` is reserved (RFC 6761) (the maintainer, 2026-10-19). Astro probably reads this at build time; M1a step 1 confirms it for Astro 7.
- Hidden from search engines until M5, so the placeholder isn't indexed: every page carries `<meta name="robots" content="noindex">` (the maintainer, 2026-10-19). No `robots.txt` blocks crawling, because a crawler that can't fetch a page never sees its `noindex`. M5's checks require the tag to be gone.
  - [ ] CI stack: the HTML of `/` and `/account` contains that tag.

## Performance

All default budgets for content routes in `frontend.md` apply: JS ≤ 15 KB gzip, lab LCP ≤ 2.0 s, CLS ≤ 0.05 and TBT ≤ 100 ms; TTFB ≤ 200 ms from the edge cache (production only) and ≤ 400 ms for `/account`. The largest JS payload is `/`, with the server-island loader.

- [ ] `size-limit` and Lighthouse CI (lab) pass in the CI stack.

## Out of scope

Real content, design tokens and components (M4–M5), the status section, the contact form and RUM (M5). Localization: the site is English only, `lang="en"` (the maintainer, 2026-10-19).

## Open questions

None.
