import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import Icons from "unplugin-icons/vite";

export default defineConfig({
  plugins: [vue(), Icons()],
  resolve: {
    alias: {
      "@base": new URL("../../layers/base", import.meta.url).pathname,
      "@desktop": new URL("../../layers/desktop", import.meta.url).pathname,
    },
  },
  test: {
    environment: "jsdom",
    include: ["../../layers/**/*.spec.ts"],
    clearMocks: true,
  },
});
