// const { createThemes } = require("tw-colors");

module.exports = {
  darkMode: "class", // or 'media' or 'false'
  content: [
    // Configuration layer

    "./components/**/*.{js,vue,ts}",
    "./layouts/**/*.vue",
    "./pages/**/*.vue",
    "./storyblok/**/*.vue",
    "./plugins/**/*.{js,ts}",
    "./nuxt.config.{js,ts}",
    "./app.vue",
    // Other layers
    "../*/components/**/*.{js,vue,ts}",
    "../*/layouts/**/*.vue",
    "../*/pages/**/*.vue",
    "../*/storyblok/**/*.vue",
    "../*/storyblok/*.vue",
    "../*/plugins/**/*.{js,ts}",
    "../*/app.vue",
    // Apps
    "../../apps/*/components/**/*.{js,vue,ts}",
    "../../apps/*/layouts/**/*.vue",
    "../../apps/*/pages/**/*.vue",
    "../../apps/*/plugins/**/*.{js,ts}",
    "../../apps/*/app.vue",
    "../../apps/*/storyblok/**/*.vue",
    "../../apps/*/storyblok/*.vue",
    // Layers with direct storyblok files
    "../../layers/*/storyblok/*.vue",
    "../../layers/**.{js,vue,ts}",
  ],
  variants: {
    extend: {
      display: ["group-hover"],
    },
  },


  theme: {
    extend: {
      colors: {
        themeBackground: "hsl(var(--background) / <alpha-value>)",
        themeBackground2: "hsl(var(--background2) / <alpha-value>)",
        themeBackground3: "hsl(var(--background3) / <alpha-value>)",
        themeText: "hsl(var(--text) / <alpha-value>)",
        themeText2: "hsl(var(--text2) / <alpha-value>)",
        themeText3: "hsl(var(--text3) / <alpha-value>)",
        themePrimary: "hsl(var(--primary) / <alpha-value>)",
        themeSecondary: "hsl(var(--secondary) / <alpha-value>)",
        themeHint: "hsl(var(--hint) / <alpha-value>)",
        themeBorder: "hsl(var(--border) / <alpha-value>)",
        themeBorder2: "hsl(var(--border2) / <alpha-value>)",
        themeHover: "hsl(var(--hover) / <alpha-value>)",
        themeActive: "hsl(var(--active) / <alpha-value>)",
        themeInvert: "hsl(var(--invert) / <alpha-value>)",
        themeButton: "hsl(var(--button) / <alpha-value>)",

        themeConnected: "hsl(var(--connected) / <alpha-value>)",
        themeConnectedBg: "hsl(var(--connectedBg) / <alpha-value>)",
        themeDisconnected: "hsl(var(--disconnected) / <alpha-value>)",
        themeDisconnectedBg: "hsl(var(--disconnectedBg) / <alpha-value>)",
        themeConnecting: "hsl(var(--connecting) / <alpha-value>)",
        themeConnectingBg: "hsl(var(--connectingBg) / <alpha-value>)",
        themePaired: "hsl(var(--paired) / <alpha-value>)",
        themePairedBg: "hsl(var(--pairedBg) / <alpha-value>)",

        // Status colors
        themeSuccess: "hsl(var(--success) / <alpha-value>)",
        themeSuccessBg: "hsl(var(--successBg) / <alpha-value>)",
        themeError: "hsl(var(--error) / <alpha-value>)",
        themeErrorBg: "hsl(var(--errorBg) / <alpha-value>)",
        themeInfo: "hsl(var(--info) / <alpha-value>)",
        themeInfoBg: "hsl(var(--infoBg) / <alpha-value>)",

        // Warning/Banner colors
        themeWarningBg: "hsl(var(--warning-bg) / <alpha-value>)",
        themeWarningBgSubtle: "hsl(var(--warning-bg-subtle) / <alpha-value>)",
        themeWarningText: "hsl(var(--warning-text) / <alpha-value>)",
        themeWarningTextSubtle: "hsl(var(--warning-text-subtle) / <alpha-value>)",
        themeWarningBorder: "hsl(var(--warning-border) / <alpha-value>)",
        themeWarningIcon: "hsl(var(--warning-icon) / <alpha-value>)",
      },
      keyframes: {
        showUp: {
          "0%": { transform: "translateY(50px)" },
          "100%": { transform: "translateY(0px)" },
        },
        hideDown: {
          "0%": { transform: "translateY(0px)" },
          "100%": { transform: "translateY(50px)" },
        },
        hideDownNavbar: {
          "0%": { transform: "translateY(0rem)" },
          "100%": { transform: "translateY(25rem)" },
        },
        spin: {
          "0%": { transform: "rotate(0deg)" },
          "100%": { transform: "rotate(360deg)" },
        },
        spinBack: {
          "0%": { transform: "rotate(180deg)" },
          "100%": { transform: "rotate(0deg)" },
        },
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
        fadeOut: {
          "0%": { opacity: "1" },
          "100%": { opacity: "0" },
        },
        shimmer: {
          "0%": { opacity: "0.5" },
          "25%": { opacity: "0.75" },
          "50%": { opacity: "1" },
          "75%": { opacity: "0.75" },
          "100%": { opacity: "0.5" },
        },
        "orb-drift-1": {
          "0%, 100%": { transform: "translate(0, 0) scale(1)" },
          "25%": { transform: "translate(30px, -20px) scale(1.05)" },
          "50%": { transform: "translate(-20px, 15px) scale(0.95)" },
          "75%": { transform: "translate(15px, 25px) scale(1.02)" },
        },
        "orb-drift-2": {
          "0%, 100%": { transform: "translate(0, 0) scale(1)" },
          "33%": { transform: "translate(-25px, 20px) scale(1.03)" },
          "66%": { transform: "translate(20px, -30px) scale(0.97)" },
        },
        "orb-drift-3": {
          "0%, 100%": { transform: "translate(0, 0) scale(1)" },
          "20%": { transform: "translate(15px, 30px) scale(0.98)" },
          "40%": { transform: "translate(-30px, -10px) scale(1.04)" },
          "60%": { transform: "translate(25px, -20px) scale(1.01)" },
          "80%": { transform: "translate(-10px, 15px) scale(0.96)" },
        },
        "grid-highlight": {
          "0%": { transform: "translateX(-100%) skewX(-15deg)" },
          "100%": { transform: "translateX(200%) skewX(-15deg)" },
        },
        "pulse-soft": {
          "0%, 100%": { opacity: "0.4" },
          "50%": { opacity: "1" },
        },
      },
      animation: {
        showUp: "showUp 0.2s linear",
        hideDownNavbar: "hideDownNavbar 0.4s linear",
        showUpNavbar: "hideDownNavbar 0.4s linear",
        spinOnce: "spin 1s forwards",
        spinOnceBack: "spinBack 1s forwards",
        shimmer: "shimmer 2s infinite linear",
        "orb-drift-1": "orb-drift-1 25s ease-in-out infinite",
        "orb-drift-2": "orb-drift-2 30s ease-in-out infinite",
        "orb-drift-3": "orb-drift-3 28s ease-in-out infinite",
        "grid-highlight": "grid-highlight 8s ease-in-out infinite",
        "pulse-soft": "pulse-soft 3s ease-in-out infinite",
      },
    },
    fontFamily: {
      sans: ["Inter", "system-ui", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Roboto", "sans-serif"],
      mono: ["ui-monospace", "SFMono-Regular", "Menlo", "Monaco", "Consolas", "Liberation Mono", "Courier New", "monospace"],
      brown: ["BROWN", "sans-serif"],
      itim: ["ITIM", "sans-serif"],
      intern: ["INTERN", "sans-serif"],
      roboto: ["ROBOTO", "sans-serif"],
      robotolight: ["ROBOTOLIGHT", "sans-serif"],
      montserrat: ["MONTSERRAT", "sans-serif"],
      elmessiri: ["ELMESSIRI", "sans-serif"],
      publicsans: ["PublicSans", "sans-serif"],
      righteous: ["RIGHTEOUS", "regular"],
      flamabook: ["FLAMABOOK", "regular"],
      mulish: ["Mulish", "regular"],
    },
    screens: {
      xs: "320px",
      // => @media (min-width: 320px) { ... }
      sm: "640px",
      // => @media (min-width: 640px) { ... }


      md: "768px",
      // => @media (min-width: 768px) { ... }

      lg: "1024px",
      // => @media (min-width: 1024px) { ... }

      xl: "1330px",
      // => @media (min-width: 1330px) { ... }

      "2xl": "1536px",
      // => @media (min-width: 1536px) { ... }
      "3xl": "1920px",
      "4xl": "2560px",
    },
  },
  plugins: [
    // createThemes({
    //   "default-light": {},
    //   "default-dark": {},
    // }),
  ],
};
