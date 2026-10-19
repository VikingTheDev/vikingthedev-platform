import { expect, test, type APIResponse } from "@playwright/test";

import { CANARY_URL, HOMEPAGE_URL, logIn } from "./hosts.ts";

const DISPLAY_NAME = "Katherine Johnson";
const NOINDEX = '<meta name="robots" content="noindex">';
const SERVER_ISLAND_PATH = /\/_server-islands\/[^"]+/;

test("the cacheable homepage holds no user data, cookie or unhashed script", async ({
  context,
}) => {
  await logIn(context, DISPLAY_NAME);

  const response = await context.request.get(HOMEPAGE_URL);
  const html = await response.text();

  expect(response.headers()["set-cookie"]).toBeUndefined();
  expect(html).not.toContain(DISPLAY_NAME);
  expect(html).toContain(NOINDEX);
  expect(html).toMatch(
    /http-equiv="content-security-policy" content="[^"]*script-src[^"]*sha256-/,
  );
  expect(html).not.toContain("unsafe-inline");
});

test("the server island is private and sets no cookie", async ({ context }) => {
  await logIn(context, DISPLAY_NAME);
  const homepage = await (await context.request.get(HOMEPAGE_URL)).text();
  const islandPath = homepage
    .match(SERVER_ISLAND_PATH)?.[0]
    .replaceAll("&amp;", "&");
  expect(islandPath).toBeDefined();

  const response = await context.request.get(`${HOMEPAGE_URL}${islandPath}`);

  expectPrivate(response);
  expect(await response.text()).toContain(DISPLAY_NAME);
});

for (const url of [`${HOMEPAGE_URL}/account`, CANARY_URL]) {
  test(`${url} is private, hashed and noindex`, async ({ context }) => {
    await logIn(context, DISPLAY_NAME);

    const response = await context.request.get(url);

    expectPrivate(response);
    const policy = response.headers()["content-security-policy"] ?? "";
    expect(policy).toMatch(/script-src[^;]*sha256-/);
    expect(policy).not.toContain("unsafe-inline");
    expect(await response.text()).toContain(NOINDEX);
  });
}

function expectPrivate(response: APIResponse): void {
  expect(response.status()).toBe(200);
  expect(response.headers()["cache-control"]).toBe("private, no-store");
  expect(response.headers()["set-cookie"]).toBeUndefined();
}
