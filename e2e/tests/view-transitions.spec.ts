import { expect, test, type Page } from "@playwright/test";

import { CANARY_URL, HOMEPAGE_URL } from "./hosts.ts";

const REVEAL_FLAG = "revealedWithTransition";

test.beforeEach(async ({ page }) => {
  await page.addInitScript((flag) => {
    window.addEventListener("pagereveal", (event) => {
      Reflect.set(window, flag, event.viewTransition instanceof ViewTransition);
    });
  }, REVEAL_FLAG);
});

test("navigating within the homepage fires a view transition", async ({
  page,
}) => {
  await page.goto(HOMEPAGE_URL);
  await page.getByRole("link", { name: "Account" }).click();
  await page.waitForURL(`${HOMEPAGE_URL}/account`);

  await expect.poll(() => revealedWithTransition(page)).toBe(true);
});

test("following the link to canary fires none", async ({ page }) => {
  await page.goto(`${HOMEPAGE_URL}/account`);
  await page.getByRole("link", { name: "Canary" }).click();
  await page.waitForURL(`${CANARY_URL}/`);

  await expect.poll(() => revealedWithTransition(page)).toBe(false);
});

test("following canary's link to the homepage fires none", async ({ page }) => {
  await page.goto(CANARY_URL);
  await page.getByRole("link", { name: "vikingthe.dev" }).click();
  await page.waitForURL(`${HOMEPAGE_URL}/`);

  await expect.poll(() => revealedWithTransition(page)).toBe(false);
});

function revealedWithTransition(page: Page): Promise<unknown> {
  return page.evaluate((flag) => Reflect.get(window, flag), REVEAL_FLAG);
}
