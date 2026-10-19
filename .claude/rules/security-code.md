# Security rules for code

These rules are always loaded, because they also apply to specs, proxy config and Cloudflare settings. They override everything except the security section of CLAUDE.md.

## Input and access

- **Validate at the boundary.** Parse every external input into a validated type at the edge. This covers bodies, queries, headers, cookies, files and environment. Interior code never sees raw input.
- **No string-built commands.** Use parameterized SQL only. Never build SQL, shell commands, URLs or file paths by concatenating input.
- **Client IP** (ADR 0003): read it only from the configured trusted header (`CF-Connecting-IP` in production), never from `X-Forwarded-For` or the peer address, and only on the public `:8080` listener, parsed at the boundary into a `ClientIp` type. A rate-limited request without the header is rejected. IPv6 rate limits key on the /64 prefix. This is safe only while the origin's inbound ports stay closed (D20).
- **Check authorization** in the domain layer on every resource access, not just "is logged in". This prevents IDOR. Deny by default.

## Authentication

- **Passwords:** hash with Argon2id (the `argon2` crate), inside `spawn_blocking`.
- **Tokens:** generate them with an OS CSPRNG (`rand::rngs::OsRng` or `getrandom`). Never roll your own crypto.
- **Sessions:** issue a new session token on login and on any privilege change. On logout, delete it server-side.
- **Rate limits:** login, signup, password reset and anything that sends email are rate-limited per IP and per account, with counters in Valkey.
- **Errors:** never reveal whether an account exists ("invalid email or password").

## Output

- **Raw HTML:** `{@html}` and `set:html` are forbidden unless the input is sanitized server-side. Flag every use in review.
- **Server islands** take no props and render no `<slot />` (a `slot="fallback"` is fine: it stays in the host page). They read what they need, such as the session, from the request itself. Astro encrypts props and slots with a key baked into the image, so treat that key as public: anyone can forge them, and slots render as raw HTML (step 5 review, 2026-10-19).
- **Security headers** on every HTML response:
  - **`Content-Security-Policy`:**
    - no `unsafe-inline` or `unsafe-eval` for scripts;
    - inline scripts, including Astro's island hydration scripts, allowed **by hash, not nonce**, because edge-cached HTML can't carry a per-request nonce;
    - hashes come from Astro's `security.csp`, with the shared directives from `@vikingthedev/config/csp`. Prerendered pages get a `<meta>`; on-demand pages get a `Content-Security-Policy` response header, hashed per response (step 5, 2026-10-19);
    - `frame-ancestors 'none'` is sent as an HTTP header from Traefik or Cloudflare, because browsers ignore it in a `<meta>` CSP. That rule must **add** a second CSP header, never set or replace one: replacing would drop the hashes Astro sends on on-demand pages. Browsers enforce every CSP they receive.
  - `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload`
  - `X-Content-Type-Options: nosniff`
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - a restrictive `Permissions-Policy`
- **Errors:** clients get a stable error code, never stack traces, SQL or internal messages.
- **Logs:** never log tokens, passwords, cookies, `Authorization` headers or secret values. Keep PII to the minimum.

## Storage

- R2 buckets are private by default.
- The server checks the size and content type of every upload.
- Downloads go through short-lived presigned URLs.
