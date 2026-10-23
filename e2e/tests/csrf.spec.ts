import { expect, test } from "@playwright/test";

import { CANARY_URL, HOMEPAGE_URL, logIn } from "./hosts.ts";
import { SMOKE } from "./tags.ts";

const LOGOUT_URL = `${CANARY_URL}/api/auth/logout`;
const FORBIDDEN_TYPE = "https://vikingthe.dev/problems/forbidden";
const ATTACKER_ORIGIN = "https://attacker.example";
// A sibling subdomain, the planted-cookie and same-site attacker of rust.md; Playwright serves it.
const SIBLING_ORIGIN = "https://sibling.vikingthe.test";

const forbiddenSenders = [
  { site: "same-site", origin: HOMEPAGE_URL },
  { site: "cross-site", origin: ATTACKER_ORIGIN },
];

for (const { site, origin } of forbiddenSenders) {
  test(`a ${site} write gets 403 forbidden`, SMOKE, async ({ request }) => {
    const response = await request.post(LOGOUT_URL, {
      headers: { "sec-fetch-site": site, origin },
      data: {},
    });

    expect(response.status()).toBe(403);
    expect((await response.json()).type).toBe(FORBIDDEN_TYPE);
  });
}

test("a same-origin write succeeds", SMOKE, async ({ request }) => {
  const response = await request.post(LOGOUT_URL, {
    headers: { "sec-fetch-site": "same-origin", origin: CANARY_URL },
    data: {},
  });

  expect(response.status()).toBe(204);
});

test("a no-cors text/plain POST from a sibling subdomain can't log the user out", async ({
  context,
  page,
}) => {
  await logIn(context, "Grace Hopper");
  // The CI stack sits on a private network, which Chromium's Local Network Access guards; production hosts are public.
  await context.grantPermissions(["local-network-access"], {
    origin: SIBLING_ORIGIN,
  });
  await page.route(`${SIBLING_ORIGIN}/`, (route) =>
    route.fulfill({
      contentType: "text/html",
      body: "<!doctype html><title>sibling</title>",
    }),
  );
  await page.goto(`${SIBLING_ORIGIN}/`);

  const [response] = await Promise.all([
    page.waitForResponse(LOGOUT_URL),
    page.evaluate(async (url) => {
      await fetch(url, {
        method: "POST",
        mode: "no-cors",
        credentials: "include",
        headers: { "content-type": "text/plain" },
        body: "{}",
      });
    }, LOGOUT_URL),
  ]);

  expect(response.status()).toBe(403);
  // Chromium hides an opaque response's body and headers even from Playwright, so the status and the surviving session are the evidence.
  expect(await (await context.request.get(CANARY_URL)).text()).toContain(
    "<p>Grace Hopper</p>",
  );
});
