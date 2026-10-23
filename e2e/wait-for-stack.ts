import { setTimeout } from "node:timers/promises";
import { request, type APIRequestContext } from "@playwright/test";

import { APP_URLS, IS_CI_STACK } from "./tests/hosts.ts";

const STARTUP_TIMEOUT_MS = 60_000;
const POLL_INTERVAL_MS = 500;

// The distroless images have no shell for a compose healthcheck, so the suite waits for /health itself.
export default async function waitForStack(): Promise<void> {
  const client = await request.newContext({
    ignoreHTTPSErrors: IS_CI_STACK,
  });
  const deadline = Date.now() + STARTUP_TIMEOUT_MS;
  for (const url of APP_URLS.flatMap((app) => [
    `${app}/health`,
    `${app}/api/health`,
  ])) {
    while (!(await isUp(client, url))) {
      if (Date.now() > deadline) {
        throw new Error(`${url} did not come up`);
      }
      await setTimeout(POLL_INTERVAL_MS);
    }
  }
  await client.dispose();
}

async function isUp(client: APIRequestContext, url: string): Promise<boolean> {
  try {
    return (await client.get(url)).ok();
  } catch {
    return false;
  }
}
