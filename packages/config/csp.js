/** Directives every app adds to Astro's hashed script-src and style-src (step 5 gate, 2026-10-19). */
export const CSP_DIRECTIVES = [
  "default-src 'self'",
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'self'",
  "img-src 'self' data:",
  "connect-src 'self'",
];
