import { expect, type BrowserContext } from "@playwright/test";

export const CI_STACK_DOMAIN = "vikingthe.test";
// The post-deploy smoke run sets it to vikingthe.dev.
export const PLATFORM_DOMAIN =
  process.env["PLATFORM_DOMAIN"] ?? CI_STACK_DOMAIN;
export const IS_CI_STACK = PLATFORM_DOMAIN === CI_STACK_DOMAIN;
export const HOMEPAGE_URL = `https://${PLATFORM_DOMAIN}`;
export const CANARY_URL = `https://canary.${PLATFORM_DOMAIN}`;
export const APP_URLS = [HOMEPAGE_URL, CANARY_URL];

/** Starts a session for a new user through the e2e-only test login; the context then holds the cookie. */
export async function logIn(
  context: BrowserContext,
  displayName: string,
): Promise<string> {
  const response = await context.request.post(
    `${HOMEPAGE_URL}/api/test/login`,
    {
      headers: { origin: HOMEPAGE_URL },
      data: { display_name: displayName },
    },
  );
  expect(response.status()).toBe(204);
  return response.headers()["set-cookie"] ?? "";
}
