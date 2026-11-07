import { createResolver } from "@nuxt/kit";

const { resolve } = createResolver(import.meta.url);

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  devtools: { enabled: process.env.NODE_ENV === "development" },
  extends: [["../base", { install: true }],["../flow", { install: true }],["../desktop-streaming", { install: true }]],
  modules: ['@vueuse/nuxt', "@nuxtjs/mdc", "nuxt-swiper"],
  ssr: false,

  alias: {
    "@desktop": resolve("./"),
  },

  swiper: {
    // Swiper options
    //----------------------
    // prefix: 'Swiper',
    // styleLang: 'css',
    // modules: ['navigation', 'pagination'], // all modules are imported by default
  },



  // routeRules: {
  //   "/": { prerender: true },
  //   "*": { prerender: true },
  //   /* do the same for all routes used */
  // },

  

  runtimeConfig: {
    GOOGLE_LOGIN_CLIENT_ID: process.env.GOOGLE_LOGIN_CLIENT_ID,
    public: {
      GOOGLE_LOGIN_CLIENT_ID: process.env.GOOGLE_LOGIN_CLIENT_ID,
    },
  },


 






});
