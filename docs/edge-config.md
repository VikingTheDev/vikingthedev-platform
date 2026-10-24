# Edge config: Cloudflare

The Cloudflare settings for the `vikingthe.dev` zone, applied by hand in the dashboard. This file is the record: after any dashboard change, update it in the same PR (step 15 checks that the two match). `e2e/tests/edge.spec.ts` asserts its headers and cache rules after every deploy, and a failure rolls the deploy back, so update that test in the same PR too.

## Cache Rules

*Caching → Cache Rules*. Rules run top to bottom, and a later match overrides an earlier one, so the bypass rule comes last.

### 1. `prerendered-html`: eligible for cache

- **When:** `http.host eq "vikingthe.dev" and http.request.uri.path in {"/"}`
- **Then:** *Eligible for cache*.
  - *Edge TTL:* ignore the cache-control header, use 1 day. CI purges the cache after every deploy (ADR 0001), so the TTL only bounds an unpurged miss.
  - *Browser TTL:* respect origin.

This rule is an allow-list of prerendered paths, never a whole host. It ignores the origin's `Cache-Control`, so an on-demand page matched here would be served to every visitor, user data included. Cloudflare doesn't cache HTML by default, so a forgotten prerendered path just stays uncached. Add each prerendered path when it ships; with Astro's default `trailingSlash: "ignore"`, list both spellings (`/blog` and `/blog/`).

### 2. `dynamic`: bypass

- **When:**

  ```
  starts_with(http.request.uri.path, "/api/")
  or starts_with(http.request.uri.path, "/_server-islands/")
  or http.request.uri.path eq "/health"
  ```

- **Then:** *Bypass cache*.

Why each part:

- `/api/` and `/_server-islands/`: `frontend.md`. Astro sends `cache-control: private, no-store` on server islands (step 12, 2026-10-24), and this rule keeps them out of the cache regardless.
- `/health`: the deploy wait must never read a cached SHA (step 4 review).

## Response header rules

*Rules → Transform Rules → Modify Response Header*. Both rules match the whole zone: `http.host eq "vikingthe.dev" or ends_with(http.host, ".vikingthe.dev")`.

### `security-headers`

| Operation | Header | Value |
|---|---|---|
| Set | `Strict-Transport-Security` | `max-age=63072000; includeSubDomains; preload` |
| Set | `X-Content-Type-Options` | `nosniff` |
| Set | `Referrer-Policy` | `strict-origin-when-cross-origin` |
| Set | `Permissions-Policy` | `accelerometer=(), browsing-topics=(), camera=(), display-capture=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()` |

### `frame-ancestors`

- **When:** the zone expression above, `and not http.host in {"coolify.vikingthe.dev" "beszel.vikingthe.dev" "ssh.vikingthe.dev"}`.
- **Then:** **Add** `Content-Security-Policy: frame-ancestors 'none'; object-src 'none'; base-uri 'none'`.

`object-src` and `base-uri` repeat Astro's baseline in a header, which prerendered pages otherwise carry only in a `<meta>` (ASVS 3.4.3, `docs/security/asvs-l2.md` decision 7). Never add `default-src` or `script-src` here: browsers enforce every CSP they receive, and a script policy without Astro's per-page hashes would block its inline scripts.

The operation must be **Add**, never Set. On-demand pages already carry Astro's hashed `Content-Security-Policy` header, and Set would replace it. Browsers enforce every CSP they receive (`security-code.md`).

The admin hosts are a deliberate exception to `security-code.md`'s "every HTML response", for this header only, until step 15 checks whether their UIs frame themselves. Cloudflare Access guards them, and they still get every header from `security-headers`.

## API token for cache purge

*My Profile → API Tokens → Create Custom Token*: `github-actions-cache-purge`.

- **Permissions:** *Zone → Cache Purge → Purge*.
- **Zone resources:** *Include → Specific zone → vikingthe.dev*.

Copy it straight into the GitHub `production` environment as `CLOUDFLARE_API_TOKEN` ([`production-setup.md`](production-setup.md)). It's a **secret**: never paste it into a Claude session.
