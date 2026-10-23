/** Safe against production: no login, and no write beyond a logout without a session. */
export const SMOKE = { tag: "@smoke" };
/** Checks Cloudflare in front of production, so only the post-deploy smoke run includes it. */
export const PRODUCTION_ONLY = { tag: ["@smoke", "@production"] };
