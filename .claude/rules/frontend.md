---
paths:
  - "apps/**"
  - "packages/**"
---

# Frontend rules: Astro, Svelte 5 and TypeScript

## TypeScript and Astro

**Compiler setup**

- `tsconfig.json` extends `astro/tsconfigs/strictest`, with `verbatimModuleSyntax: true`.
- Use `import type` for type-only imports.

**Types**

- No `any`. Use `unknown` and narrow it.
- No non-null `!` outside tests.
- No TypeScript `enum`. Use string-literal unions.

**Props and locals**

- Declare `interface Props` in the frontmatter.
- Type `App.Locals` in `src/env.d.ts`.

**Imports**

- Within an app, use the `@/…` alias instead of `../../..` chains.
- Import packages by name, for example `@vikingthedev/ui`.

**Components**

- Default to `.astro` components, which ship no JS.
- Use Svelte only for interactive islands.
- Hydrate islands with `client:visible` or `client:idle`. Use `client:load` only for interactivity above the fold.

**Environment variables**

- Go through the `astro:env` schema (`envField`). Never use `import.meta.env` or `process.env` in app code.
- Secrets are `context: "server", access: "secret"`.
- Set `validateSecrets: true`.

**Images:** use `<Image>` or `<Picture>` from `astro:assets`, with explicit dimensions. This protects LCP and CLS.

**Structured content** goes in content collections, with a Zod schema in `src/content.config.ts`.

**Data:** fetch and shape data in the frontmatter. Templates only render.

## Svelte 5

Source: [Svelte best practices](https://svelte.dev/docs/svelte/best-practices).

**Runes only.** No legacy syntax:

| Don't use | Use instead |
|---|---|
| `export let` | `$props()` |
| `$:` | `$derived` and `$effect` |
| `<slot>` | `{#snippet}` and `{@render}` |
| `use:action` | `{@attach}` |
| `<svelte:component>` | A dynamic component |
| The `class:` directive | clsx-style arrays and objects in `class` |
| Stores, in new code | `$state` |

**State**

- Use `$state` only for values that must be reactive.
- Use `$state.raw` for large objects that are reassigned rather than mutated, such as API responses.

**Derived values**

- Compute them with `$derived`, never by setting state inside an `$effect`.
- Pass `$derived` an expression. Use `$derived.by` when the logic needs a block.
- Keep derived values free of side effects.

**Effects**

- Use `$effect` only to sync with something outside Svelte.
- User actions belong in event handlers.
- Window and document listeners belong in `<svelte:window>` and `<svelte:document>`.
- Don't wrap effects in `if (browser)`. Effects don't run on the server.

**Props:** props can change, so anything computed from them is `$derived`.

**Templates**

- Key every `{#each}` by a stable ID, never by index.
- Use `{#key}` sparingly.
- Use `onclick={…}`, not `on:click`.

**Styling**

- Pass JS values into CSS with `style:--name={value}`.
- Parents style children through CSS custom properties.
- Use `:global` only for third-party markup.

**Sharing state**

- Share state through `createContext`, never through module-level state, which leaks between users during SSR.
- For shared reactive logic, use classes with `$state` fields.

**`packages/ui`**

- Apps consume it as source with `workspace:*`. No build step, no Changesets, never published.
- One component per file, with a typed `Props`.
- No data fetching and no app-specific imports.
- No barrel files with side effects. Components must tree-shake.

## View transitions

- Put `@view-transition { navigation: auto; }` in the shared layout from `packages/ui`. Never add `<ClientRouter />`, which ships JS on every page.
- **Across subdomains:** never try to animate. The spec forbids transitions between different origins.
- **Animated entry from the homepage:** an app that needs this would move to `vikingthe.dev/<app>`. Ask first.
- **Firefox** has no cross-document transitions. Accept instant navigation there, with no polyfill.

## Sessions in Astro

**Validation**

- Astro never reads Valkey.
- Middleware (`src/middleware.ts`) runs on on-demand routes.
  - It forwards only the `__Secure-vt_sid` cookies (repeats included, so the API still treats several distinct tokens as anonymous) to `GET http://<api-internal>:8081/session`, with a 50 ms timeout (step 6 gate, 2026-10-19).
  - A timeout means the user is treated as anonymous.
  - It sets `Astro.locals.user`.
- On prerendered routes, middleware does nothing.

**Writes:** every browser write, including login and logout, is a JSON `fetch()` to the app's own `/api`. Never use native `<form>` posts to the API.

**Astro security settings**

- Keep `security.checkOrigin` on.
- Set `security.allowedDomains` to the app's own host, plus its `*.vikingthe.test` host for the CI stack (ADR 0002). Astro reads it at build time, so one image serves both.
- Set `session: false`: Rust alone creates sessions, and `@astrojs/node` otherwise enables Astro's own.

## Performance budgets

A budget regression is a failing test, not a judgment call.

| Metric | Field p75 | CI gate (Lighthouse mobile: the CI stack per PR, production after deploy; ADR 0002) |
|---|---|---|
| LCP | ≤ 1.8 s | ≤ 2.0 s |
| INP | ≤ 100 ms | TBT ≤ 100 ms (Lighthouse can't measure INP) |
| CLS | ≤ 0.05 | ≤ 0.05 |
| TTFB, static HTML from the Cloudflare cache | ≤ 200 ms | ≤ 200 ms, production only (the CI stack has no edge) |
| TTFB, server-rendered route | ≤ 400 ms | ≤ 400 ms |
| JS per route (gzip) | content ≤ 15 KB, app ≤ 75 KB | `size-limit` plus Lighthouse `resource-summary:script:size` |

| Route | Rendering | Caching |
|---|---|---|
| Content: homepage, marketing, blog, docs | Prerendered (`output: 'static'`) | Cloudflare edge |
| User-specific fragments on static pages | Server island (`server:defer`) or a client fetch to `/api/me` | The static shell stays cacheable |
| Logged-in app pages | On demand | `private, no-store` |

- Don't render a content page on demand just to show user state.
- Field data comes from the `web-vitals` library, which reports to `/api/rum`.

## Caching

- **Hashed assets** (`/_astro/*`): `Cache-Control: public, max-age=31536000, immutable`.
- **User-dependent responses:** `Cache-Control: private, no-store`. Never set `Set-Cookie` on a response meant to be cached.
- **Prerendered HTML:** a Cloudflare Cache Rule with an Edge TTL, purged after each production deploy.
- **Cloudflare bypass:** `/api/*` and `/_server-islands/*`. Keep the server-island bypass: the cache headers Astro sets on those responses are unverified.
- **R2, public objects:** served from a custom domain.
- **R2, private objects:** short-lived presigned URLs from Rust, never cached.
