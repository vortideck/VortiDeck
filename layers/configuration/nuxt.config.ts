import { defineNuxtConfig } from "nuxt/config";
import Icons from "unplugin-icons/vite";
import { createResolver } from "@nuxt/kit";
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'
import { existsSync } from 'node:fs'

const { resolve } = createResolver(import.meta.url);
const currentDir = dirname(fileURLToPath(import.meta.url));
// The blog layer is private (website only); public builds have no Storyblok components.
const blogComponentsDir = resolve("../blog/storyblok");

// https://v3.nuxtjs.org/api/configuration/nuxt.config
export default defineNuxtConfig({
  // ssr: true,

  modules: [
    "@nuxtjs/tailwindcss",
    "@nuxtjs/i18n",
    "@formkit/auto-animate/nuxt",
    "@nuxt/image",
    [
      "@pinia/nuxt",
      {
        autoImports: ["defineStore", "acceptHMRUpdate", "useStore"],
      },
    ],
    "pinia-plugin-persistedstate/nuxt",
    "unplugin-icons/nuxt",
    ["@storyblok/nuxt", {
      accessToken: process.env.STORYBLOK_TOKEN, //preview for published and draft
      componentsDir: existsSync(blogComponentsDir) ? blogComponentsDir : undefined,
      bridge: process.env.NODE_ENV === 'development',
      apiOptions: {
        region: "eu",
      },
      devtools: process.env.NODE_ENV === 'development',
    }],
  ],

  future: {
    compatibilityVersion: 4,
  },

  experimental: {
    viewTransition: true,
    typedPages: true,
    asyncContext: true,
  },

  compatibilityDate: "2025-02-07",
  alias: {
    "@": resolve("./"),
    "@configuration": resolve("./"),
    "@blog": resolve("../blog"),
  },


  build: {
    transpile: ["@headlessui/vue"],
  },

  image: {
    storyblok: {
      baseURL: "https://a.storyblok.com",
    },
  },






  // nitro: {
  //   prerender: {
  //     failOnError: false,
  //     crawlLinks: true,
  //     // routes: ["/"],
  //   },
  // },


  app: {
    head: {
      link: [
        { rel: "icon", type: "image/x-icon", href: "/favicon.ico" },
      ],
    },
  },


  css: [
    join(currentDir, './assets/css/styles.css'),
    join(currentDir, './assets/scss/main.scss')
  ],
  vite: {
    plugins: [
      Icons({
        // the feature below is experimental ⬇️
        autoInstall: true,
      }),
    ],
    css: {
      preprocessorOptions: {
        scss: {
          api: "modern-compiler", // or "modern"
        },
      },
    },
  },




});
