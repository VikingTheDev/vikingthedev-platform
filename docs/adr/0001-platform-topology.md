# ADR 0001: Platform topology for the vikingthe.dev suite

- **Status:** Accepted
- **Date:** 2026-10-18
- **Decider:** Solo maintainer

## Context

vikingthe.dev is a suite of websites.

**Stack**

| Layer | Choice |
|---|---|
| Frontend | Astro 7 + Svelte 5 (`@astrojs/svelte`, `@astrojs/node`) |
| Backend | Rust (axum 0.8) |
| Data | PostgreSQL and Valkey, each a separate Coolify resource |
| Objects | Cloudflare R2 |
| Deploy | Coolify on Docker |
| CI | GitHub Actions |

**Shape of the suite**

- **Apps:** the homepage at `vikingthe.dev` today. About 5–6 apps within 12 months, mostly at `<name>.vikingthe.dev`.
- **Releases:** each app is released on its own.
- **Shared code:** a shared Svelte component library.
- **Team:** one maintainer.

**Requirements**

- Performance is the primary metric.
- A logged-in session on the homepage must survive navigation to `<name>.vikingthe.dev`.

**Assumption:** DNS is proxied through Cloudflare (orange cloud).

## Decision

### 1. URL layout: subdomains of one registrable domain

- **Placement:** apps live at `<name>.vikingthe.dev`.
- **Session survives:** one session cookie with `Domain=vikingthe.dev` reaches every app (draft-ietf-httpbis-rfc6265bis-22 §4.1.2.3). All apps are same-site, so `SameSite` does not block it (§2.3).
- **No animation between apps:** cross-document view transitions require both documents to be same-origin (CSS View Transitions Module Level 2, *check eligibility*, step 4). Subdomains are different origins, so navigating from the homepage to an app is a plain, unanimated navigation.
- **Animation inside apps:** native cross-document view transitions (`@view-transition { navigation: auto; }`), set in the shared layout from `packages/ui`.
  - They ship no JavaScript.
  - Supported in Chrome/Edge 126+ and Safari 18.2+. Firefox gets instant, unanimated navigation.
  - Astro's `<ClientRouter />` is not used.
- **Escape hatch:** an app that needs animated entry from the homepage is mounted at `vikingthe.dev/<app>` instead. The domain cookie covers that path, so nothing else changes.

### 2. Session

**Cookie**

```
Set-Cookie: __Secure-vt_sid=<base64url 256-bit random>; Domain=vikingthe.dev; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=2592000
```

- **Prefix:** `__Secure-`. `__Host-` forbids `Domain` (§4.1.3.2), so it cannot be used.
- **`SameSite=Lax`:** `Strict` would show users arriving from external links as logged out on the first page load.
- **Opaque token, not a JWT:** a session can be revoked with one delete, and the cookie stays small on every request.

**Store**

- **Owner:** Rust owns all session logic. Sessions live in Valkey.
- **Session record:** `sess:{sha256(token)}` is a hash of `{user_id, created, last_seen, ua_hash}`.
  - It expires after 30 days.
  - Expiry is refreshed at most once per hour.
- **Per-user index:** `user_sess:{user_id}` is a set of that user's sessions, used for revoke-all.
- **Hashed keys:** keys are token hashes, so a dump of Valkey contains no usable tokens.
- **Persistence:** Valkey runs with AOF on (`appendonly yes`, `appendfsync everysec`), so a restart does not log everyone out.

**Routing**

- **Path:** the Rust service gets a Coolify path domain on every app host: `https://vikingthe.dev/api`, `https://<name>.vikingthe.dev/api`, and so on.
- **Same-origin calls:** every browser call to the API is same-origin. That means no CORS and no preflight on JSON writes.
- **Listeners:** Rust exposes two.
  - `:8080`, public, routed by Traefik.
  - `:8081`, internal, reachable only on the Coolify Docker network and never given a domain.

**Validation in Astro**

- **Where:** shared middleware (`src/middleware.ts`) runs only on on-demand routes. Prerendered routes do nothing at request time.
- **How:** the middleware forwards the `Cookie` header to `GET http://<api-internal>:8081/session`.
  - Timeout is 50 ms. A timeout is treated as anonymous.
  - The result goes into `Astro.locals.user`.
