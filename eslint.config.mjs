import { createConfigForNuxt } from "@nuxt/eslint-config";

export default createConfigForNuxt({
  features: {
    stylistic: false,
  },
}).prepend({
  ignores: [
    "**/.nuxt/**",
    "**/.output/**",
    "**/dist/**",
    "**/node_modules/**",
    "**/target/**",
    "docs/archive/**",
    "packages/plugin-sdk/src/pdk.generated.ts",
    "packages/plugin-sdk/src/api.ts",
    "examples/plugin-new-pdk/src/pdk.generated.ts",
  ],
}).append({
  files: ["**/*.vue"],
  rules: {
    // Most components use Pug templates. The current Vue ESLint parser does
    // not mark Pug template references as script-setup usage, producing
    // thousands of false unused-variable reports.
    "@typescript-eslint/no-unused-vars": "off",
    "vue/attribute-hyphenation": "off",
    "vue/attributes-order": "off",
    "vue/first-attribute-linebreak": "off",
    "vue/html-self-closing": "off",
    "vue/multi-word-component-names": "off",
  },
});
