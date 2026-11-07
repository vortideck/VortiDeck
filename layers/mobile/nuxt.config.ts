import { createResolver } from "@nuxt/kit";

const { resolve } = createResolver(import.meta.url);

export default defineNuxtConfig({
  devtools: { enabled: true },
  extends: [["../base", { install: true }]],
  ssr: false,
  alias: {
    "@mobile": resolve("./"),
  },
  routeRules: {
    "/": { prerender: true },
    "*": { prerender: true },
    /* do the same for all routes used */
  },

  app: {
    head: {
      script: [
        {
          // Runs before any framework JS — prevents white flash
          innerHTML: `(function(){try{var t=localStorage.getItem('theme');if(!t||'system dark light'.indexOf(t)===-1)t='dark';var r=t;if(t==='system'){r=window.matchMedia&&window.matchMedia('(prefers-color-scheme:dark)').matches?'dark':'light'}var c=r==='dark'?'theme-mobile-dark':'theme-mobile-light';document.documentElement.classList.add(c,'select-none','bg-themeBackground')}catch(e){}})()`,
          tagPosition: 'head',
        },
      ],
    },
  },

  ogImage: { enabled: false },

  // Runtime config for OAuth and backend relay
  runtimeConfig: {
  
  },
});
