import { expect, type BrowserContext } from "@playwright/test";

export const HOMEPAGE_URL = "https://vikingthe.test";
export const CANARY_URL = "https://canary.vikingthe.test";
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
