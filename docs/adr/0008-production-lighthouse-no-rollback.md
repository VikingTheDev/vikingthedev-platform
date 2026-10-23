# ADR 0008: A production Lighthouse failure doesn't roll back

- **Status:** Accepted (2026-10-24)
- **Date:** 2026-10-24
- **Decider:** Solo maintainer
- **Supersedes (in part):** in ADR 0002 point 2: Lighthouse CI as a rollback trigger. The other post-deploy smoke checks still roll back.

## Context

ADR 0002 rolls a release back when any post-deploy smoke check fails, including Lighthouse CI against production. Lighthouse runs from a GitHub-hosted runner: it's lab data, and its TTFB through the edge carries network noise. One slow sample would roll back a healthy release, and its re-run would redeploy the same code.

## Decision

- **Rollback triggers:** a failed redeploy, a failed health wait, or a failed functional smoke check. The smoke checks are the `@smoke` Playwright tests, which include the edge headers, the cache rules and Mozilla Observatory's A+.
- **Production Lighthouse** runs once per release, in a job of its own, only when every deploy job succeeded. A budget miss fails that job, so the run goes red and the report is kept as an artifact, but the release stays live.

## Consequences

- **Positive:** noise in lab timings never takes down a good release.
- **Negative:** a real performance regression stays live until someone acts on the red run.
- **Docs to update on acceptance:** ADR 0002's status; the rollback rule in `ci-deploy.md`.

## Revisit if

- field data (Cloudflare Web Analytics or CrUX) becomes available and stable enough to gate on;
- red Lighthouse runs start being ignored.
