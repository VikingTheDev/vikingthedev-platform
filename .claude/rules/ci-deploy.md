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
- **Triggers:** never use `pull_request_target`.
- **Event text:** never interpolate `${{ github.event.* }}` into `run:` scripts. Pass it through `env:`.
- **Secrets:** only jobs that deploy, on pushes to `main`, use the `staging` and `production` environments.
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

- **Images:** Actions builds them and pushes to GHCR, tagged `:<sha>` plus a moving `:staging` or `:prod` tag.
- **Coolify:**
  - Each service is a Docker Image resource.
  - Auto Deploy is off.
  - Deploys are triggered only by the authenticated Deploy Webhook, called from Actions.
- **Health:** every service has a health endpoint that returns its git SHA.
- **Pipeline,** for each changed target:
  1. checks;
  2. build and push the image;
  3. call the staging webhook;
  4. wait for the health endpoint to report the new SHA;
  5. run Playwright and Lighthouse CI;
  6. call the production webhook;
  7. purge the Cloudflare cache.
- **Path filters** (`dorny/paths-filter`):

  | Target | Paths |
  |---|---|
  | `api` | `crates/**`, `Cargo.toml`, `Cargo.lock` |
  | `contract` | `crates/**`, `packages/api-types/**` |
  | `<app>` | `apps/<app>/**`, `packages/**`, `pnpm-lock.yaml` |

  Never add `crates/**` to an app filter. A real contract change reaches the apps through `packages/api-types`.
- **Typecheck scope:** pull requests check only the affected packages, using `fetch-depth: 0` and `origin/<base_ref>`. Pushes to `main` check everything.
- **Staging:** it lives on its own registrable domain, laid out like production.
- **Known Coolify risks** (ADR 0001):
  - Routing `/api` and `/` on one host to different resources is unverified. The fallback is `api.vikingthe.dev` with CORS and an exact `Origin` allowlist. Ask first.
  - How a Docker Image resource reaches the API's internal `:8081` is unconfirmed. The fallback is a shared Docker network, attached manually.

## Repo settings (applied by the human)

- secret scanning with push protection;
- Dependabot alerts;
- private vulnerability reporting, plus a `SECURITY.md`;
- branch protection on `main` that requires CI.
