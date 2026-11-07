// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  devtools: { enabled: process.env.NODE_ENV === "development" },
  app: {
    head: {
      title: "VortiDeck",
    },
  },
  extends: [["../../layers/desktop", { install: true }]],
  future: {
    compatibilityVersion: 4,
  },
  experimental: {
    watcher: "parcel",
  },
  ssr: false,



  build: {
    transpile: ["@tauri-apps/api"],
  },




  i18n: {
    strategy: "no_prefix",
  },

  // ogImage: { enabled: false },

  //!tauri
  // build: {
  //   distDir: "../.output/public",
  // },
  devServer: {
    host: "localhost",
    port: 4450,
  },

  hooks: {
    "nitro:config": function (nitroConfig) {
      const publicConfig = nitroConfig.runtimeConfig?.public;
      const storyblok = publicConfig?.storyblok;
      if (storyblok && typeof storyblok === "object") {
        // Storyblok needs this path while generating components, but its
        // absolute build-machine value has no purpose in the packaged client.
        storyblok.componentsDir = "";
      }
    },
    "vite:extend": function ({ config }) {
      if (config.server && config.server.hmr && config.server.hmr !== true) {
        config.server.hmr.protocol = "ws";
        // config.server.hmr.host = "0.0.0.0";
        config.server.hmr.port = 4450;
        config.server.strictPort = true;
      }
    },
  },

  vite: {
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
    server: {
      strictPort: true,
      watch: {
        // Rust build output can contain tens of thousands of files. Watching it
        // stalls the Windows dev server before it can answer its first request.
        ignored: [
          /(^|[\\/])target([\\/]|$)/,
          /(^|[\\/])\.titor([\\/]|$)/,
        ],
      },
    },
    optimizeDeps: {
      include: [
        '@gtm-support/vue-gtm',
        '@tauri-apps/plugin-fs',
        '@tauri-apps/plugin-http',
        '@tauri-apps/plugin-process',
        '@tauri-apps/plugin-shell',
        '@tauri-apps/plugin-updater',
        '@tauri-store/pinia',
        'buffer',
        'device-detector-js',
        'posthog-js/dist/module.no-external',
        'tauri-plugin-better-posthog',
        '@storyblok/vue',
        '@vue/devtools-core',
        '@vue/devtools-kit',
        '@tauri-apps/plugin-store',
        'swapy',
        '@tauri-apps/plugin-autostart',
        '@tauri-apps/plugin-dialog',
        '@tauri-apps/plugin-log',
        '@tauri-apps/plugin-notification',
        '@tauri-apps/plugin-opener',
        '@tauri-apps/plugin-os',
        'vue-multiselect',
      ],
    },
  },

  // !tauri

  // A packaged desktop client has no private Nitro runtime. Keep only values
  // that are intentionally exposed to its browser context and actively used.
  runtimeConfig: {
    public: {
      API_URL: process.env.API_URL,
      WS_URL: process.env.WS_URL,
    },
  },
});
// Trigger Nuxt config reload for studio stabilization
