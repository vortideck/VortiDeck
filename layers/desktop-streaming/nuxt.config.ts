import { defineNuxtConfig } from "nuxt/config";
import { createResolver } from "@nuxt/kit";

const { resolve } = createResolver(import.meta.url);

// https://v3.nuxtjs.org/api/configuration/nuxt.config
export default defineNuxtConfig({
  alias: {
    "@": resolve("./"),
    "@desktopStreaming": resolve("./"),
  },

  imports: {
    dirs: [
      'composables/**', // Auto-import all composables including subdirectories
    ],
  },

 
})
