# Open decisions

- **Status:** Living document. A row leaves this table when the decision lands in an accepted ADR or in a spec.
- **Owner:** whoever must act to settle it. "Maintainer" decisions are never picked silently.

## Summary

| # | Decision | State | Owner | ADR |
|---|---|---|---|---|
| D1 | Auth method | **Narrowed: email + password and/or OAuth (GitHub, Discord); magic link optional.** Details open | Maintainer | 0004 (proposed) |
| D2 | Transactional email | **Resolved: Proton SMTP** (unlimited plan). A test send is pending | Maintainer creates the SMTP token | 0007 (proposed) |
| D3 | PostgreSQL library | **Resolved: Diesel through diesel-async** (ADR 0005 accepted 2026-10-19). The M2 spike measures latency | Claude (spike in M2) | 0005 |
| D4 | API conventions | **Resolved** (ADR 0006 accepted 2026-10-19) | — | 0006 |
| D5 | Backups and restore | **Resolved: OVH daily backups, no restore drill.** Nightly `pg_dump` recommended | Maintainer | No |
| D6 | Monitoring and external uptime | **Resolved: UptimeRobot free tier** (M1 gate). Uptime Kuma noted for later | Maintainer signs up (M1 step 19) | No |
| D7 | Staging | **Resolved: none for now** (revisited 2026-10-19, on hold; findings under D7) | — | 0002 |
| D8 | GDPR basics | Recommended | Claude drafts, maintainer approves | No |
| D9 | Ingress through Cloudflare Tunnel | **Resolved: facts recorded** | — | 0003 |
| D10 | Admin hosts on the cookie domain | **Resolved and applied: Cloudflare Access** (M0) | — | 0003 |
| D11 | Authenticated E2E without staging | **Resolved: (a)** (ADR 0002 accepted 2026-10-19) | — | 0002 |
| D12 | Design token pipeline | Recommended | Maintainer approves at M4 | No |
| D13 | Swap and container memory limits | **Swap applied: 4 GiB, swappiness 10.** Limits are set per resource from M1 | The maintainer sets each limit from `capacity.md` when creating the resource | No |
| D14 | Contact form anti-spam | Recommended | Maintainer | No |
| D15 | GitHub data for the portfolio | Recommended | Maintainer | No |
| D16 | When migrations run | **Resolved: at startup under an advisory lock, expand-only** (M1 gate) | — | Recorded in 0005 |
| D17 | R2 layout for the file drop | Deferred to that app's spec | Maintainer | Maybe |
| D18 | Sign-up and access model | Open | Maintainer | 0004 (proposed) |
| D19 | Admin app (`admin.vikingthe.dev`) protection and scope | Open | Maintainer | No; covered by its spec |
| D20 | Host firewall for Docker-published ports | **Resolved: accept OVH's edge firewall as the only layer.** Improvements are listed | Maintainer | No |
| D21 | Error tracking | Recommended: Sentry (Education pack), server-side only, EU region | Maintainer, at the M2 or M3 gate | Yes, if adopted |
| D22 | Security hardening: development secrets from Proton Pass | Proposal for later | Maintainer, when development credentials are first needed | No |

## Details

### D1: Auth method

The maintainer wants user accounts with email and password, or OAuth2 through GitHub or Discord, and finds magic links a nice extra (2026-10-18). Passkeys were dropped from the recommendation.

