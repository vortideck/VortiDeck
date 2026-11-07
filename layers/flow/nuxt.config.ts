import { defineNuxtConfig } from "nuxt/config";
import { createResolver } from "@nuxt/kit";

const { resolve } = createResolver(import.meta.url);


// https://v3.nuxtjs.org/api/configuration/nuxt.config
export default defineNuxtConfig({
  // ssr: true,

  // extends: [["../configuration", { install: true }]],
 


  // i18n: {
  //   vueI18n: "./i18n.config.ts", // if you are using custom path, default
  //   locales: [
  //     { code: "en", language: "en-US" },
  //     { code: "nl", language: "nl-NL" },
  //   ],
  //   defaultLocale: "en",
  // },

  alias: {
    "@": resolve("./"),
    "@flow": resolve("./"),
  },

  // devServer: {
  //   port: 3001,
  // },

  // // content: {
  // //   // https://content.nuxtjs.org/api/configuration
  // // },



  // image: {
  //   domains: [process.env.FRONTEND_URL as string, "localhost:3001"],
  //   format: ["webp"],
  //   provider: "ipx",
  //   ipx: {
  //     dir: "public",
  //   },
  //   storyblok: {
  //     baseURL: "https://a.storyblok.com",
  //   },
  //   twicpics: {
  //     baseURL: "https://freshandtidy.twic.pics/partners/",
  //     // Feel free to use our demo domain to try the following examples.
  //     // baseUrl: 'https://demo.twic.pics/'
  //   },
  // },

  // app: {
  //   head: {
  //     charset: "utf-8",
  //     viewport: "width=device-width, initial-scale=1",
  //     title: "",
  //     meta: [
  //       // <meta name="description" content="My amazing site">
  //       {
  //         hid: `description`,
  //         name: "description",
  //         content: " ",
  //       },
  //     ],
  //     link: [
  //       { rel: "icon", type: "image/x-icon", href: "/images/favicon.ico" },
  //     ],
  //   },
  // },




  // runtimeConfig: {
  //   public: {
  //     WEBSOCKET_SERVER_URL: process.env.WEBSOCKET_SERVER_URL,
  //     API_URL: process.env.API_URL,
  //   },
  // },
});
