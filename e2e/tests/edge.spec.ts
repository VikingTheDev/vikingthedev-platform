import { expect, test, type APIResponse } from "@playwright/test";

import { APP_URLS, CANARY_URL, HOMEPAGE_URL } from "./hosts.ts";
import { PRODUCTION_ONLY } from "./tags.ts";

// docs/edge-config.md's `security-headers` rule; step 15 checks that the record and the live rule match.
const SECURITY_HEADERS = {
  "strict-transport-security": "max-age=63072000; includeSubDomains; preload",
  "x-content-type-options": "nosniff",
  "referrer-policy": "strict-origin-when-cross-origin",
  "permissions-policy":
    "accelerometer=(), browsing-topics=(), camera=(), display-capture=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()",
};
const EDGE_POLICY =
  "frame-ancestors 'none'; object-src 'none'; base-uri 'none'";
const UNCACHED = ["DYNAMIC", "BYPASS"];
const OBSERVATORY_SCAN_URL =
  "https://observatory-api.mdn.mozilla.net/api/v2/scan";
const CACHE_FILL_TIMEOUT_MS = 10_000;

for (const url of APP_URLS) {
  test(
    `${url} carries the edge's security headers`,
    PRODUCTION_ONLY,
    async ({ request }) => {
      const response = await request.get(url);

      expect(response.headers()).toMatchObject(SECURITY_HEADERS);
      expect(policies(response)).toContain(EDGE_POLICY);
    },
  );

  test(
    `${url}'s health endpoints bypass the edge cache`,
    PRODUCTION_ONLY,
    async ({ request }) => {
      for (const path of ["/health", "/api/health"]) {
        const response = await request.get(`${url}${path}`);

        expect(UNCACHED).toContain(response.headers()["cf-cache-status"]);
      }
    },
  );

  test(
    `${url} gets an A+ from Mozilla Observatory`,
    PRODUCTION_ONLY,
    async ({ request }) => {
      const response = await request.post(OBSERVATORY_SCAN_URL, {
        params: { host: new URL(url).host },
      });

      expect(response.ok()).toBe(true);
      expect((await response.json()).grade).toBe("A+");
    },
  );
}

test(
  "the prerendered homepage is served from the edge cache",
  PRODUCTION_ONLY,
  async ({ request }) => {
    // The deploy just purged the cache, so the first request may be a MISS.
    await expect
      .poll(
        async () =>
          (await request.get(HOMEPAGE_URL)).headers()["cf-cache-status"],
        { timeout: CACHE_FILL_TIMEOUT_MS },
      )
      .toBe("HIT");
  },
);

test(
  "canary bypasses the edge cache and keeps Astro's hashed CSP beside the edge's",
  PRODUCTION_ONLY,
  async ({ request }) => {
    const response = await request.get(CANARY_URL);

    expect(UNCACHED).toContain(response.headers()["cf-cache-status"]);
    // Both headers must arrive: the edge rule adds its CSP and never replaces Astro's (security-code.md).
    expect(policies(response)).toContain(EDGE_POLICY);
    expect(policies(response)).toContainEqual(
      expect.stringMatching(/script-src[^;]*sha256-/),
    );
  },
);

function policies(response: APIResponse): string[] {
  return response
    .headersArray()
    .filter(({ name }) => name.toLowerCase() === "content-security-policy")
    .map(({ value }) => value);
}
