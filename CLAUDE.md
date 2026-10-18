# CLAUDE.md

This file has five parts:

- **Part 1: Behavioral guidelines**, adapted from [multica-ai/andrej-karpathy-skills](https://github.com/multica-ai/andrej-karpathy-skills/blob/main/CLAUDE.md).
- **Part 2: Code style**, covering general rules plus the Astro, Svelte and Rust specifics.
- **Part 3: Security.**
- **Part 4: Orchestration and review.**
- **Part 5: Project architecture rules**, taken from ADR 0001 (`docs/adr/0001-platform-topology.md`). The ADR holds the full rationale.

**Precedence:** Part 3 (Security) overrides everything else. For the other parts, where they conflict, later parts win.

**Tradeoff:** these guidelines favor caution over speed. Use judgment on trivial tasks.

---

# Part 1: Behavioral guidelines

## 1. Think before coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:

- **Assumptions:** state them explicitly. If you're uncertain, ask.
- **Several interpretations:** present them. Don't pick one silently.
- **A simpler approach exists:** say so, and push back when it's warranted.
- **Something is unclear:** stop, name what's confusing, and ask.
- **A task would break a rule in Part 3 or Part 5:** say which rule, and ask before proceeding.

## 2. Simplicity first

**Write the minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you wrote 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:

- Don't "improve" adjacent code, comments or formatting.
- Don't refactor things that aren't broken.
- Match the existing style, even if you'd write it differently.
- If you notice unrelated dead code, mention it. Don't delete it.

When your changes leave things unused:

- Remove the imports, variables and functions that *your* changes made unused.
- Don't remove pre-existing dead code unless asked.

**The test:** every changed line should trace directly to the request.

## 4. Goal-driven execution

**Define success criteria. Loop until they're verified.**

Turn tasks into goals you can check:

- "Add validation" → "Write tests for invalid inputs, then make them pass."
- "Fix the bug" → "Write a test that reproduces it, then make it pass."
- "Refactor X" → "Make sure the tests pass before and after."

For multi-step tasks, state a brief plan:

```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you work through the loop on your own. Weak ones ("make it work") need constant clarification. Use the checks in Part 5, §8 as your verification steps.

**These guidelines are working if** diffs contain fewer unnecessary changes, fewer rewrites are caused by overcomplication, and clarifying questions come before implementation instead of after mistakes.

---

# Part 2: Code style

**Goal: short code whose logic you can read without comments.** When code needs a comment to explain *what* it does, rename or extract until it doesn't.

## General

**Naming**

- Use full words. The only abbreviations allowed are the standard ones: `id`, `url`, `db`, `http`.
- Functions are verbs (`create_session`, `formatDate`).
- Booleans read as questions (`is_expired`, `hasAccess`).
- A name should make the comment above it unnecessary.

**Functions**

- One function does one thing. Extract once a block needs a name to be understood.
- **Shape:**
  - Use guard clauses and early returns instead of nesting. Keep to at most two levels of nesting.
  - Keep parameter lists short. Past three parameters, pass a typed struct or object.
  - Prefer pure functions. Keep I/O and side effects at the edges: handlers, endpoints and event handlers.

**Values**

- No magic numbers or strings. Use named constants, for example `SESSION_TTL`.
- **Types do the checking:** make illegal states unrepresentable.
  - In Rust, use enums and newtypes.
  - In TypeScript, use discriminated unions and literal types.
  - Never validate at runtime what a type could guarantee.

**Organization**

- Organize code by feature (`auth/`, `sessions/`), not by layer (`models/`, `utils/`).
  - Never create a `utils` or `helpers` dumping ground.
  - Shared code gets a module named for what it does.
- Delete code rather than commenting it out. Git keeps the history.

**Comments**

- Write a comment only when the code can't be made self-explanatory. Typical cases:
  - a non-obvious *why*;
  - a workaround, with a link to the issue;
  - a constraint imposed from outside, such as a spec rule or a browser quirk.
- Keep them to one line where possible. Never restate the code.
  - Bad: `// increment counter`
  - Good: `// Valkey TTL refresh is throttled to 1/h to avoid a write per request`
- `TODO`s carry a reason, for example `// TODO(perf): batch these lookups once >1k users`.
- Doc comments go only on public items whose behavior isn't obvious from the signature. One sentence, plus an `# Errors` line where it applies.

**Formatting is the tools' job.** Never hand-format, and never argue style the formatter decides.

- **JS/TS/Svelte/Astro:** Prettier, with `prettier-plugin-svelte` and `prettier-plugin-astro`.
- **Lint:** ESLint, with `typescript-eslint`, `eslint-plugin-svelte` and `eslint-plugin-astro`.
- **Rust:** `rustfmt` with default settings.

## TypeScript and Astro

**TypeScript setup**

- `tsconfig.json` extends `astro/tsconfigs/strictest`, with `verbatimModuleSyntax: true`.
- Use `import type` for type-only imports.
- **Types:**
  - No `any`. Use `unknown` and narrow it.
  - No non-null `!` outside tests.
  - No TypeScript `enum`. Use string-literal unions.

**Components and props**

- Type component props with an `interface Props` in the frontmatter.
- Type `App.Locals` (for example `user`) in `src/env.d.ts`.
- Import within an app through a path alias (`@/…` via `compilerOptions.paths`), not deep `../../..` chains. Import packages by name (`@vikingthedev/ui`).

**Astro vs. Svelte**

- **Default to `.astro` components.** They ship no JS.
- Use a Svelte component only for an interactive island.
- Hydrate with `client:visible` or `client:idle`. Use `client:load` only for above-the-fold interactivity.

**Environment variables:** use the `astro:env` schema (`envField`), never raw `import.meta.env` or `process.env` in app code.

- Secrets are `context: "server", access: "secret"`.
- Set `validateSecrets: true`.

**Images:** use `<Image>` or `<Picture>` from `astro:assets`, with explicit dimensions. This protects LCP and CLS.

**Content:** structured content (blog, docs) lives in content collections with a Zod schema in `src/content.config.ts`.

**Frontmatter**

- Data fetching and shaping happen there.
- Templates only render. Keep logic out of the markup.

## Svelte 5

Sources: [Svelte best practices](https://svelte.dev/docs/svelte/best-practices) and the legacy-feature migrations.

**Runes only.** No legacy syntax:

| Don't use | Use instead |
|---|---|
| `export let` | `$props()` |
| `$:` | `$derived` and `$effect` |
| `<slot>` | `{#snippet}` and `{@render}` |
| `use:action` | `{@attach}` |
| `<svelte:component>` | A dynamic component |
| The `class:` directive | clsx-style arrays and objects in `class` |
| Stores, for new code | `$state` |

**State**

- Use `$state` only for values that must be reactive.
- Use `$state.raw` for large objects that are reassigned rather than mutated, such as API responses.
- **Derived values:**
  - Compute derived values with `$derived`, never by setting state inside an `$effect`.
  - Pass `$derived` an expression. Use `$derived.by` when the logic needs a block.
  - Keep `$derived` free of side effects.
- **`$effect` is an escape hatch**, used only to sync with something outside Svelte.
  - User actions belong in event handlers.
  - Window and document listeners belong in `<svelte:window>` and `<svelte:document>`.
  - Don't wrap effects in `if (browser)`. Effects don't run on the server.

**Props:** treat them as values that can change. Anything computed from props is `$derived`.

**Templates**

- `{#each}` is always keyed by a stable ID, never by index.
- Use `{#key}` sparingly.
- Use event attributes (`onclick={…}`), not `on:click`.

**Styling**

- Pass values from JS into CSS with `style:--name={value}`.
- Let parents style children through CSS custom properties.
- Use `:global` only for third-party markup.

**Sharing state**

- Share state through context created with `createContext`, never through module-level state, which leaks between users during SSR.
- For shared reactive logic, use classes with `$state` fields.

**`packages/ui` components**

- One component per file, with a typed `Props`.
- No data fetching. Components receive data, they don't load it.
- No app-specific imports.

## Rust

**Toolchain and lints**

- Use edition 2024 with `rustfmt` defaults.
- Configure lints once in the workspace:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
unwrap_used = "warn"
expect_used = "warn"
dbg_macro = "warn"
todo = "warn"
print_stdout = "warn"
```

- **CI:** `cargo clippy --workspace --all-targets -- -D warnings`.
- **`#[allow]`:** every one is local, with a short reason, for example `#[allow(clippy::cast_possible_truncation, reason = "ids < 2^32")]`.

**Errors**

- Domain errors are `thiserror` enums.
- The API crate has one `AppError` that implements `IntoResponse`, so handlers return `Result<T, AppError>` and use `?`.
  - Never leak internal error text to clients.
  - Log it, and return a stable error code instead.
- `anyhow` is allowed only in binaries' startup code and in tooling (`openapi-gen`).
- **No `unwrap` or `expect` in request paths.** `expect("reason")` is fine at startup, where failing fast is correct.

**Naming and types** (from the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html))

- **Casing:** follow RFC 430 (C-CASE).
- **Conversion names:** `as_`, `to_`, `into_` (C-CONV).
- **Getters:** no `get_` prefix (C-GETTER).
- **Conversion traits:** implement `From`, `TryFrom` and `AsRef` rather than ad-hoc constructors (C-CONV-TRAITS).
- **Newtypes:** wrap domain values (`UserId`, `SessionToken`, `EmailAddress`) and validate in their constructor (C-NEWTYPE).
- **Argument types:** use an enum instead of a `bool` or `Option` argument whose meaning isn't obvious at the call site (C-CUSTOM-TYPE).

**Code shape**

- Prefer iterator chains over manual loops when they read better.
- Borrow rather than clone. Take `&str` or `&[T]` in parameters, and return owned types.
- Match exhaustively on your own enums. No `_ =>` catch-all that would silently accept new variants.

**axum**

- **Handlers stay thin:** extract, call a domain function, map the result to a response. Business logic never lives in a handler.
- **Routes:** each feature module exposes `pub fn routes() -> Router<AppState>`, and `main` merges them.
- **Shared state:** `AppState` holds clients that are cheap to clone (the Valkey pool, the DB pool, config behind an `Arc`), and is passed with `State<AppState>`.
- **DTOs:**
  - Request and response DTOs are separate from database rows and domain types.
  - DTOs derive `serde` and `utoipa::ToSchema`.
  - Request DTOs use `#[serde(deny_unknown_fields)]`.
  - JSON fields keep serde's default snake_case.

**Async**

- Never block inside async code. CPU-heavy work such as password hashing goes in `tokio::task::spawn_blocking`.
- Never hold a lock across `.await`.

**Logging:** use `tracing` only, never `println!`.

- Put `#[tracing::instrument(skip_all, fields(…))]` on handlers and domain entry points.
- Never log tokens, passwords or full cookies.

**Tests**

- Unit tests sit in `#[cfg(test)] mod tests` next to the code.
- HTTP-level tests live in `crates/api/tests/` and drive the `Router` directly with `tower::ServiceExt::oneshot`.

**Database:** the PostgreSQL access library is not decided yet. Ask before choosing one.

---

# Part 3: Security

**Security is paramount.** The repo must be safe to make public at any moment.

- Assume an attacker has read every line of code, config and history. Only secrets are secret.
- Nothing may rely on obscurity.

## Secrets

- **Never in git.** No secrets in source, config, tests, fixtures, docs, commit messages or history. A secret that is ever committed is burned: rotate it, don't just delete it.
- **Never in front of Claude.**
  - Claude never reads, prints, echoes, logs or asks for a secret value.
  - If a task seems to need one, stop and ask the human to do that step.
  - If a secret appears in output anyway, stop, say so, and ask the human to rotate it.

**Production**

- **Where secrets live:** only in Coolify environment variables, injected at runtime.
- **Never at build time:** not as build variables, not as Docker `ARG`s, and never baked into image layers.
- **Images:** built in CI and contain zero secrets.

**GitHub Actions**

- **The only secrets it holds:**
  - `COOLIFY_TOKEN`, with deploy permission only;
  - the Coolify webhook URLs;
  - `CLOUDFLARE_API_TOKEN`, scoped to cache purge on the zone.
- **Scope:** store them as *environment* secrets on `staging` and `production`. Jobs triggered by pull requests never see them.
- **Registry:** GHCR uses the built-in `GITHUB_TOKEN`.

**Local development** uses no real secrets by default.

- **Throwaway values:** `compose.dev.yaml` runs PostgreSQL, Valkey and MinIO (S3-compatible, standing in for R2) bound to `127.0.0.1`.
  - Their values live in the committed `dev.env`.
  - These values are not secrets and must never be reused anywhere else.
  - Claude may read `dev.env` and run the dev stack with it.
- **Real development credentials**, if one is ever needed (for example a dev OAuth app or a dev R2 bucket):
  - Each is development-only, least-privilege, and revocable.
  - It lives **outside the repo** in `~/.config/vikingthedev/secrets.env` (mode `600`).
  - The **human** loads it with `scripts/with-secrets <command>`, in a **separate terminal**. Never via the `!` prefix: `!` commands run unsandboxed, and their output lands in Claude's context.
  - **Claude never runs `scripts/with-secrets`.** Claude is never started from a shell with secrets exported.
- **What enforces this** (`.claude/settings.json`; this file only describes it):
  - **The Bash sandbox is the real boundary.**
    - It denies reads of `~/`, except the project and the toolchain caches. The cargo credentials files stay denied even inside the allowed `~/.cargo`.
    - It denies reads of `.env*` and `*.pem` inside the project.
    - It denies writes to `.claude/` and `scripts/with-secrets`.
    - `allowUnsandboxedCommands: false` removes the escape hatch.
    - Network access is limited to package registries, GitHub and GHCR.
  - **`Read` deny rules** cover Claude's file tools, which run outside the sandbox: `.env*`, `*.pem`, `~/.config/vikingthedev`, `~/.ssh`, `~/.aws`, `~/.docker`, `~/.config/gh`, the cargo credentials files, `~/.npmrc`, `~/.git-credentials`, `~/.netrc` and `/proc`.
  - **`Edit` deny rules** protect all Claude settings files, so the guardrails can't be rewritten.
  - **`credentials.envVars`** strips secret variable names from sandboxed commands.
  - **The deny on `scripts/with-secrets` is only a speed bump** (`bash scripts/with-secrets` bypasses it). The rule above is what actually counts.
- **Never weaken these settings.** Propose changes to them and let the human apply them.

**Untested assumptions:** the human checks these once, whenever the setup or the settings change.

- **Secret files:** put a sentinel value in a dummy `.env`. Both Claude's Read tool and a sandboxed `cat .env` must fail.
- **Other processes:** a sandboxed `cat /proc/<pid>/environ` of a process the human started must fail.
- **Toolchain:** `pnpm install`, `cargo build` and `docker compose -f compose.dev.yaml up` must work inside the sandbox.
  - The toolchain `allowRead` paths are guesses.
  - Docker socket access from the sandbox is untested.
- **Requirements:** the sandbox needs `bubblewrap` and `socat`. With `failIfUnavailable: true`, Claude Code won't start without them.

**Example files:** use `env.example` (no leading dot), which lists variable names with placeholder values. It is the only env-shaped file besides `dev.env` that may be committed. `.gitignore` covers `.env`, `.env.*` and `*.pem`.

## Code

**Input and data access**

- **Validate at the boundary:** every external input (HTTP body, query, header, cookie, file, environment) is parsed into a validated type at the edge. Interior code never sees raw input.
- **Queries:** parameterized SQL only. Never build SQL, shell commands, URLs or file paths by string concatenation from input.
- **Authorization:**
  - It's checked in the domain layer for every resource access (to prevent IDOR), not just "is logged in".
  - Deny by default.

**Authentication**

- **Passwords:** hash with Argon2id (the `argon2` crate) inside `spawn_blocking`.
- **Tokens:** generate them with an OS CSPRNG (`rand::rngs::OsRng` or `getrandom`).
- **Crypto:** never roll your own.
- **Session tokens:**
  - Issue a new token on login and on any privilege change.
  - On logout, delete it server-side.
- **Rate limiting:** login, signup, password reset and any endpoint that sends email are rate-limited per IP and per account (counters in Valkey).
- **Account enumeration:** error messages don't reveal whether an account exists ("invalid email or password").

**Output and headers**

- **HTML injection:** Svelte and Astro escape output by default. `{@html}` and `set:html` are forbidden unless the input is sanitized server-side, and every use is flagged in review.
- **Security headers** on every HTML response:
  - a strict `Content-Security-Policy`:
    - no `unsafe-inline` or `unsafe-eval` for scripts;
    - inline scripts (including Astro's island hydration scripts) allowed by **hash, not nonce**, because prerendered HTML is cached at Cloudflare's edge and a per-request nonce can't work there;
    - use Astro's built-in CSP hash generation (*unverified:* the exact Astro 7 config key);
    - `frame-ancestors 'none'`, sent as an HTTP header from Traefik or Cloudflare, because browsers ignore `frame-ancestors` in a `<meta>` CSP;
  - `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload`;
  - `X-Content-Type-Options: nosniff`;
  - `Referrer-Policy: strict-origin-when-cross-origin`;
  - a restrictive `Permissions-Policy`.
- **Errors:** clients get a stable error code, never stack traces, SQL or internal messages.
- **Logs:** never log tokens, passwords, cookies, `Authorization` headers or secret env values. Keep PII to the minimum needed.

**Uploads and storage**

- R2 buckets are private by default.
- Uploads are checked for size and content type server-side.
- Downloads go through short-lived presigned URLs.

**Rust:** `unsafe_code = "forbid"` is set workspace-wide.

## Supply chain and CI

- **Dependencies:**
  - Keep them few. Every new dependency is justified in the commit message.
  - Lockfiles (`Cargo.lock`, `pnpm-lock.yaml`) are committed.
  - pnpm runs no dependency install scripts unless they're explicitly listed in `onlyBuiltDependencies`.
- **Scanning, on every push:**
  - `cargo deny check` (advisories, licenses, sources);
  - `pnpm audit --prod`;
  - `gitleaks` on the full history.
  - Dependabot opens update PRs.
- **Workflows:**
  - Start from `permissions: {}`, and grant each job only what it needs.
  - Pin third-party actions to a full commit SHA.
  - Never use `pull_request_target`.
  - Never interpolate `${{ github.event.* }}` text into `run:` scripts. Pass it through `env:` instead.
- **Containers:**
  - Distroless images, run as a non-root user.
  - Read-only root filesystem where possible.
  - No package managers or shells in runtime images.
- **GitHub repo settings:**
  - secret scanning with push protection;
  - Dependabot alerts;
  - private vulnerability reporting, plus a `SECURITY.md`;
  - branch protection on `main` that requires CI to pass.
- **Never** bypass hooks or checks (`--no-verify`, `continue-on-error` on a security step, `#[allow]` on a security lint) without asking.

---

# Part 4: Orchestration and review

## Models

The main session (the orchestrator) runs **Opus 5.5 or newer** (`"model": "opus"` in `.claude/settings.json`). The orchestrator owns the plan, the integration and every commit.

**Delegate to subagents by difficulty:**

| Model | Use for | Examples |
|---|---|---|
| **Haiku 5.5** | Mechanical, fully specified work where a mistake is easy to spot | File and code search, collecting call sites, renames, applying formatter or lint fixes, summarizing logs and CI output, boilerplate from an exact template |
| **Sonnet 5.5** | Well-scoped implementation inside one feature, with a clear spec and a check to run | A new endpoint or component following an existing pattern, writing tests, refactors covered by tests, docs |
| **Opus 5.5+** (the orchestrator, or an `opus` subagent) | Anything that needs judgment | Design and cross-cutting changes. Non-obvious debugging. **All security-sensitive code:** auth, sessions, CSRF, crypto, secrets handling, CI workflows, Dockerfiles, Coolify and Cloudflare config. All reviews. |

**Delegation rules**

- **Spawn only when it pays.** Do it inline if that's faster than writing the brief. Run subagents in parallel only when their tasks are independent.
- **Every brief is self-contained:** the goal, the files involved, the constraints (point to the relevant parts of this file), and the exact check that proves it's done.
- **Subagents never commit, push or deploy.** Their output is unreviewed until the orchestrator has checked it and the review below has passed.
- **Escalate on doubt:** if a Haiku or Sonnet task turns out to touch security-sensitive code, take it back to Opus.

## Review before every commit

No commit without an Opus-level review of exactly what is being committed.

1. **Run the checks** for the change (Part 5, §8). Fix any failures first.
2. **Stage the change,** then get a review of the staged diff from one of these:
   - **The advisor tool**, with `advisorModel` set to `opus` (Opus 5.5 or newer).
     - The advisor sees the conversation, not the repo. Print `git diff --cached` into the conversation immediately before calling it.
     - This matters most for subagent-written code, which isn't otherwise in the orchestrator's context.
   - **The `reviewer` subagent** (`.claude/agents/reviewer.md`, read-only, `model: opus`), which reads the diff itself. Use it if the advisor is unavailable, or if the diff is too large to print.
   - **If the diff touches security-sensitive code** (as listed in the delegation table above), also run `/security-review`.
3. **The review covers:**
   - correctness;
   - Part 3 (Security), including any secret, token or credential in the diff;
   - Part 2 (Code style);
   - Part 5 (Project rules).
4. **Fix every correctness or security finding.**
   - If you disagree with a finding, tell the human why. Don't silently skip it.
   - Re-review after any non-trivial fix.
5. **Commit only when the review has no open correctness or security findings.** Report any style findings you chose not to act on in your reply.

This applies equally to code that subagents wrote.

**Enforcement:** this part is guidance. To make the review mandatory, add a `PreToolUse` hook on `git commit` that blocks the commit unless a review marker exists for the current staged-tree hash.

---

# Part 5: Project: the vikingthe.dev suite

## What this is

A suite of websites, maintained by one person, in the monorepo `vikingthedev-platform`. Workspace packages use the `@vikingthedev/` scope, for example `@vikingthedev/ui`.

- **Apps:** the homepage at `vikingthe.dev`, plus about 5–6 apps at `<name>.vikingthe.dev`. Each app is released on its own.
- **Primary metric:** performance.
- **Hard requirement:** a session started on the homepage must survive navigation to every subdomain app.

**Stack**

| Layer | Choice |
|---|---|
| Frontend | Astro 7 + Svelte 5 (`@astrojs/svelte`, `@astrojs/node`) |
| Backend | Rust (axum 0.8) |
| Data | PostgreSQL and Valkey, each a separate Coolify resource |
| Object storage | Cloudflare R2 |
| Deploys | Coolify on Docker |
| CI | GitHub Actions |
| CDN | DNS proxied through Cloudflare |

## Repository layout

```
apps/homepage/            Astro + Svelte
apps/<app>/               one per subdomain
packages/ui/              Svelte component library (workspace:*, consumed as source)
packages/api-types/       generated from OpenAPI; committed; never hand-edited
packages/config/          shared tsconfig, size-limit and Lighthouse CI presets
crates/api/               axum binary (:8080 public, :8081 internal)
crates/openapi-gen/       writes the OpenAPI JSON
Cargo.toml                Cargo workspace
pnpm-workspace.yaml
docs/adr/
.github/workflows/
```

- **One monorepo:** pnpm and Cargo workspaces.
  - No Turborepo or moon yet.
  - If a typical PR's CI run passes about 5 minutes, propose moon rather than Turborepo.
- **`packages/ui`:**
  - It's consumed as source and never published. No Changesets, no build step.
  - Components must tree-shake, so no barrel files with side effects.
- **New app:**
  1. Add `apps/<name>/`.
  2. Give it its own path filter in CI.
  3. Give it its own Coolify Docker Image resource.
  4. Give its host an `/api` path domain on the Rust resource.

## 1. URLs and view transitions

- **App placement:**
  - Apps live on subdomains of `vikingthe.dev`.
  - **Never** put an app on a different registrable domain. That breaks the shared session.
- **Transitions inside an app:**
  - Use native cross-document view transitions, set with `@view-transition { navigation: auto; }` in the shared layout from `packages/ui`.
  - **Do not add Astro's `<ClientRouter />`.** It ships JS on every page.
- **Transitions between apps:** navigation from one subdomain to another never animates. The spec requires the same origin. Don't try to work around it.
  - If an app needs animated entry from the homepage, the sanctioned fix is to mount it at `vikingthe.dev/<app>`.
  - Ask before doing that.
- **Firefox:** it has no cross-document view transitions. Accept instant navigation there and don't polyfill.

## 2. Sessions and auth

**The cookie** is exactly:

```
__Secure-vt_sid=<base64url 256-bit random>; Domain=vikingthe.dev; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=2592000
```

- **Prefix:** not `__Host-`, which forbids `Domain`.
- **SameSite:** not `Strict`, because it breaks arrivals from external links.
- **Token:** an opaque token. No JWTs.

**Who owns sessions**

- Rust is the only place that creates, validates or deletes sessions.
- Astro never reads Valkey directly.

**Valkey layout**

- **Session record:** `sess:{sha256(token)}` is a hash of `{user_id, created, last_seen, ua_hash}`.
  - It expires after 30 days.
  - Refresh the expiry at most once per hour.
- **Per-user index:** `user_sess:{user_id}` is a set of that user's sessions, used for revoke-all.
- **Hashing:** never store the raw token.
- **Persistence:** Valkey runs with AOF on. Treat it as state, not as a disposable cache.

**Astro middleware** (`src/middleware.ts`)

- **On-demand routes:** forward the `Cookie` header to `GET http://<api-internal>:8081/session`.
  - Use a 50 ms timeout. A timeout means anonymous.
  - Set `Astro.locals.user`.
- **Prerendered routes:** do nothing.

**Logout:** `POST /api/auth/logout` deletes the Valkey key and expires the cookie with the same `Domain`.

## 3. API routing and CSRF

- **One path domain per host:** the Rust service gets an `/api` path domain on every app host (`https://<host>/api`).
  - Browser code calls only its own origin's `/api`.
  - **Don't introduce `api.vikingthe.dev` or CORS.** That's the documented fallback, and it needs explicit approval.
- **Two listeners:**
  - `:8080` is public, behind Traefik.
  - `:8081` is internal, on the Docker network only, and never gets a domain. SSR and server islands call `:8081`.
- **CSRF middleware** in Rust covers POST, PUT, PATCH and DELETE on `:8080`, and runs **before** auth.
  - Require `Sec-Fetch-Site: same-origin`.
  - If that header is absent, require an exact `Origin` allowlist **and** `Content-Type: application/json`.
  - Reject `same-site` with a 403.
- **Internal listener:** `:8081` skips Fetch Metadata checks.
- **Writes from the browser:** every write, including login and logout, is a JSON `fetch()`. Never use native `<form>` posts to the API.
- **Astro security settings:**
  - Keep `security.checkOrigin` on.
  - Set `security.allowedDomains` to the app's own host.
- **DNS hygiene:** never point a `*.vikingthe.dev` record at a third-party service. Every subdomain receives the session cookie.

## 4. The API contract

- `utoipa` derives OpenAPI from the axum handlers.
- `openapi-typescript` generates `packages/api-types`.
- `openapi-fetch` is the client.
- **Changing a request or response shape:**
  1. Change the Rust code.
  2. Regenerate the types.
  3. Fix the frontends.
  4. Commit all three together.
- **Never hand-edit `packages/api-types`.**

## 5. Performance budgets

Treat a budget regression as a failing test, not a judgment call.

| Metric | Field target (p75) | CI gate (Lighthouse, mobile, staging) |
|---|---|---|
| LCP | ≤ 1.8 s | ≤ 2.0 s |
| INP | ≤ 100 ms | TBT ≤ 100 ms (stands in for INP, which a lab run can't measure) |
| CLS | ≤ 0.05 | ≤ 0.05 |
| TTFB, static HTML from the Cloudflare cache | ≤ 200 ms | ≤ 200 ms |
| TTFB, server-rendered route | ≤ 400 ms | ≤ 400 ms |
| JS per route (gzip) | content ≤ 15 KB, app ≤ 75 KB | `size-limit` plus Lighthouse `resource-summary:script:size` |
| Backend | p50 ≤ 10 ms, p99 ≤ 50 ms for reads, session check p99 ≤ 5 ms | `oha` against staging, run nightly |

**Rendering defaults**

| Route | Rendering | Caching |
|---|---|---|
| Content (homepage, marketing, blog, docs) | Prerendered (`output: 'static'`) | Cloudflare edge |
| User-specific fragments on static pages | Server island (`server:defer`) or a client fetch to `/api/me` | Static shell stays cacheable |
| Logged-in app pages | On demand | `private, no-store` |

- Don't make a content page render on demand just to show user state.
- **Hydration:** hydrate Svelte islands only when they need interactivity. Prefer `client:visible` or `client:idle` over `client:load`.
- **Field data:** the `web-vitals` library reports to `/api/rum`.

## 6. Caching

- **Hashed assets** (`/_astro/*`): `Cache-Control: public, max-age=31536000, immutable`.
- **Prerendered HTML:** a Cloudflare Cache Rule with an Edge TTL. It's purged after each production deploy.
- **Cloudflare bypass:** `/api/*` and `/_server-islands/*`.
- **Responses that depend on the user:** `Cache-Control: private, no-store`. Never set `Set-Cookie` on a response meant to be cached.
- **R2, public objects:** served from a custom domain (for example `assets.vikingthe.dev`).
- **R2, private objects:** short-lived presigned URLs from Rust, never cached.

## 7. Build and deploy

- **Build:** GitHub Actions builds the images and pushes them to GHCR, tagged `:<sha>` plus a moving `:staging` or `:prod` tag.
- **Coolify:**
  - Coolify does **not** build. Each service is a Docker Image resource.
  - Auto Deploy is off.
  - Deploys are triggered only from Actions, via the authenticated Deploy Webhook.
- **Images:**
  - **Rust:** `cargo-chef` plus the BuildKit cache, on a distroless `cc` base.
  - **Astro:** `pnpm deploy --filter <app> --prod`.
- **Health endpoints:** every service exposes one that returns the git SHA it is running.
- **Pipeline, per changed target:**
  1. Checks
  2. Build and push the image
  3. Staging webhook
  4. Wait for the health endpoint to report the new SHA
  5. Playwright and Lighthouse CI
  6. Production webhook
  7. Cloudflare purge
- **Path filters** (`dorny/paths-filter`):

  | Target | Paths |
  |---|---|
  | `api` | `crates/**`, `Cargo.toml`, `Cargo.lock` |
  | `contract` | `crates/**`, `packages/api-types/**` |
  | `<app>` | `apps/<app>/**`, `packages/**`, `pnpm-lock.yaml` |

  Don't add `crates/**` to the app filters.
- **Staging:**
  - Lives on a separate registrable domain (for example `vikingthe-staging.dev`) with the same layout as production.
  - Never create staging hosts under `vikingthe.dev`. They would receive the production cookie.

## 8. Checks (use these as verification steps)

| Change | Verify with |
|---|---|
| Any JS or Svelte code | `pnpm -r check` (`astro check` and `svelte-check`), `pnpm lint`, `pnpm format:check`, and the affected app's tests |
| Rust code | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` |
| API shapes | The contract check, below |
| Bundle size | `size-limit` in the affected app |
| Dependencies or any commit | `gitleaks git --staged` (or `gitleaks protect --staged` on older versions), `cargo deny check`, `pnpm audit --prod` |
| Session, auth or CSRF | Playwright on staging: the cookie has the right attributes; the session shows up in a subdomain app's server-rendered HTML; logout makes `/api/me` return 401 on every host; CSRF `same-site` and `cross-site` requests get 403 while a `same-origin` control request succeeds |
| Transitions | A transition fires within an app and does not fire across subdomains. The test listens for `pagereveal` and reads `event.viewTransition`. |

**The contract check:**

```sh
cargo run -p openapi-gen > packages/api-types/openapi.json
pnpm --filter api-types generate
git diff --exit-code packages/api-types
pnpm -r check
```

These commands describe the planned setup. If one doesn't exist yet, say so instead of inventing a substitute.

## 9. Known risks (accepted without verification)

If you hit one of these, use the fallback and record the outcome in the ADR.

| Risk | Fallback |
|---|---|
| Coolify might not route `/api` and `/` on the same host to different resources. | `api.vikingthe.dev` with CORS and an Origin allowlist. Ask first. |
| How a Docker Image resource reaches the API's internal `:8081` is unconfirmed. | Attach the resources to a shared Docker network manually |
| `pnpm deploy` in pnpm 10 may need `inject-workspace-packages=true`. | Set it |
| Astro's cache headers on server-island responses are unknown. | Keep the Cloudflare bypass rule |
| Whether `tower-sessions` hashes session IDs at rest is unknown. | Hand-roll the session store on `fred` |

## 10. When to stop and ask

Ask before any change that would:

- put an app off `vikingthe.dev`, or move an app from a subdomain to a path;
- change the session cookie's attributes, or have Astro talk to Valkey directly;
- add CORS, `api.vikingthe.dev` or `<ClientRouter />`;
- have Coolify build images, or turn on Auto Deploy;
- split the monorepo, or publish `packages/ui`;
- loosen a performance budget;
- touch `.claude/settings.json`, secret handling, CSP or security headers, or GitHub workflow permissions;
- add a dependency that runs install scripts, or any new third-party GitHub Action;
- commit without a completed review.
