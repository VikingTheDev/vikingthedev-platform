# Design system and homepage sequence

- **Status:** Draft
- **Skill:** use `frontend-design` from stage 2 onwards.
- **Rule:** each stage ends at a checkpoint where the maintainer approves, edits or redirects before the next stage starts.

## Brief (from the maintainer)

- **Feel:** clean, modern, minimalist. A mountain topography theme, tactical with a tech twist.
- **Themes:** light and dark. Dark is dark grey, dark green and dark blue: rock, moss and vegetation, glacier.
- **References:**
  - `gertix.studio`: hero and components, striped lines, minimalism, tasteful spacing.
  - `netlight.com`: component design.
  - The maintainer can add more.
- **Homepage scope:** header, footer, hero, portfolio (blog, GitHub activity, highlighted repos), contact form, platform status, terminal easter egg.

## Stage 1: Content inventory

- List every section, the content each needs, who writes it, and its data source:
  - static copy;
  - content collection (blog, highlighted repos);
  - build-time fetch (GitHub, D15);
  - runtime (status, contact).
- Write a draft copy outline for the hero and the about section.
- Map the terminal commands to page actions (`help`, `ls`, `open blog`, `cat about`, `contact`, `status`, `theme dark`). Every command does something a normal UI element also does, so nothing is reachable only through the terminal.

**Checkpoint 1:** the maintainer approves the sections and their order, and writes or edits the copy. Then decide the open copy questions: the name or handle shown, whether a CV link appears, and which repos to highlight.

## Stage 2: Visual direction

- Study the references with `frontend-design`, and name the specific traits to borrow (line work, grid, spacing rhythm, type pairing).
- Build two or three direction boards as static HTML:
  - palettes;
  - a type pairing;
  - one hero and one card in each theme;
  - the contour treatment.
- **Contour lines** are pre-generated static SVG, no runtime JS. Lines are drawn with `currentColor` so they follow the theme. The hero's SVG is under 10 KB gzip.

**Checkpoint 2:** the maintainer picks one direction or mixes them.

## Stage 3: Tokens

- **Format:** DTCG JSON in `packages/ui/tokens/`. A script writes `tokens.css`, which is committed and checked in CI (D12).
- **Colour:**
  - OKLCH values, with three tonal families (granite grey, moss green, glacier blue) plus a signal colour for focus and warnings;
  - semantic tokens (`surface`, `text`, `muted`, `line`, `accent`) mapped for both themes;
  - contrast checked at WCAG 2.2 AA minimum for text, and 3:1 for UI parts and focus rings.
- **Theme switching:**
  - `prefers-color-scheme` is the default;
  - a toggle stores the choice in `localStorage`;
  - a tiny inline script in `<head>` sets `data-theme` before first paint, so there's no flash. It's allowed by its CSP hash.
- **Type:**
  - a fluid scale with `clamp()`, in the style of Utopia, from a 360 px to a 1440 px viewport;
  - at most two families, and probably a monospace one for the tactical and terminal accent;
  - self-hosted WOFF2, subset to Latin plus Norwegian characters;
  - one preloaded font file, with `font-display: swap` and fallback metric overrides (`size-adjust`, `ascent-override`) to keep CLS at 0.
- **Space, radius, line:** a fluid space scale, hairline borders as the main structure, and striped-line patterns as tokens.
- **Motion:** durations and easings; everything turned off under `prefers-reduced-motion`. The view-transition defaults live here.

**Checkpoint 3:** the maintainer reviews a token preview page in both themes.

## Stage 4: Components (`packages/ui`)

Default to `.astro` components with no JS. Use Svelte only where something is interactive.

| Component | Type | Notes |
|---|---|---|
| Layout (with `@view-transition`) | Astro | Security headers, theme script, font preload |
| Header, Footer | Astro | Navigation needs no JS. The menu uses `<details>` or a popover |
| Hero | Astro | Contour SVG plus a stripe treatment |
| Section heading, Card, Tag, Button, Link | Astro | |
| Theme toggle | Svelte (tiny) or plain JS | Must stay a few hundred bytes |
| Status badge | Astro shell plus a client fetch | |
| Contact form | Svelte | JSON `fetch()`, never a native `<form>` post (ADR 0001) |
| Terminal | Svelte, loaded on demand | `import()` on a keyboard shortcut or click. Not part of the initial JS |

**Checkpoint 4:** the maintainer reviews a component gallery page that passes axe and the size budgets.

## Stage 5: Homepage

- Assemble the approved sections with real content. Then run Lighthouse CI, `size-limit` (content route ≤ 15 KB JS), axe and Observatory.
- Iterate on spacing and motion with the maintainer, using screenshots in both themes at 360, 768 and 1440 px.

**Checkpoint 5:** the maintainer gives the go-ahead to launch.
