import { defineConfig, devices } from "@playwright/test";

// Runs inside the pinned Playwright container on the CI stack's network (step 7 gate).
export default defineConfig({
  testDir: "./tests",
  globalSetup: "./wait-for-stack.ts",
  // Below the writable mount, which Playwright can empty but not delete (compose.ci.yaml).
  outputDir: "test-results/latest",
  forbidOnly: true,
  reporter: "list",
  use: {
    ...devices["Desktop Chrome"],
    // The CI stack's certificate is generated per run and trusted by nothing.
    ignoreHTTPSErrors: true,
  },
  // Chromium only: Firefox has no cross-document view transitions (step 7 gate).
  projects: [{ name: "chromium" }],
});
