import node from "@astrojs/node";
import svelte from "@astrojs/svelte";
import { defineConfig, envField } from "astro/config";

export default defineConfig({
  site: "https://vikingthe.dev",
  adapter: node({ mode: "standalone" }),
  integrations: [svelte()],
  // Rust alone creates sessions (ADR 0001); @astrojs/node would otherwise enable Astro's own.
  session: false,
  env: {
    schema: {
      // Baked in at build time from the image's build argument, like the API's.
      GIT_SHA: envField.string({
        context: "server",
        access: "public",
        default: "dev",
      }),
    },
    validateSecrets: true,
  },
  security: {
    checkOrigin: true,
    // Read at build time; the reserved .test host lets the CI stack run the image that ships.
    allowedDomains: [
      { hostname: "vikingthe.dev", protocol: "https" },
      { hostname: "vikingthe.test", protocol: "https" },
    ],
  },
});