- **Caching:** responses that depend on the user send `Cache-Control: private, no-store`.

**Navigating from the homepage to an app**

1. The browser sends the cookie to the app.
2. The app's middleware validates it over the internal network.
3. The HTML arrives already personalized.

**Logout:** `POST /api/auth/logout` deletes the Valkey key and expires the cookie with the same `Domain`. That logs the user out of every app.

**CSRF**

`SameSite` does not stop one subdomain from forging requests to another, because they are same-site. Rust middleware therefore protects unsafe methods, and it runs **before** authentication.

- **Public listener:**
  - Require `Sec-Fetch-Site: same-origin`.
  - If that header is absent, require an `Origin` from an exact allowlist and `Content-Type: application/json`.
  - Because all legitimate calls go to the app's own `/api`, a `same-site` value is always an attack.
- **Writes are JSON:** every write, including login and logout, is a JSON `fetch()` call, never a native `<form>` post.
- **Internal listener:** no Fetch Metadata check. SSR and server islands call it, and they send no browser headers. The network boundary is what keeps it private.
- **Astro:**
  - `security.checkOrigin` stays on (the default).
  - `security.allowedDomains` is set to the app's own host.

**Hardening:** every `*.vikingthe.dev` host receives the session cookie.

- No subdomain may point to a third-party service outside our control.
- DNS records for retired services are deleted.

### 3. Performance budgets and rendering

Every target is tighter than Google's "good" thresholds (LCP 2.5 s, INP 200 ms, CLS 0.1 at p75).

