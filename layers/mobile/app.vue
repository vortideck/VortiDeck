<template>
  <Html :lang="locale" :class="['select-none bg-themeBackground', themeClass]">
    <NuxtLayout>
      <NuxtLoadingIndicator :height="5" :duration="3000" :throttle="400" />
      <NuxtPage class="" />
    </NuxtLayout>
  </Html>
</template>

<script lang="ts" setup>
import { onMounted } from "vue";
import { useTheme } from "~/composables/useTheme";

AppSetup();
const locale = useState<string>("locale.setting");
const app = useAppConfig();
const { themeClass, initTheme } = useTheme();

const $gtm = useGtm();

onMounted(() => {
  initTheme();
});

// Signal native splash screen only after the first page is fully painted
const nuxtApp = useNuxtApp();
nuxtApp.hook("page:finish", () => {
  if (!(window as any).__APP_READY__) {
    // Double requestAnimationFrame ensures the browser has actually
    // committed pixels to the screen, not just updated the DOM
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        (window as any).__APP_READY__ = true;
      });
    });
  }
});
</script>

<style lang="scss">
.page-enter-active,
.page-leave-active {
  transition: all 0.4s;
}
.page-enter-from,
.page-leave-to {
  opacity: 0;
  filter: blur(1rem);
}
</style>
