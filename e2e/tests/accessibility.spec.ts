import { AxeBuilder } from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

import { CANARY_URL, HOMEPAGE_URL, logIn } from "./hosts.ts";

const BLOCKING_IMPACTS = new Set(["serious", "critical"]);
const PAGES = [HOMEPAGE_URL, `${HOMEPAGE_URL}/account`, CANARY_URL];

for (const url of PAGES) {
  test(`${url} has no serious or critical axe violations, logged out and in`, async ({
    context,
    page,
  }) => {
    await page.goto(url);
    expect(await blockingViolations(page)).toEqual([]);

    await logIn(context, "Mary Jackson");
    await page.goto(url);
    expect(await blockingViolations(page)).toEqual([]);
  });
}

async function blockingViolations(page: Page): Promise<string[]> {
  const { violations } = await new AxeBuilder({ page }).analyze();
  return violations
    .filter((violation) => BLOCKING_IMPACTS.has(violation.impact ?? ""))
    .map((violation) => violation.id);
}
