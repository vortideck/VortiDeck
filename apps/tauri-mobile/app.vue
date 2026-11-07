<template>
  <Html :lang="locale" :class="['select-none', mainStore.getTheme === 'dark' ? 'theme-default-dark' : 'theme-default-light']">
    <NuxtLayout>
      <NuxtLoadingIndicator :height="5" :duration="3000" :throttle="400" />
      <NuxtPage class="" />
      <Notifications :notifications="mainStore.getOpenNotificationsForWindow('mobile')" />
    </NuxtLayout>
  </Html>
</template>

<script lang="ts" setup>
import { keepScreenOn } from "tauri-plugin-keep-screen-on-api";
import { useMainStore } from "@/stores/Main";
import { listen } from "@tauri-apps/api/event";
import Notifications from "@base/components/Notifications/index.vue";

AppSetup();
const locale = useState<string>("locale.setting");
const app = useAppConfig();
import { useConnectionStore } from "@/stores/Connection";

const mainStore = useMainStore();
const connectionStore = useConnectionStore();
const $gtm = useGtm();

// Listen for theme changes from other windows
listen("theme-changed", (event) => {
  const { theme } = event.payload as { theme: string };

  // Disable transitions temporarily for instant theme switching
  const html = document.documentElement;
  html.classList.add('disable-transitions');

  // Update theme in this window's store
  mainStore.theme = theme;

  // Re-enable transitions after sufficient time for theme to fully apply
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      html.classList.remove('disable-transitions');
    });
  });
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
