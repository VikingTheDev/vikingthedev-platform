import { expect, test } from "@playwright/test";

import { APP_URLS } from "./hosts.ts";
import { SMOKE } from "./tags.ts";

for (const url of APP_URLS) {
  test(
    `${url} routes /health to the app and /api/health to the API`,
    SMOKE,
    async ({ request }) => {
      for (const path of ["/health", "/api/health"]) {
        const response = await request.get(`${url}${path}`);

        expect(response.status()).toBe(200);
        expect(response.headers()["cache-control"]).toBe("no-store");
        expect(await response.json()).toMatchObject({
          status: "ok",
          sha: expect.any(String),
        });
      }
    },
  );
}
