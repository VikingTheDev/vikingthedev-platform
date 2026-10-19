# CLAUDE.md

These rules are always loaded for `vikingthedev-platform`. The rule files in `.claude/rules/` load as shown:

| File | Covers | Loads |
|---|---|---|
| `security-code.md` | Security rules for code, specs and config | Always |
| `rust.md` | Rust and the backend | For `crates/` and `Cargo.*` |
| `frontend.md` | Astro, Svelte and the frontend | For `apps/` and `packages/` |
| `ci-deploy.md` | CI, containers and deploys | For workflows, Dockerfiles and compose files |

**Precedence:** Security overrides everything. Otherwise, the more specific rule wins. The rationale lives in `docs/adr/`.

## Working style

Adapted from [andrej-karpathy-skills](https://github.com/multica-ai/andrej-karpathy-skills/blob/main/CLAUDE.md). These rules favor caution over speed; use judgment on trivial tasks.

- **Think first.**
  - State your assumptions. If you're uncertain, ask.
  - If a request has more than one reading, present them instead of picking one silently.
  - If a simpler approach exists, say so, and push back when it's warranted.
  - If something is unclear, stop and name what's unclear.
  - If a task would break a security rule or an architecture invariant, name the rule and ask before proceeding.
- **Keep it simple.** Write the minimum code that solves the problem.
  - No unrequested features, configurability or single-use abstractions.
  - No handling for cases that can't happen.
  - If 200 lines could be 50, rewrite it.
- **Make surgical changes.** Touch only what the task needs.
  - Don't reformat, refactor or "improve" adjacent code. Match the existing style.
  - Mention unrelated dead code instead of deleting it. Remove only what your own change left unused.
  - Every changed line should trace back to the request.
- **Work toward checkable goals.**
  - Turn each task into one: "fix the bug" becomes "write a failing test, then make it pass".
  - For multi-step work, state the plan as `step → verify: check`, using the checks below.

## Code style

**Code should be short, and its logic should read without comments.** If code needs a comment to explain *what* it does, rename or extract until it doesn't.

- **Names**
  - Use full words. The only allowed abbreviations are `id`, `url`, `db` and `http`.
  - Name functions as verbs and booleans as questions (`is_expired`).
- **Functions**
  - Each function does one job.
  - Use guard clauses and early returns, with at most two levels of nesting.
  - More than three parameters means a typed struct or object instead.
  - Keep functions pure where possible, with side effects at the edges.
- **Values**
  - Use named constants, not magic numbers or strings.
  - Make illegal states unrepresentable: enums and newtypes in Rust, discriminated unions in TypeScript. Never check at runtime what a type can guarantee.
- **Organization**
  - Organize by feature (`auth/`, `sessions/`), not by layer. No `utils` or `helpers` dumping grounds.
  - Delete code instead of commenting it out.
- **Comments** are only for:
  - a non-obvious *why*;
  - a workaround, with a link to the issue;
  - a constraint from outside the code, such as a spec rule or a browser quirk.

  Keep them to one line, and never restate the code: write `// Valkey TTL refresh is throttled to 1/h to avoid a write per request`, never `// increment counter`.

  - Every `TODO` carries a reason: `TODO(perf): …`.
  - Doc comments go only on public items whose behavior the signature doesn't show.
- **Formatting is the tools' job:**
  - Prettier, with the Svelte and Astro plugins.
  - ESLint, with `typescript-eslint`, `eslint-plugin-svelte` and `eslint-plugin-astro`.
  - `rustfmt` with default settings.

## Security (overrides everything)

The repo must be safe to make public at any moment. Assume an attacker has read all code, config and history. Only secrets are secret, and nothing may rely on obscurity.

- **Secrets never go in git.** That covers source, config, tests, fixtures, docs, commit messages and history. A committed secret is burned: rotate it, don't just delete it.
- **Secrets never go in front of Claude.**
  - Never read, print, echo, log or ask for a secret value.
  - If a task needs one, stop and ask the human to do that step.
  - If a secret appears in output, stop, say so, and ask the human to rotate it.
- **Production secrets** live only in Coolify environment variables, injected at runtime. Never as build variables, Docker `ARG`s or in image layers.
- **GitHub Actions** holds only:
  - `COOLIFY_TOKEN`, with deploy permission only;
  - the Coolify webhook URLs;
  - `CLOUDFLARE_API_TOKEN`, scoped to cache purge.

  They are environment secrets on `production` (and on `staging`, if one is ever added; ADR 0002), so pull request jobs never see them. GHCR uses `GITHUB_TOKEN`.
- **Local development uses no real secrets.**
  - `compose.dev.yaml` runs PostgreSQL and Valkey on `127.0.0.1`, plus an S3-compatible store once an app uses R2.
  - Their throwaway values come from the committed `dev.env`, which Claude may read and use.
  - Those values are never reused anywhere else.
- **Real development credentials** are rare. Each must be development-only, least-privilege and revocable.
  - They live in `~/.config/vikingthedev/secrets.env` (mode `600`).
  - Only the human loads them, with `scripts/with-secrets <cmd>` in a separate terminal.
  - Never use `!` for this: its output reaches Claude.
  - Claude never runs that script, and is never started from a shell with secrets exported.
- **DNS hygiene:** every `*.vikingthe.dev` host receives the session cookie. Never point a subdomain at a third-party service, and delete records for retired services.
- **Committed env files:** only `dev.env`, and `env.example`, which lists variable names with placeholder values. `.gitignore` covers `.env`, `.env.*` and `*.pem`.
- **Dependencies**
  - Keep them few, and justify each new one in its commit message.
  - Commit `Cargo.lock` and `pnpm-lock.yaml`.
  - pnpm runs install scripts only for packages approved in `allowBuilds` (pnpm 11+; `strictDepBuilds` fails the install otherwise). Approve builds only with `pnpm approve-builds` or a manual `allowBuilds` entry. Never run `pnpm approve` or `pnpm permissions approve`, and never use the `permissions` setting: they also link a package's agent skills into `.claude/skills`, and `permissions` overrides `allowBuilds`.
- **Guardrails:** `.claude/settings.json` enforces these rules, and `docs/claude-code-setup.md` explains them.
  - Never weaken or work around the settings. Propose changes, and let the human apply them.
  - Never bypass a check without asking: no `--no-verify`, no `continue-on-error` on a security step, no `#[allow]` on a security lint.

## Orchestration and review

The main session runs Opus 5.5 or newer and owns the plan, the integration and every commit. Delegate by difficulty:

| Model | Use for |
|---|---|
| Haiku 5.5 | Mechanical, fully specified work: search, collecting call sites, renames, lint and format fixes, summarizing logs, boilerplate from a template |
| Sonnet 5.5 | Scoped work inside one feature, with a spec and a check: endpoints or components that follow an existing pattern, tests, refactors covered by tests, docs |
| Opus 5.5+ | Anything needing judgment: design, cross-cutting changes, hard debugging, all reviews, and **all security-sensitive code** (auth, sessions, CSRF, crypto, secrets, CI, Dockerfiles, Coolify and Cloudflare config) |

- **When to delegate:** spawn a subagent only when that's faster than doing it inline, and run subagents in parallel only for independent tasks.
- **Briefs** are self-contained: the goal, the files, the constraints (cite the rule files), and the check that proves the task is done.
- **Limits:** subagents never commit, push or deploy. If a Haiku or Sonnet task turns out to be security-sensitive, take it back to Opus.

**Before every commit,** including commits of code a subagent wrote:

1. Run the checks for the change, and fix any failures.
2. Stage the change, then have the `reviewer` subagent (read-only, Opus) review the staged diff. Security-sensitive diffs also get `/security-review`.
3. Fix every correctness or security finding, or tell the human why not. Re-review after any non-trivial fix. Report the style findings you didn't act on.
4. Commit only when no correctness or security findings remain open.

**The advisor tool is off by default.** The human enables it with `/advisor`.
- When it's on, it gives a second opinion, never a replacement for `reviewer`.
- It sees only the conversation. Before calling it, print the staged diff and the checklist from `reviewer.md`.

## Project

A suite of websites, built by one maintainer: the homepage at `vikingthe.dev`, plus about 5–6 apps at `<name>.vikingthe.dev`. Each app is released on its own.

- **Performance is the primary metric.**
- A session started on the homepage must survive navigation to every app.

**Stack:** Astro 7 + Svelte 5 (`@astrojs/node`); Rust (axum 0.8); PostgreSQL (Diesel through diesel-async, ADR 0005) and Valkey as separate Coolify resources; Cloudflare R2; Coolify on Docker; GitHub Actions; Cloudflare in front of DNS. Workspace packages use the `@vikingthedev/` scope.

```
apps/<app>/            Astro + Svelte, one per host (apps/homepage/ first)
packages/ui/           Svelte component library, consumed as source, never published
packages/api-types/    generated from OpenAPI, committed, never hand-edited
packages/config/       shared tsconfig, size-limit and Lighthouse CI presets
crates/api/            axum binary (:8080 public, :8081 internal)
crates/openapi-gen/    writes the OpenAPI JSON
docs/adr/              numbered decision records
docs/specs/            one spec per app, from TEMPLATE.md
```

The repo is one monorepo, with pnpm and Cargo workspaces. There is no Turborepo or moon yet. Propose moon if a typical PR's CI run takes longer than about 5 minutes.

**Architecture invariants** (from ADR 0001; details are in the scoped rules)

- **Hosting:** apps live on subdomains of `vikingthe.dev`.
- **Session:** one session cookie, `__Secure-vt_sid` with `Domain=vikingthe.dev`. Rust alone creates, validates and deletes sessions.
- **API routing:** each host's `/api` routes to the Rust service, so browser calls are same-origin. No CORS, and no `api.` subdomain.
- **View transitions:** native cross-document view transitions within an app, and none across subdomains (the spec forbids cross-origin transitions). No `<ClientRouter />`.
- **Rendering:** content pages are prerendered and cached at the edge. Only logged-in pages render on demand.
- **Deploys:** CI builds the images and Coolify pulls them. Coolify never builds, and Auto Deploy is off.
- **Staging:** none for now; a full stack in CI replaces it (ADR 0002). If one is added, it lives on a separate registrable domain, never under `vikingthe.dev`.
- **Known risks:** ADR 0001 lists them, each with a fallback, and the scoped rules repeat the relevant ones. If you hit one, use its fallback and record the outcome in a new ADR.

**New app:** add `apps/<name>/`, a CI path filter for it, a Coolify Docker Image resource for it, and an `/api` path domain for its host on the Rust resource.

### Specs and decisions

- **Specs come before code.**
  - Each app's spec is `docs/specs/<app>.md`, created from `TEMPLATE.md`.
  - Implement a feature only once its spec section exists.
  - When behavior changes, update the spec in the same commit.
- **Open questions:** a spec's open questions must be resolved before implementation starts. Ask the human, and never pick an answer silently.
- **ADRs:** a decision that would be expensive to reverse gets a new ADR, `docs/adr/NNNN-<slug>.md`, numbered next in sequence. That covers a new category of dependency, an auth method, a data store, a public API shape, or a change to an architecture invariant.
  - Accepted ADRs are never rewritten. A later ADR supersedes them, and the superseded ADR's status is updated to point to its successor.

### Checks

| Change | Verify with |
|---|---|
| JS, TS or Svelte | `pnpm -r check` (`astro check`, `svelte-check`), `pnpm lint`, `pnpm format:check`, the affected app's tests |
| Rust | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` |
| API shapes | The contract check in `.claude/rules/rust.md` |
| Bundle size | `size-limit` in the affected app |
| Every commit | `gitleaks git --staged` (`gitleaks protect --staged` on older versions); also `cargo deny check` and `pnpm audit --prod` once those workspaces exist |
| Session, auth or CSRF | Playwright on the CI stack (ADR 0002): cookie attributes; the session appears in a subdomain app's server-rendered HTML; after logout, `/api/me` is 401 on every host; `same-site` and `cross-site` CSRF requests get 403 while a `same-origin` control succeeds |
| Transitions | The `pagereveal` test, which reads `event.viewTransition`: a transition fires within an app and not across subdomains |

These commands describe the planned setup. If one doesn't exist yet, say so instead of inventing a substitute.

### Stop and ask before

- moving an app off `vikingthe.dev`, or from a subdomain to a path;
- changing the session cookie, or letting Astro talk to Valkey directly;
- adding CORS, an `api.` subdomain or `<ClientRouter />`;
- letting Coolify build images, or turning on Auto Deploy;
- splitting the monorepo, or publishing `packages/ui`;
- loosening a performance budget;
- touching `.claude/settings.json`, secret handling, the CSP or security headers, or workflow permissions;
- adding a dependency that runs install scripts, or a third-party GitHub Action not on `ci-deploy.md`'s approved list;
- committing without a completed review.
