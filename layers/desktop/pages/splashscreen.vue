<template lang="pug">
div(class="splash-page")
    WindowsTitlebar
    div(id="main-app" class="splash-page__content")
        Splashscreen(:tasks="tasks")
</template>

<script setup lang="ts">
import { useDesktopUtils } from "@desktop/stores/desktopUtils";

const desktopUtils = useDesktopUtils();

const tasks = computed(() => {
  return desktopUtils.getSplashscreenTasks;
});

definePageMeta({
  layout: "clean",
});
</script>

<style lang="scss" scoped>
.splash-page {
  position: relative;
  min-height: 100vh;
}

.splash-page__content {
  min-height: 100vh;
  width: 100%;
}

.splash-page :deep(.titlebar) {
  background: transparent !important;
  border: 0;
  box-shadow: none;
  padding-left: 0.5rem !important;
}

.splash-page :deep(.titlebar > p),
.splash-page :deep(.titlebar #titlebar-title) {
  display: none;
}

/* Hide AI Assistant, Settings, Theme Toggle, and Pin buttons permanently on splashscreen */
.splash-page :deep(.titlebar #titlebar-assistant),
.splash-page :deep(.titlebar #titlebar-settings),
.splash-page :deep(.titlebar #titlebar-theme-toggle),
.splash-page :deep(.titlebar #titlebar-pin-mobile-simulator),
.splash-page :deep(.titlebar #titlebar-pin-streaming) {
  display: none !important;
}

/* Hide minimize, maximize, and close by default, and fade them in on hover */
.splash-page :deep(.titlebar #titlebar-minimize),
.splash-page :deep(.titlebar #titlebar-maximize),
.splash-page :deep(.titlebar #titlebar-close) {
  opacity: 0 !important;
  transition: opacity 0.2s ease-in-out;
}

.splash-page :deep(.titlebar:hover #titlebar-minimize),
.splash-page :deep(.titlebar:hover #titlebar-maximize),
.splash-page :deep(.titlebar:hover #titlebar-close),
.splash-page :deep(.titlebar-shell:hover #titlebar-minimize),
.splash-page :deep(.titlebar-shell:hover #titlebar-maximize),
.splash-page :deep(.titlebar-shell:hover #titlebar-close) {
  opacity: 1 !important;
}

.splash-page :deep(.titlebar .titlebar-button) {
  border-radius: 0.45rem;
}

.splash-page :deep(.titlebar .titlebar-button img) {
  filter: brightness(0.25) !important;
}

.splash-page :deep(.titlebar .titlebar-button:hover) {
  background: rgba(27, 39, 66, 0.12) !important;
}

.splash-page :deep(.titlebar #titlebar-close:hover) {
  background: #e81123 !important;
}

.splash-page :deep(.titlebar #titlebar-close:hover img) {
  filter: invert(1) brightness(1) !important;
}

.splash-page :deep(.titlebar-shell) {
  z-index: 10005 !important;
  pointer-events: auto !important;
}

.splash-page :deep(.titlebar-overlay) {
  display: none !important;
}
</style>