| Sub-decision | Options | Recommendation |
|---|---|---|
| **D1a: which methods at launch** | (a) Email and password only. (b) OAuth only. (c) Both. | **(c), in two steps:** email and password first (M3 step 2), then OAuth (M3 step 3). Password alone excludes nobody. OAuth alone ties every account to a third party. |
| **D1b: which OAuth providers** | GitHub, Discord, or both | **Both**, if friends use Discord. Each provider costs one client secret in Coolify plus one callback route. Same flow otherwise. |
| **D1c: magic link** | (a) As a login option. (b) Only as the password-reset mechanism. (c) Skip it. | **(b) first.** A password reset *is* a single-use emailed link, so the code exists anyway. Exposing it as a login option later is a small change. |
| **D1d: account linking** | (a) Link an OAuth identity to an existing account automatically when the provider's verified email matches. (b) Link only from a logged-in session. | **(b).** Automatic linking lets anyone who controls a matching provider email take over the account. |
| **D1e: breached passwords** (ASVS v5.0.0-6.2.12) | (a) The HIBP range API (k-anonymity, sends 5 hash characters). (b) A bundled list of the top 100k passwords. | **(b).** No third-party call at sign-up, and nothing to add to the privacy policy. Plus a minimum of 12 characters and no composition rules (NIST SP 800-63B). |
| **D1f: Argon2id parameters** | OWASP's minimum (19 MiB, t=2, p=1) or stronger | **OWASP's minimum,** measured in M3. It must stay under about 100 ms on this VPS, in `spawn_blocking`, behind rate limits. |
| **D1g: second factor for the admin role** | (a) TOTP required for admins from M3. (b) Cloudflare Access in front of `admin.`, **and** Rust accepts `/api/admin/*` only with `Host: admin.vikingthe.dev` and a valid `Cf-Access-Jwt-Assertion` (signature checked against the team's certificates, plus `aud`). (c) None. | **(b) now and (a) later.** Access on the admin pages alone is *not* a second factor: `/api` is routed on every host, so a stolen admin password could call `vikingthe.dev/api/admin/*` without ever passing Access. The Rust-side host and JWT check closes that. ASVS 5.0.0 L2 goes further and requires MFA for every user (v5.0.0-6.3.3), an open decision for M3 (`docs/security/asvs-l2.md`). |

**Evidence that settles it:** the maintainer's answers to D1a–D1g. For D1f, a timing measurement on the VPS. **ADR:** yes, 0004.

### D2: Transactional email and DNS hygiene

| | |
|---|---|
| **Options** | (a) Proton SMTP submission with a per-address SMTP token. (b) A free tier of a transactional provider (Resend, Brevo, Postmark's trial). (c) Self-hosted SMTP on the VPS. |
| **Recommendation** | **(a) Proton SMTP**, sending from a dedicated address such as `noreply@vikingthe.dev`. It needs **no new DNS records**: SPF, DKIM (three `_domainkey` CNAMEs) and DMARC are already in place. So it satisfies the DNS hygiene rule trivially: no host that serves HTTP points at a third party, and there are no click-tracking or bounce subdomains. (b) would add DKIM and return-path records, and sometimes a tracking CNAME that would receive the session cookie. (c) has poor deliverability from an OVH IP. |
| **Resolved** | The maintainer's Proton Unlimited plan includes SMTP submission (2026-10-18). |
| **Still unverified** | Proton's sending limits and terms for automated mail, and whether OVH allows outbound port 587. |
| **Evidence that settles it** | A test email from the VPS passes SPF, DKIM and DMARC (M3 step 1). |
| **Secret handling** | The SMTP token lives only in Coolify environment variables. The human creates and enters it. |
| **DMARC** | Currently `p=quarantine` with no `rua`. Optionally add `rua=` pointing to a Proton address for reports. That's a TXT record, so no hygiene impact. |
| **ADR** | Yes, 0007 (a new category of external dependency). |

### D3: PostgreSQL library

| | |
|---|---|
| **Resolved** | Diesel (the maintainer's choice, 2026-10-18). |
| **Resolved (M1 gate, 2026-10-19)** | `diesel-async`, with ADR 0005 accepted. M1 checks the binary has no libpq; the M2 spike measures latency. |
| **Was open** | `diesel-async` (on `tokio-postgres`, pure Rust) with a `bb8` or `deadpool` pool, vs synchronous `diesel` (libpq) through `deadpool-diesel`, which runs each query in `spawn_blocking`. |
| **Recommendation** | **`diesel-async`.** The distroless `cc` runtime image has no `libpq.so`, so synchronous Diesel would need a custom base or static linking. Async also fits axum without hopping a thread per query. Migrations use `diesel_migrations` through `AsyncConnectionWrapper`, which also avoids libpq. |
| **Evidence that settles it** | A 1-day timeboxed spike in M2. The same three queries (a hot read by primary key, a filtered list with a cursor, and a transaction across two tables) checked for: p99 under `oha`, clean compile time, the `ldd` output of the binary, and how errors map to `AppError`. |
| **ADR** | Yes, 0005. |

### D4: API conventions

Recommendation, detailed in ADR 0006 and applied in `api-contract.md`:

| Topic | Recommendation |
|---|---|
| Errors | RFC 9457 `application/problem+json`, with `type` a stable URI (`https://vikingthe.dev/problems/<code>`) that clients switch on. No internal text. |
| IDs | UUIDv7, serialized as strings, wrapped in Rust newtypes. |
| Timestamps | RFC 3339 in UTC with `Z`, in fields ending in `_at`. |
| Pagination | Cursor-based: `?limit=` (default 20, max 100) and `?cursor=`, returning `{ items, next_cursor }`. |
| Idempotency | An `Idempotency-Key` header (per the IETF httpapi draft) on POSTs that create resources or send email. Stored in Valkey for 24 h, keyed by caller plus key, with a request-body hash. A reused key with a different body gets 422. |
| Versioning | No `/v1` prefix: every consumer lives in this repo. |
| Breaking changes | `oasdiff breaking` compares the PR's `openapi.json` with `main`'s, and fails CI on a breaking change. The fix is to expand then contract: add the new field, deploy, then remove the old one in a later PR, because the API and the apps deploy separately. |
| Tooling | `oasdiff` runs as a pinned release binary with a checksum check, not as a third-party GitHub Action (CLAUDE.md "stop and ask"). |

**Resolved (M1 gate, 2026-10-19):** approved with ADR 0006 accepted. **ADR:** yes, 0006 (a public API shape).

### D5: Backups and restore

| | |
|---|---|
| **Resolved** | OVH's daily automated VPS backup is the backup, and there is no restore drill (the maintainer, 2026-10-18). OVH can only restore over the live disk. |
| **Recommendation (free additions)** | (1) A nightly `pg_dump -Fc` to a local volume, so the OVH image always holds a consistent logical dump, not only crash-consistent data files. (2) Valkey RDB snapshots alongside AOF. |
| **Gaps to accept knowingly** | Restoring rolls back the whole VPS, including Coolify. The backup is on the same provider as the VPS. Retention follows OVH's policy. Files in R2 (D17) aren't covered by OVH backups. The backups have never been tested. |
| **ADR** | No. It's cheap to change. |

### D6: Monitoring and external uptime

| | |
|---|---|
| **In place** | Beszel hub and agent, for host and container metrics. |
| **Options for the external check** | (a) UptimeRobot free tier (5-minute checks, email alerts). (b) A scheduled GitHub Actions workflow running `curl` (runs at most every 5 minutes, and is often delayed). (c) Cloudflare health checks (paid). |
| **Recommendation** | **(a)** for `/`, `/api/health` and `canary.vikingthe.dev`, alerting a Proton address. No hosted status page on a `vikingthe.dev` subdomain (DNS hygiene). The homepage status section reads our own `/api/status`. The page itself is served from the edge cache, so it can still say "unreachable" when the VPS is down. |
| **Also** | Pin Beszel's images to a version instead of `latest`. |
| **Resolved** | **(a)**, at the M1 gate (2026-10-19). The maintainer signs up. |
| **Later option** | Self-hosted Uptime Kuma on a separate VPS (about €3.81 a month). If it's built: a different OVH datacenter from production, so one outage doesn't take out both; a free external check on Kuma itself, so a dead monitor is noticed; its UI behind its own tunnel and Access, and never on `*.vikingthe.dev`, which receives the session cookie; set up like M0. It could share the VPS of a future staging environment (D7), with a memory limit. |
| **Evidence** | M1 step 19 (a forced outage triggers an alert). |
| **ADR** | No. |

### D7: Staging

**Resolved:** none for now (the maintainer, 2026-10-18). The build stays staging-ready: cookie domain, hosts and the `Origin` allowlist come from config. See ADR 0002.

**Revisited 2026-10-19 and put on hold.** The maintainer leaned toward staging on a separate VPS, then deferred it. When staging comes back, start from these findings:

| Topic | Finding |
|---|---|
| Money | About $12 a year for a second `.dev` domain (registrar list price, estimate), plus an OVH VPS 1 (2 vCores, 4 GB RAM, 50 GB disk) at €3.81 a month, as quoted by the maintainer on 2026-10-19. Cloudflare (zone, tunnel, Universal SSL, Access for up to 50 users), an R2 bucket within the free 10 GB, OAuth apps, Mailpit and Actions minutes cost nothing |
| Domain | `vikingthe-staging.dev`: the same TLD, so `.dev`'s HSTS preload gives staging the same HTTPS, `Secure` and `__Secure-` cookie behavior as production; clearly not production; a separate registrable domain, so the production cookie never reaches it. It was unregistered on 2026-10-19 (RDAP 404), as were `vikingthe-stage.dev` and `vtstaging.dev` |
| Admin hosts | Never put them on the staging domain: staging runs less-proven code on the same site, so an XSS bug there could send same-site requests to production's Coolify. A third domain (for example `vikingthe-ops.dev`, also unregistered on 2026-10-19) is the clean option for ADR 0003 point 4 |
| Why a separate VPS | Staging on the production VPS steals CPU from production during `oha` and test runs, skews its own numbers with that contention, and shares a host with production if a container is compromised |
| VPS sizing | 4 GB fits staging with smaller limits than `capacity.md`'s production values (for example PostgreSQL about 768 MiB, the API 256 MiB), but not its own Coolify, whose documented minimum is 2 GB (measured use here is about 0.6 GiB). Add it to the existing Coolify as a remote server over SSH, which gives production's Coolify root SSH on the staging host. Open: how that SSH gets through staging's OVH firewall, either an allowlist for the production VPS's IP or SSH through a Cloudflare tunnel with Access and a service token. The future staging ADR settles it |
| Real cost | Upkeep, not money: every Coolify resource, environment variable and OAuth app exists twice, and the Cloudflare cache, header and Access rules must be copied to the second zone by hand, where they drift. Each deploy to `main` takes roughly 3–6 minutes longer (estimate) |
| Authenticated E2E | Staging alone doesn't solve logging in. Either the staging image carries the `e2e` test login, so it isn't the exact production binary, or a staging test account's credential becomes a secret in a `staging` GitHub environment. If Access protects staging, Playwright also needs an Access service token. Both secrets are outside CLAUDE.md's allowed Actions secrets, so that path needs a CLAUDE.md security change the maintainer approves. The `e2e` path conflicts with ADR 0002 point 3, which never compiles `e2e` into a pushed image: a staging image built with it is a different GHCR artifact from `:prod` |
| What it adds | Coolify, Traefik-through-tunnel and Cloudflare regressions, plus real TTFB through the edge, are caught before production instead of by the post-deploy smoke checks and a rollback. It also restores what ADR 0002 drops from ADR 0001: the Lighthouse CI gate on staging and the nightly `oha` run against staging |
| How to add it | On top of the CI stack in ADR 0002, not instead of it: the CI stack still tests every PR without secrets. Staging is then a config change (ADR 0002 point 4), a second VPS set up like M0 (tunnel, OVH firewall, unprivileged `cloudflared`, swap, Access if private), and a new ADR superseding the staging parts of ADR 0002 |

### D8: GDPR basics (Norway, EEA)

| Item | Recommendation |
|---|---|
| Privacy policy | One page on the homepage. Covers the controller's identity and contact, what's collected (account email, password hash, linked OAuth identities, session metadata, contact-form messages, RUM without identifiers), purposes, legal bases, retention, processors and rights. |
| Processors | OVH (EU), Cloudflare (CDN, DNS, R2, Access; EU-US transfers under its DPA), Proton (email, Switzerland). R2 buckets use the EU jurisdiction. GitHub and Discord are independent controllers for OAuth sign-in; the policy names them and lists the profile fields we receive. |
| Cookie consent | Not needed for the session cookie, which is strictly necessary under the ePrivacy exemption, as implemented in Norway's ekomloven. No other cookies, and RUM stores no identifiers. |
| Data export | `GET /api/me/export` returns JSON with every personal field. |
| Account deletion | `DELETE /api/me` removes the user's rows and Valkey sessions immediately. Deleted data stays in backups until they rotate out, and the policy says so. |
| Retention | Contact messages 12 months. RUM aggregated, with raw rows deleted after 30 days. Logs keep no IP beyond 14 days. |

**Owner:** Claude drafts in M3, and the maintainer approves the wording and supplies the controller details. **Evidence that settles it:** maintainer approval. **ADR:** no.

### D9: Ingress through Cloudflare Tunnel

**Resolved facts (the maintainer, 2026-10-18):**

- `cloudflared` ran on the VPS as `root`. Since M0 it runs as an unprivileged `cloudflared` user.
- There are four tunnels: `ssh.`, `beszel.`, `coolify.`, and a catch-all that sends the apex and every other `*.vikingthe.dev` to Coolify's Traefik.
- Everything else is blocked by OVH's edge firewall. It's the only layer in front of Docker-published ports and sshd, because those bypass `ufw` (D20). All traffic arrives through the tunnels.

**Consequence:** the API can trust `CF-Connecting-IP`. The scheme Coolify domains need (`http://` vs `https://`) is still settled in M1 step 14. Merging the tunnels is optional and not recommended: separate tunnels mean separate credentials.

**Done in M0:** `cloudflared` moved from `root` to an unprivileged `cloudflared` system user, under hardened systemd units. Cloudflare's packaged service runs as root, so it wasn't used. All tunnels force `http2`, because QUIC over UDP fails through OVH's firewall. **ADR:** yes, 0003.

### D10: Admin hosts on the cookie domain

`coolify.`, `beszel.` and `ssh.vikingthe.dev` receive `__Secure-vt_sid`. The cookie is `HttpOnly` and our CSRF check rejects `same-site`, so the remaining risks are a compromised or logging admin app capturing session cookies, and cookie tossing: a compromised subdomain setting its own `Domain=vikingthe.dev` `__Secure-vt_sid` to plant a session. A new token on every login (ADR 0004) limits the tossing risk.

| Option | Notes |
|---|---|
| (a) Cloudflare Access (free for up to 50 users) in front of all three | Strangers can't reach them at all. The cookie still flows to them. |
| (b) Move them to a separate, cheap registrable domain | The cookie never reaches them. Costs a domain, which is never the staging domain (see D7). |
| (c) Accept as is | Coolify's login page is exposed to the internet. |

**Resolved:** (a) now (the maintainer, 2026-10-18), and (b) when a dedicated admin domain is bought (never the staging domain; see D7). **Applied in M0 (2026-10-18):** login by emailed one-time PIN or Cloudflare. There's a Bypass rule only for `coolify.vikingthe.dev/api/v1/deploy`, which Coolify's deploy-only token still protects. **ADR:** recorded in 0003.

### D11: Authenticated E2E without staging

| Option | Notes |
|---|---|
| (a) A full stack in CI (compose with Traefik, a throwaway TLS cert, `*.vikingthe.test` → 127.0.0.1) and an `e2e`-only test login | No secrets. Tests the code, not Coolify or Cloudflare. |
| (b) A test account on production with its credential in GitHub Actions | Adds a secret that CLAUDE.md doesn't allow. |
| (c) Run authenticated tests against production only, by hand | Slow and easy to skip. |

**Recommendation:** (a) for every PR, plus production smoke checks that need no login after each deploy, plus the human-run session check in M1 step 17. Anything Coolify- or Cloudflare-specific is verified once in M1 and then by the smoke suite. **ADR:** recorded in 0002.

### D12: Design token pipeline

`packages/ui` has no build step. DTCG JSON has to become CSS custom properties.

**Recommendation:** DTCG JSON is the source. A short script in `packages/ui` writes `tokens.css`, which is committed, and CI checks `git diff --exit-code`, the same pattern as `api-types`. Style Dictionary would add a dependency for something about 60 lines of TypeScript can do. Decide at M4.

### D13: Swap and container memory limits

**Applied in M0 (2026-10-18):** a 4 GiB swapfile with `vm.swappiness=10`. The VPS had no swap before. Without container limits, one runaway process gets OOM-killed, or kills something else, so the maintainer gives every Coolify resource its limit from `capacity.md` when creating it. Claude has no Coolify access.

### D14: Contact form anti-spam

**Recommendation:** a honeypot field, a minimum time to fill the form, rate limits per IP (5 a minute, 20 a day), and an `Idempotency-Key`. No Cloudflare Turnstile at first: it's a third-party script that costs JS budget and needs CSP changes. Revisit if spam gets through.

### D15: GitHub data for the portfolio

**Recommendation:** fetch at build time from GitHub's public, unauthenticated API (60 requests per hour is plenty for one build), and render into the prerendered page. A daily scheduled workflow rebuilds and redeploys the homepage. No runtime calls and no token. Highlighted repos are a list in a content collection.

### D16: When migrations run

**Recommendation:** at API startup, through `diesel_migrations` with an advisory lock, and only expand-style migrations (add a column, add a table). Destructive migrations wait for a later release, following D4's expand-then-contract rule.

**Resolved (M1 gate, 2026-10-19):** as recommended, recorded in ADR 0005. It comes into M1 with PostgreSQL.

### D17: R2 layout for the file drop

Deferred to that app's spec. The questions: one EU-jurisdiction bucket per environment, per-user quotas within the 10 GB free tier, maximum file size, link expiry and virus scanning (probably none, since only users with a grant can upload).

### D18: Sign-up and access model

Accounts are wanted, but gated apps such as Stash must not be open to the whole world.

| Option | Notes |
|---|---|
| (a) Open sign-up; gated apps need a per-app grant from the admin | Anyone can make an account and use public features, such as game streak sync. Storage-costing apps stay closed. Spam accounts are possible, but email verification and rate limits contain them. |
| (b) Invite-only sign-up; every account is trusted | Simplest authorization. Public games can't offer accounts to strangers. |
| (c) Open sign-up, with grants requested in-app and approved in admin | Like (a), plus a request flow. More to build. |

**Recommendation:** **(a).** The model is a `role` enum (`user` or `admin`) plus an `app_grants(user_id, app, granted_by, granted_at)` table, checked in the domain layer and denied by default. A request flow (c) can come later. **ADR:** recorded in 0004.

### D19: Admin app protection and scope

`admin.vikingthe.dev` is a new app: it manages users, access grants and other platform features. It's our own code, so it doesn't break DNS hygiene. But it's the most privileged surface on the platform.

| Question | Options | Recommendation |
|---|---|---|
| **Extra protection** | (a) Cloudflare Access in front of `admin.`, plus the role check, plus the Rust-side host and Access JWT check from D1g(b). (b) Admin role only. | **(a).** Without the Rust-side check, Access would protect only the admin app's pages, because `/api` is routed on every host. The CI stack has no Access, so its `e2e` build signs test JWTs with a throwaway key. Accepting that key is compiled in only under the `e2e` feature, never as a runtime option; production always verifies against Cloudflare's team certificates. |
| **Re-authentication** | Require a login within the last 15 minutes for every admin write (any non-GET request) | **Yes.** Needs a `last_auth_at` field in the session record, which changes ADR 0001's record shape (`plan.md` §2). A stale login gets 401 `reauthentication_required`. |
| **Audit** | An append-only `admin_audit` table recording who did what, when, and to whom | **Yes.** Cheap, and needed to investigate abuse. |
| **Scope v1** | List and search users; grant and revoke app access; disable an account (revoking its sessions); the audit log. Invites only if D18 lands on invite-only sign-up | Confirm. Anything more goes in the spec. |
| **When** | M3, since grants are needed before Stash | Confirm. |

**ADR:** no. The protections land in the admin spec (`docs/specs/admin.md`), written before its code.

### D20: Host firewall for Docker-published ports

**Resolved (the maintainer, 2026-10-18):** accept OVH's edge firewall as the only layer in front of Docker-published ports. External probes on IPv4 found 22, 80, 443, 6001, 6002, 8000, 8080 and 52843 (sshd) closed or filtered. The VPS has no public IPv6, only a private `fda6:…` address.

**Why it's a single layer:** Docker publishes 80, 443, 8080 (Traefik; removed 2026-10-23), 8000 (Coolify), 6001 and 6002 on every interface, and those ports bypass `ufw`. sshd (socket-activated on port 52843) also listens on every interface. If the edge firewall were ever turned off:

- 80 and 443 would let clients bypass Cloudflare and forge `CF-Connecting-IP`, which defeats the per-IP rate limits (ADR 0003 point 3);
- Coolify on `:8000` would be reachable without going through Cloudflare Access;
- Traefik's `:8080` dashboard and sshd would be directly reachable.

**Future improvements (not scheduled):**

- A systemd unit that adds a `DOCKER-USER` rule dropping new inbound connections from the public interface.
- ~~Stop publishing Traefik's `:8080` dashboard, or bind it to `127.0.0.1`.~~ Done 2026-10-23: the maintainer removed the `8080:8080` mapping.
- Bind sshd's socket to `127.0.0.1:52843`, since only the tunnel uses it. OVH's KVM console stays as the fallback.
- Bind Coolify's `8000`, `6001` and `6002` to `127.0.0.1`, if Coolify supports a lasting override.
- Remove passwordless sudo from `ubuntu`.

### D21: Error tracking

| | |
|---|---|
| **Option** | Sentry through the GitHub Student Developer Pack: 50K errors and 100K transactions, for 1 year, renewable while the maintainer is a student. It's the only pack offer the maintainer wants to use (2026-10-19). |
| **Recommendation** | Server-side only: the `sentry` crate in the API, and Astro's server runtime. No browser SDK: it would add tens of KB to every page and widen the CSP. Use Sentry's EU data region (GDPR), scrub request bodies, cookies and headers, and send no user identifiers beyond the UUID. The DSN is a production secret in Coolify. |
| **Cost of leaving** | Low: removing it is a dependency and an environment variable. The pack expiring doesn't break anything. |
| **Evidence that settles it** | The maintainer decides at the M2 or M3 gate. A forced error appears in Sentry with no cookie, token or email in the event. |
| **ADR** | Yes, if adopted (a new category of external dependency). |

### D22: Security hardening: development secrets from Proton Pass

**Proposal for later** (the maintainer, 2026-10-19). Like D20's improvements, it's not scheduled.

| | |
|---|---|
| **Today** | Real development credentials live in plain text in `~/.config/vikingthedev/secrets.env` (mode `600`), loaded only by the human with `scripts/with-secrets <cmd>`. |
| **Proposal** | `scripts/with-secrets` would wrap Proton Pass's official CLI: `pass-cli run --env-file <template> -- <cmd>`. The template would hold only `pass://<vault>/<item>/<field>` references, never values, and the plain-text file would go away. Whether `pass-cli run` masks values in output is unverified, and doesn't change the rule that its output never reaches Claude. Production secrets stay in Coolify. |
| **Availability** | Sources disagree on which plans include the CLI. Proton's announcement lists Pass Plus, Family, Professional and every bundle, which covers the maintainer's Unlimited plan. Check Proton's current docs for the exact flags. |
| **Risk to close first** | While `pass-cli` is logged in, any process running as the maintainer can call it, including Claude's shell commands. The boundary is the sandbox; a Bash deny rule for `pass-cli` is only a speed bump (`docs/claude-code-setup.md`). Before adopting it, the human confirms that: `pass-cli`'s session and config storage is under a sandbox read-denied path; Proton's API hosts aren't in the sandbox's allowed network domains; and, if it keeps its session key in the OS keyring, the D-Bus socket (`/run/user/<uid>/bus`) is unreachable from the sandbox. The human also adds a Read-tool deny rule for `pass-cli`'s state directory, because Claude's file tools run outside the sandbox. |
| **Rules it touches** | Secret handling and `.claude/settings.json`, both stop-and-ask in CLAUDE.md. CLAUDE.md's "Real development credentials" section and `docs/claude-code-setup.md` change in the same commit. |
| **Evidence that settles it** | The maintainer decides. Then: `scripts/with-secrets` works without the plain-text file, and the human checks, before any real credential is in the vault, that a `pass-cli` command that can't print secret values (a login-status check) fails from Claude's sandbox. |
| **ADR** | No. It's cheap to reverse. |
