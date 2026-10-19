/** Gzipped JS budget for content routes (ADR 0001 §3), over all of an app's client JS (step 7b gate). */
export const CONTENT_ROUTE_BUDGET = [
  {
    name: "all client JS, an upper bound on any content route",
    path: "dist/client/_astro/*.js",
    gzip: true,
    // SI units: 15000 bytes, the same number as lighthouserc.json.
    limit: "15 kB",
  },
];
