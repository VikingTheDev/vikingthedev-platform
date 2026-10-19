# ADR 0002: No staging environment for now; a full stack in CI instead

- **Status:** Proposed
- **Date:** 2026-10-18
- **Decider:** Solo maintainer
- **Would supersede:** in ADR 0001: §3's staging CI gate and "`oha` against staging"; §5 "Staging" and pipeline steps 3–5; and the staging parts of "CI guardrails"

## Context

ADR 0001 deploys to a staging environment on a separate registrable domain and runs end-to-end tests and Lighthouse there before production. The maintainer decided that nothing is high-stakes enough to need staging yet. They also want to keep everything free, and accept a single point of failure.

There are two gaps without staging. Authenticated end-to-end tests would have to run against production, which needs a test credential in GitHub Actions; CLAUDE.md allows no such secret. And nothing tests a change before production.

## Decision

1. **No staging environment.** The pipeline per changed target becomes:
   1. checks, including the CI stack below;
   2. build and push `:<sha>` and `:prod`;
   3. the production webhook;
   4. wait for the health endpoint to report the new SHA;
   5. the Cloudflare cache purge, so the checks below see the new HTML;
   6. post-deploy smoke checks: Playwright with no login, Lighthouse CI, Observatory.

   If step 4 or 6 fails, CI rolls back. The Docker Image resource always pulls `:prod`, and the webhook can't choose a tag. So CI retags `:prod` to the previous `:<sha>` in GHCR (`docker buildx imagetools create`), calls the webhook again and purges the cache. Rollback is safe only because migrations are expand-only (D16).
2. **A full stack in CI.** A compose file brings up Traefik, the API, every changed app, PostgreSQL, Valkey and Mailpit inside the GitHub runner.
   - `*.vikingthe.test` resolves to `127.0.0.1`, with a TLS certificate generated during the job.
   - The cookie domain is `vikingthe.test`.
   - Every Playwright, axe and Lighthouse lab check from ADR 0001 runs here on every PR.
3. **A test login only in the CI stack.** It's behind a cargo feature `e2e`, and is never compiled into the production image. An HTTP test and a CI check enforce that. The CI-stack image is built with `--load` only: it's never pushed to GHCR, and never tagged `:<sha>` or `:prod`.
4. **Configurable environment values.** The cookie `Domain`, the allowed hosts and the CSRF `Origin` allowlist are runtime config. Production's values stay exactly as in ADR 0001. A future staging environment is then only a config change, not a code change.
5. **Coolify- and Cloudflare-specific behavior** is verified once in milestone 1, then continuously by the production smoke checks.

## Consequences

- **Positive:** no second domain or extra resources; no test credentials anywhere; every PR runs the full functional suite.
- **Negative:**
  - a bad deploy is caught only after it reaches production, with rollback as the mitigation;
  - Coolify, Traefik-through-tunnel or Cloudflare regressions surface only in production;
  - Lighthouse's TTFB through the edge can only be measured on production.
- **Docs to update on acceptance:** ADR 0001's status; CLAUDE.md's checks table ("Playwright on staging"); the pipeline in `ci-deploy.md`; "Measure with `oha` against staging" in `rust.md`; and the "CI gate (Lighthouse mobile, staging)" column in `frontend.md`.

## Revisit if

- An app holds data whose loss or exposure would hurt other people, beyond the maintainer's friends.
- A production incident is traced to something only staging would have caught.
- The maintainer decides to fund a second VPS and domain for staging.

Whatever domain staging uses is never shared with the admin hosts in ADR 0003.

The findings from revisiting staging on 2026-10-19 (costs, domain, a separate VPS, sizing, authenticated E2E) are in `decisions.md` under D7.
