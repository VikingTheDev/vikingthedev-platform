import { expect, test, type Page, type Route } from "@playwright/test";

import {
  APP_URLS,
  CANARY_URL,
  HOMEPAGE_URL,
  PLATFORM_DOMAIN,
  logIn,
} from "./hosts.ts";
import { SMOKE } from "./tags.ts";

const DISPLAY_NAME = "Ada Lovelace";
const SESSION_COOKIE =
  /^__Secure-vt_sid=[A-Za-z0-9_-]{43}; Domain=vikingthe\.test; Path=\/; Secure; HttpOnly; SameSite=Lax; Max-Age=2592000$/;
const WELL_FORMED_UNKNOWN_TOKEN = "A".repeat(43);

test("the session cookie has exactly the ADR 0001 attributes", async ({
  context,
}) => {
  expect(await logIn(context, DISPLAY_NAME)).toMatch(SESSION_COOKIE);
});

test("a session from the homepage's host shows in canary's server-rendered HTML", async ({
  context,
}) => {
  await logIn(context, DISPLAY_NAME);

  const html = await (await context.request.get(CANARY_URL)).text();

  expect(html).toContain(`<p>${DISPLAY_NAME}</p>`);
});

test(
  "canary renders anonymous without a cookie or with an unknown one",
  SMOKE,
  async ({ context }) => {
    expect(await (await context.request.get(CANARY_URL)).text()).toContain(
      "<p>anonymous</p>",
    );

    await context.addCookies([
      {
        name: "__Secure-vt_sid",
        value: WELL_FORMED_UNKNOWN_TOKEN,
        domain: PLATFORM_DOMAIN,
        path: "/",
        secure: true,
        httpOnly: true,
        sameSite: "Lax",
      },
    ]);

    expect(await (await context.request.get(CANARY_URL)).text()).toContain(
      "<p>anonymous</p>",
    );
  },
);

test("the homepage's server island and /account show the session user", async ({
  context,
  page,
}) => {
  await logIn(context, DISPLAY_NAME);

  await page.goto(HOMEPAGE_URL);
  await expect(page.locator(".session")).toHaveText(DISPLAY_NAME);
  await page.goto(`${HOMEPAGE_URL}/account`);
  await expect(page.getByText(`Logged in as ${DISPLAY_NAME}.`)).toBeVisible();
});

for (const url of APP_URLS) {
  test(`${url}/api/me is 401 without a session`, SMOKE, async ({ request }) => {
    expect((await request.get(`${url}/api/me`)).status()).toBe(401);
  });
}

test(
  "anonymous visitors see no logout button and are not logged in on /account",
  SMOKE,
  async ({ page }) => {
    await page.goto(CANARY_URL);
    await expect(page.getByRole("button", { name: "Log out" })).toHaveCount(0);
    await page.goto(`${HOMEPAGE_URL}/account`);
    await expect(page.getByText("Not logged in.")).toBeVisible();
  },
);

test("logging out on canary ends the session on every host", async ({
  context,
  page,
}) => {
  await logIn(context, DISPLAY_NAME);

  await page.goto(CANARY_URL);
  await logOut(page);

  await expect(page.getByText("anonymous")).toBeVisible();
  for (const url of APP_URLS) {
    expect((await context.request.get(`${url}/api/me`)).status()).toBe(401);
  }
  await page.goto(HOMEPAGE_URL);
  await expect(page.locator(".session")).toHaveText("anonymous");
});

const logoutFailures = [
  { failure: "a 403", fail: (route: Route) => route.fulfill({ status: 403 }) },
  { failure: "a network error", fail: (route: Route) => route.abort() },
];

for (const { failure, fail } of logoutFailures) {
  test(`a logout that fails with ${failure} keeps the page and shows an error`, async ({
    context,
    page,
  }) => {
    await logIn(context, DISPLAY_NAME);
    await page.route(`${CANARY_URL}/api/auth/logout`, fail);

    await page.goto(CANARY_URL);
    await logOut(page);

    await expect(page.getByRole("alert")).toBeVisible();
    await expect(page.getByText(DISPLAY_NAME)).toBeVisible();
  });
}

async function logOut(page: Page): Promise<void> {
  // The island hydrates on idle; a click before then does nothing.
  await page.locator("astro-island:not([ssr])").waitFor({ state: "attached" });
  await page.getByRole("button", { name: "Log out" }).click();
}
