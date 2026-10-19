import { defineConfig, globalIgnores } from "eslint/config";
import astro from "eslint-plugin-astro";
import svelte from "eslint-plugin-svelte";
import tseslint from "typescript-eslint";

export default defineConfig(
  globalIgnores([
    "**/dist/",
    "**/.astro/",
    "target/",
    "packages/api-types/src/schema.d.ts",
  ]),
  tseslint.configs.strict,
  svelte.configs.recommended,
  astro.configs.recommended,
  {
    files: ["**/*.svelte", "**/*.svelte.ts"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: [".svelte"],
      },
    },
  },
);
