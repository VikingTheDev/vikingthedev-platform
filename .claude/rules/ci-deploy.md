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
- **Astro images:** `pnpm deploy --filter <app> --prod`. In pnpm 10 this may need `inject-workspace-packages=true`. That's unverified, so set it if the build fails.

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
  6. smoke checks: Playwright with no login, Lighthouse CI, Observatory.

  If step 4 or 6 fails, roll back: retag `:prod` to the digest recorded in step 2 with `docker buildx imagetools create`, call the webhook again, and purge the cache. Deploy jobs share one `concurrency` group per target with `cancel-in-progress: false`, so two merges never race on `:prod`.
- **Path filters** (`dorny/paths-filter`):

  | Target | Paths |
  |---|---|
  | `api` | `crates/**`, `Cargo.toml`, `Cargo.lock` |
  | `contract` | `crates/**`, `packages/api-types/**` |
  | `<app>` | `apps/<app>/**`, `packages/**`, `pnpm-lock.yaml` |

  Never add `crates/**` to an app filter. A real contract change reaches the apps through `packages/api-types`.
- **Typecheck scope:** pull requests check only the affected packages, using `fetch-depth: 0` and `origin/<base_ref>`. Pushes to `main` check everything.
- **Staging:** none for now (ADR 0002). If one is added, it lives on its own registrable domain, laid out like production.
- **Known Coolify risks** (ADR 0001):
  - Routing `/api` and `/` on one host to different resources is unverified. The fallback is `api.vikingthe.dev` with CORS and an exact `Origin` allowlist. Ask first.
  - How a Docker Image resource reaches the API's internal `:8081` is unconfirmed. The fallback is a shared Docker network, attached manually.

## Repo settings (applied by the human)

- secret scanning with push protection;
- Dependabot alerts;
- private vulnerability reporting, plus a `SECURITY.md`;
- a ruleset on `main` that requires a PR and the CI checks, with no approvals and no bypass, and blocks force-pushes and deletion. Required checks are jobs that always run (the `dorny/paths-filter` gate job), never path-skipped ones. Only the maintainer pushes.
