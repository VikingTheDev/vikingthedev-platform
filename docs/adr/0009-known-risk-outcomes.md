# ADR 0009: Outcomes of the M1 known risks

- **Status:** Accepted (2026-10-24)
- **Date:** 2026-10-24
- **Decider:** Solo maintainer
- **Supersedes (in part):** in ADR 0001: the Known risks fallbacks for `:8081` and the session store, and where §3's backend budgets are measured (already moved off staging by ADR 0002).

## Context

ADR 0001 accepted seven risks without verifying them, each with a fallback, and `plan.md` step 14 drew two more from ADR 0003's tunnel. M1's walking skeleton tested each one in production (`plan.md`, M1a and M1b). Some held, some fell back, and two fallbacks changed on the way. M1b also turned up Coolify behavior that no ADR predicted, and a budget that can't be met from the maintainer's location.

## Decision

### ADR 0001's known risks

| Risk | Outcome | Evidence |
|---|---|---|
| Coolify routes `Host && /api` to a different resource than `/` | **Confirmed.** No `api.` subdomain | Step 10, 2026-10-23 |
| How a Docker Image resource reaches `:8081` | **Fell back, with a different fallback.** The resource UUID never resolved, and both containers already share the `coolify` network, so the stated fallback (a shared network) didn't apply. A Coolify *Network Alias*, `vikingthedev-api`, on the API resource gives a stable name; both apps use `API_INTERNAL_URL=http://vikingthedev-api:8081` | Step 11, 2026-10-23; step 18 measured a real lookup |
| A Compose resource re-pulling a moving tag | **Avoided, as planned.** Docker Image resources re-pull `:prod` on each webhook deploy | Step 9, 2026-10-23 |
| `pnpm deploy` needing `inject-workspace-packages=true` | **Not needed.** `pnpm deploy --prod` works without it: every workspace package is a `devDependency`, bundled at build time. Recheck if an app ever takes one as a runtime dependency | Step 4, 2026-10-19; every M1b image build, with the workspace packages as `devDependencies` |
| Astro's cache headers on server islands | **Answered:** `private, no-store`. The Cloudflare bypass for `/_server-islands/*` stays, so the edge never depends on Astro's header | Step 12, 2026-10-24 |
| Universal SSL covering one subdomain level | **Confirmed.** The edge certificate covers `*.vikingthe.dev` | Step 13, 2026-10-24 |
| `tower-sessions` hashing session IDs at rest | **Fell back, on a different crate.** It doesn't hash them. The store is hand-rolled on the `redis` crate instead of `fred`, which has had no release since 2025-02-27 | Step 2, 2026-10-19 |

### The tunnel's risks (ADR 0003, `plan.md` step 14)

| Risk | Outcome | Evidence |
|---|---|---|
| HTTPS redirect loops between Cloudflare, cloudflared and Traefik | **None.** `http://` gets one 301, and every HTTPS page answers 200 | Step 14, 2026-10-23 |
| The client IP behind the tunnel | **Confirmed.** The API logs the caller's IP from `CF-Connecting-IP` on `:8080` only | Step 14, 2026-10-23 |

### Backend budgets are gated inside the origin's network

Each of ADR 0001 §3's backend budgets is gated on an `oha` run from inside the VPS, never during backups:

| Budget | Gated measurement |
|---|---|
| Session check p99 ≤ 5 ms | `:8081/session` with a real session |
| Reads p50 ≤ 10 ms, p99 ≤ 50 ms | the API's read endpoints, from M2 on |

Step 18 also gated canary's server render, with a real session, at p99 ≤ 50 ms on its own port. A run against production `/api` from the maintainer's machine is a regression baseline, recorded with the network round-trip beside it, and never a gate.

From the maintainer's location, no request can meet the external budget: the fastest of 1,000 took 62 ms, against a 24 ms ping to Cloudflare and 65 ms to the Frankfurt origin. Inside the VPS, `/session` measured p99 1.72 ms and canary's server render p99 24.0 ms (step 18).

### Coolify 4.4.6 behavior, accepted or worked around

- **`--read-only` is silently dropped** from *Custom Docker Options*. Accepted: the images are distroless and run as `nonroot`.
- **`--security-opt=no-new-privileges` is rejected.** Deferred as voluntary hardening. Docker's daemon-wide `"no-new-privileges": true` in `/etc/docker/daemon.json` would apply to every container created after dockerd restarts, Coolify's own included. Restarting dockerd stops every container on the VPS unless `live-restore` is on, so it waits for a planned window.
- **The deploy webhook accepts only `POST`.**
- **A redeploying resource briefly loses its routes**, so `/api` answers 404 mid-deploy. Deploys run one at a time (phase 4b).
- **A resource's *Ports Exposes* overrides the image's port.** Canary's was `80` until 2026-10-24, when it was set to `4321`. `production-setup.md` is the source of truth for every port.

### Production `statement_timeout`

On 2026-10-24, the API resource's `DATABASE_URL` was found without the 2 s server-side `statement_timeout` that `production-setup.md` requires. `plan.md` tracks adding it; migrations already lift it on their own connection.

## Consequences

- **Positive:** every M1 risk has a recorded answer, so M2 builds on confirmed ground. The two changed fallbacks are now the documented design.
- **Positive:** the gated budgets measure the code, not the maintainer's distance from Frankfurt.
- **Negative:** a regression in the tunnel or the edge shows up only in the ungated external baseline, in Lighthouse's TTFB, and in UptimeRobot.
- **Negative:** until the daemon-wide setting is applied, no container runs with `no-new-privileges`.
- **Docs to update on acceptance:** ADR 0001's status; `plan.md` step 20; `production-setup.md`'s hardening note; `rust.md`'s `oha` line and Known risks heading; `ci-deploy.md`'s `pnpm deploy` and `:8081` notes; CLAUDE.md's Known risks invariant.

## Revisit if

- a Coolify release accepts `--read-only` or `no-new-privileges`;
- an app takes a workspace package as a runtime dependency (`pnpm deploy`);
- the origin moves, or a monitor closer to Frankfurt becomes available for the external baseline.
