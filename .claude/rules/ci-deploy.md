---
paths:
  - ".github/**"
  - "**/Dockerfile*"
  - "**/*.Dockerfile"
  - "**/compose*.yaml"
  - "**/compose*.yml"
  - "**/.dockerignore"
  - "deny.toml"
  - "pnpm-workspace.yaml"
  - "**/.npmrc"
---

# CI, containers and deploy rules

## Workflows

- **Permissions:** start from `permissions: {}` and grant each job only what it needs.
- **Pinning:** pin third-party actions to a full commit SHA.
- **Approved third-party actions** (M1 gate, 2026-10-19): `pnpm/action-setup`, `Swatinem/rust-cache`, `docker/setup-buildx-action`, `docker/login-action`, `docker/build-push-action`, `dorny/paths-filter`, `treosh/lighthouse-ci-action` (reports to artifacts only, never public storage), `gitleaks/gitleaks-action`, `EmbarkStudios/cargo-deny-action`. Each is justified in the commit that adds it. Any other is stop-and-ask. `oasdiff` runs as a pinned binary with a checksum check.
- **Triggers:** never use `pull_request_target`.
- **Event text:** never interpolate `${{ github.event.* }}` into `run:` scripts. Pass it through `env:`.
- **Secrets:** only jobs that deploy, on pushes to `main`, use the `production` environment (and `staging`, if one is ever added).
- **Scans on every push:**
  - `cargo deny check` (advisories, licenses, sources);
  - `pnpm audit --prod`;
  - `gitleaks` over the full history.
- **Updates:** Dependabot opens update PRs.

## Containers

- Use distroless images that run as a non-root user.
- Make the root filesystem read-only where possible.
- Runtime images have no package manager and no shell.
- No secrets in `ARG`s, `ENV` or any layer.
- **Rust images:** `cargo-chef` with the BuildKit cache, on a distroless `cc` base.
- **Astro images:** one shared `docker/astro.Dockerfile` with `--build-arg APP=<app>`, built with `pnpm deploy --filter <app> --prod`. On pnpm 12.14 this works without `inject-workspace-packages=true` (step 4, 2026-10-19), but no app depended on a workspace package yet, so recheck when one first does.
- **Base images** are pinned by tag and digest.

## Build and deploy

- **Images:** Actions builds them and pushes to GHCR, tagged `:<sha>` plus a moving `:prod` tag. The CI-stack image (with the `e2e` feature) is built with `--load` only and never pushed.
- **Coolify:**
  - Each service is a Docker Image resource.
  - Auto Deploy is off.
  - Deploys are triggered only by the authenticated Deploy Webhook, called from Actions.
- **Health:** every service has a health endpoint that returns its git SHA.
- **Pipeline,** for each changed target (ADR 0002):
  1. checks, including Playwright, axe and Lighthouse CI on the CI stack;
  2. record the digest `:prod` points at now, then build and push `:<sha>` and `:prod`;
  3. call the production webhook;
  4. wait for the health endpoint to report the new SHA;
  5. purge the Cloudflare cache;
  6. smoke checks: the `@smoke` Playwright tests with no login, including the edge headers, the cache rules and Observatory's A+.
    
  If the redeploy (step 3), the health wait (step 4) or the smoke checks (step 6) fail, roll back; a cancelled or timed-out smoke step counts as failed. To roll back, retag `:prod` to the digest recorded in step 2 with `docker buildx imagetools create`, call the webhook again, and purge the cache. A failed purge alone doesn't roll back. Once every deploy job succeeds, Lighthouse CI runs against production in a job of its own; a miss fails the run but never rolls back (ADR 0008). Deploy jobs run one at a time and share one `concurrency` group per target with `cancel-in-progress: false`, so two merges never race on `:prod`.

- **Path filters** (`dorny/paths-filter`):

  | Target | Paths |
  |---|---|
  | `api` | `crates/**`, `Cargo.toml`, `Cargo.lock` |
  | `contract` | `crates/**`, `packages/api-types/**` |
  | `<app>` | `apps/<app>/**`, `packages/**`, `pnpm-lock.yaml` |

  Never add `crates/**` to an app filter. A real contract change reaches the apps through `packages/api-types`.
- **Typecheck scope:** pull requests check only the affected packages, using `fetch-depth: 0` and `origin/<base_ref>`. Pushes to `main` check everything.
- **Staging:** none for now (ADR 0002). If one is added, it lives on its own registrable domain, laid out like production.
- **Coolify risks resolved** (ADR 0001; outcomes in `docs/plan.md`, ADR 0009 pending):
  - Routing `/api` and `/` on one host to different resources works (step 10, 2026-10-23), so there's no `api.` subdomain or CORS.
  - A Docker Image resource reaches the API's internal `:8081` through a Coolify *Network Alias* (`vikingthedev-api`), not its UUID, which never resolves (step 11, 2026-10-23; the 50 ms budget waits for step 18).

## Repo settings (applied by the human)

- secret scanning with push protection;
- Dependabot alerts;
- private vulnerability reporting, plus a `SECURITY.md`;
- a ruleset on `main` that requires a PR and the CI checks, with no approvals and no bypass, and blocks force-pushes and deletion. Required checks are jobs that always run (the `dorny/paths-filter` gate job), never path-skipped ones. Only the maintainer pushes.
