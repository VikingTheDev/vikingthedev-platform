import { defineConfig, devices } from "@playwright/test";

import { IS_CI_STACK } from "./tests/hosts.ts";

// Runs inside the pinned Playwright container, on the CI stack's network or against production (step 16).
export default defineConfig({
  testDir: "./tests",
  globalSetup: "./wait-for-stack.ts",
  // Below the writable mount, which Playwright can empty but not delete (compose.ci.yaml).
  outputDir: "test-results/latest",
  forbidOnly: true,
  reporter: "list",
  // Production gets only the tests that need no login and change nothing.
  ...(IS_CI_STACK ? { grepInvert: /@production/ } : { grep: /@smoke/ }),
  use: {
    ...devices["Desktop Chrome"],
    // The CI stack's certificate is generated per run and trusted by nothing.
    ignoreHTTPSErrors: IS_CI_STACK,
  },
  // Chromium only: Firefox has no cross-document view transitions (step 7 gate).
  projects: [{ name: "chromium" }],
});