| Metric | Field target (p75, real users) | CI gate (Lighthouse, mobile, staging) |
|---|---|---|
| LCP | ≤ 1.8 s | ≤ 2.0 s |
| INP | ≤ 100 ms | TBT ≤ 100 ms (stands in for INP, which a lab run can't measure) |
| CLS | ≤ 0.05 | ≤ 0.05 |
| TTFB, static HTML from the Cloudflare cache | ≤ 200 ms | ≤ 200 ms |
| TTFB, server-rendered route | ≤ 400 ms | ≤ 400 ms |
| JS per route (gzip) | content ≤ 15 KB, app ≤ 75 KB | Lighthouse `resource-summary:script:size` per URL, plus `size-limit` |
| Backend | p50 ≤ 10 ms, p99 ≤ 50 ms for reads, session check p99 ≤ 5 ms | `oha` against staging, run nightly |

Field data comes from the `web-vitals` library, which reports to `/api/rum`.

**Rendering**

| Route | Rendering | Caching |
|---|---|---|
| Homepage, marketing, blog, docs | Prerendered (`output: 'static'`) | Cloudflare edge |
| User-specific fragments on static pages | Server island (`server:defer`) or a client fetch to `/api/me` | Static shell stays cacheable |
| Logged-in app pages | On demand | `private, no-store` |

**Cloudflare**

Cloudflare caches neither HTML nor JSON by default, and never caches responses that carry `Set-Cookie`, `private` or `no-store`.

- **Hashed assets** (`/_astro/*`): `Cache-Control: public, max-age=31536000, immutable`.
- **Prerendered HTML:** a Cache Rule with an Edge TTL. CI purges the cache after each production deploy.
- **Bypass rules:** `/api/*` and `/_server-islands/*`.
- **R2, public objects:** served from a custom domain (for example `assets.vikingthe.dev`) and cached at the edge.
- **R2, private objects:** short-lived presigned URLs issued by Rust, never cached.

**Repo structure affects none of these runtime numbers.** It affects build time, CI time, how many services one change redeploys, and whether a contract change lands atomically. Where images are built does affect runtime (see §5).

### 4. Repository: one monorepo

```
apps/homepage/            Astro + Svelte
apps/<app>/               one per subdomain
packages/ui/              Svelte component library (workspace:*, consumed as source)
packages/api-types/       generated from OpenAPI; committed
packages/config/          shared tsconfig, size-limit and Lighthouse CI presets
crates/api/               axum binary (:8080 public, :8081 internal)
crates/openapi-gen/       writes the OpenAPI JSON
Cargo.toml                Cargo workspace
pnpm-workspace.yaml
docs/adr/
.github/workflows/
```

- **API contract:**
  - `utoipa` derives OpenAPI from the axum handlers.
  - `openapi-typescript` generates `packages/api-types`.
  - `openapi-fetch` is the client.
  - A Rust change, the regenerated types and the frontend fix land in one commit.
- **Component library:** a workspace package consumed as source.
  - No publishing, no Changesets, no version skew.
  - Each app's Vite build compiles and tree-shakes only the components it imports.
- **Independent releases:** each app is its own Coolify resource with its own deploy trigger. Releases are independent per deploy, not per repo.
- **Tooling:**
  - **Workspaces:** pnpm and Cargo.
  - **Change detection:** `pnpm --filter "...[<base>]"` for JS packages and `dorny/paths-filter` for deploy targets.
  - **Task runner:** none at first. Adopt moon, not Turborepo, if a typical PR's CI run exceeds about 5 minutes, because moon models the Rust → OpenAPI → TypeScript edge natively.
- **Images:**
  - **Rust:** `cargo-chef` plus the BuildKit cache, on a distroless `cc` base.
  - **Astro:** `pnpm deploy --filter <app> --prod`.

### 5. Build and deploy

- **Build:** GitHub Actions builds the images and pushes them to GHCR, tagged `:<sha>` plus a moving `:staging` or `:prod` tag.
  - Coolify builds Compose resources on the deployment server, and Compose is not one of the build types that can use a separate build server.
  - Building in CI keeps Rust release builds off the production machine.
- **Coolify resources:** each service is a Coolify **Docker Image** resource, which pulls the image instead of building it.
- **Triggering deploys:**
  - Auto Deploy is off.
  - Deploys are triggered only from Actions, through Coolify's authenticated Deploy Webhook, after the checks pass.
  - Watch Paths do not filter webhook deploys, so path filtering lives in Actions.
- **Staging:**
  - Lives on a separate registrable domain (for example `vikingthe-staging.dev`) with the same layout as production, also proxied through Cloudflare.
  - Staging hosts under `vikingthe.dev` would receive the production cookie.
- **Health endpoints:** every service exposes one that returns the git SHA it is running.

**Pipeline, per changed target**

1. Checks
2. Build and push the image
3. Staging webhook
4. Wait for the health endpoint to report the new SHA
5. End-to-end tests and Lighthouse CI
6. Production webhook
7. Cloudflare cache purge

**Path filters**

| Target | Paths |
|---|---|
| `api` | `crates/**`, `Cargo.toml`, `Cargo.lock` |
| `contract` | `crates/**`, `packages/api-types/**` |
| `<app>` | `apps/<app>/**`, `packages/**`, `pnpm-lock.yaml` |

A backend-only change does not redeploy the frontends. A real contract change still reaches them, because it must update the committed `packages/api-types`.

## Consequences

**Positive**

- **Session:** survives across the whole suite. A single logout ends it everywhere.
- **Speed:**
  - No CORS preflights.
  - Static pages are served from the edge.
  - No router JS for transitions.
  - Production never shares CPU with builds.
- **Atomic contract changes:** API changes land in one commit, and type drift is caught before any deploy.
- **Shared components without publishing:** the component library needs no publishing workflow.
- **Selective deploys:** each change redeploys only the services it affects.

**Negative**

- **No animated entry from the homepage into a subdomain app.** Firefox gets no animations at all.
- **Domain-wide cookie:** any compromised or dangling subdomain receives the cookie, so DNS hygiene is mandatory.
- **Valkey is now state, not cache.** It needs persistence and backups.
- **More moving parts:** the CI pipeline and GHCR registry are additions to a plain "Coolify builds from git" setup.
- **Staging costs a second domain.**

## CI guardrails

- **Contract check:**
  - Regenerate the OpenAPI spec and TypeScript types, then `git diff --exit-code packages/api-types`.
  - Then run `pnpm -r check` (`astro check` and `svelte-check`).
  - A breaking Rust API change fails CI before any image is pushed.
- **Typecheck scope:** PRs check only affected packages, using `fetch-depth: 0` and `origin/<base_ref>`. Pushes to `main` check everything.
- **Size:** `size-limit` per app, 15 KB gzip for content routes and 75 KB for app routes.
- **Lighthouse CI:** an `assertMatrix` per host against public staging routes, using the budgets in §3.
- **Playwright on staging:**
  - The session cookie has the expected attributes.
  - A transition fires within an app and does not fire across subdomains.
  - The session shows up in the app's server-rendered HTML.
  - Logout makes `/api/me` return 401 on every host.
  - CSRF requests that are `same-site` or `cross-site` get 403, and a `same-origin` control request succeeds.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Everything on one origin, path-routed | Its only gain is animation between apps. It loses origin isolation between apps and requires `base` paths everywhere. Kept as the escape hatch for a single app. |
| Separate registrable domains | Would require SSO redirects. Nothing calls for it. |
| One frontend repo + one backend repo | Contract changes would span two repos, and types would have to travel by publishing or submodule. The frontend repo would still be a monorepo of apps. |
| One repo per app + backend repo | `packages/ui` would have to be published and bumped across 5–6 repos for every change, by one person. |
| Astro `<ClientRouter />` | Ships JS on every page. Its only advantage over the native transitions is a fallback animation in Firefox. |
| `api.vikingthe.dev` | A CORS preflight on every JSON write, and only a weaker, allowlist-based CSRF check. Kept as the fallback for API routing. |
| Coolify building from Compose | Builds would run on the production server. If ever used, set Base Directory to `/`, use root-level `compose.<service>.yaml` files with `build.context: .`, and set per-resource Watch Paths. |

## Revisit if

| Trigger | Response |
|---|---|
| An app needs animated entry from the homepage | Mount it under a path |
| An app moves to another registrable domain | SSO redirects for that app |
| Coolify cannot route `/api` per host to a separate resource | `api.vikingthe.dev` with CORS and an Origin allowlist |
| A collaborator or team arrives with separate ownership | Reconsider a repo per app, publishing `ui` and `api-types` |
| Subdomains start hosting untrusted or third-party content | Host-only cookies plus SSO |
| Cloudflare stops proxying | Revisit the TTFB budgets and caching |

## Known risks (accepted without prior verification)

These were not confirmed before acceptance. Each has a stated fallback.

| Risk | Fallback |
|---|---|
| Coolify might not route `Host && /api` to a different resource than `/` on the same host. Traefik's default rule-length priority suggests it will. | `api.vikingthe.dev` |
| How a Docker Image resource reaches the API's internal `:8081` on the Coolify network is unconfirmed. | Attach the resources to a shared Docker network manually |
| Whether a Compose resource re-pulls a moving image tag on redeploy (the reason Docker Image resources are used) is unconfirmed. | Docker Image resources, which avoid the question |
| `pnpm deploy` in pnpm 10 may need `inject-workspace-packages=true`. | Set it |
| Astro's cache headers on server-island responses are unknown. | Cloudflare bypass rule for `/_server-islands/*` |
| Cloudflare's free Universal SSL covers only one subdomain level, from memory and not checked. This does not affect the chosen layout. | None needed |
| Whether `tower-sessions` hashes session IDs at rest is unknown. | Hand-rolled session store on `fred` |

## References

- CSS View Transitions Module Level 2: https://drafts.csswg.org/css-view-transitions-2/
- draft-ietf-httpbis-rfc6265bis-22 (an Internet-Draft, not yet an RFC): https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis
- MDN browser-compat-data, `@view-transition`: https://github.com/mdn/browser-compat-data/blob/main/css/at-rules/view-transition.json
- Astro view transitions: https://docs.astro.build/en/guides/view-transitions/
- Astro configuration reference: https://docs.astro.build/en/reference/configuration-reference/
- Coolify domains: https://coolify.io/docs/core/networking/domains
- Coolify build and deployment model: https://coolify.io/docs/core/build-deployment-model
- Coolify automatic deployments: https://coolify.io/docs/applications/deployments/automatic-deployments
- Coolify with GitHub Actions: https://coolify.io/docs/applications/ci-cd/github/actions
- Coolify Docker Compose: https://coolify.io/docs/knowledge-base/docker/compose
- Cloudflare default cache behavior: https://developers.cloudflare.com/cache/concepts/default-cache-behavior/
- Web Vitals thresholds: https://web.dev/articles/vitals
