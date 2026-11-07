<template lang="pug">
div(class="select-none bg-themeBackground text-themeText")
    WindowsTitlebar(class="z-[10001]")

    div(id="streaming-app" class="h-[calc(100%-30px)] flex flex-row bg-themeBackground justify-center items-start w-full mt-[30px] text-themeText")

        div(class="scrollbar-visible   w-full")
          div(class="flex overflow-x-hidden flex-col w-full min-h-[calc(100vh-30px)] h-[calc(100vh-30px)] ")
            Transition(name="fade")
              <div v-show="desktopUtils.getIsSidebar" @click="desktopUtils.toggleSidebar()" tabindex="-1" class="fixed 2xl:hidden top-0 pointer-event-none flex items-center bg-[rgba(0,0,0,0.55)] justify-center left-0 right-0 z-50 w-full p-4 overflow-x-hidden overflow-y-auto md:inset-0 h-screen md:h-full"></div>
            <slot class="" />

    div(class="cursor cursor--small")
</template>

<script setup lang="ts">
import { useDesktopUtils } from "../../desktop/stores/desktopUtils";
import { getCurrentWindow } from "@tauri-apps/api/window";

const desktopUtils = useDesktopUtils();

const { initScale } = useAppScale();

onMounted(async () => {
  initScale();
  await getCurrentWindow().show();
});
</script>

<style lang="scss" scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.5s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.cursor {
  position: fixed;
  left: 0;
  top: 0;
  pointer-events: none;
}

.cursor--small {
  width: 5px;
  height: 5px;
  left: -2.5px;
  top: -2.5px;
  border-radius: 50%;
  z-index: 11000;
  background: var(--color-text);
}
</style>
