<template lang="pug">
  .titlebar-shell(:class="$attrs.class")
    .titlebar(class="relative w-full h-[30px] pl-4 pr-1 bg-themeBackground" data-tauri-drag-region)
      p(
        class="titlebar-beta text-sm my-auto font-intern italic text-themePrimary font-semibold"
        data-tauri-drag-region
      )
      .titlebar-title#titlebar-title(data-tauri-drag-region)
      .titlebar-drag-fill(data-tauri-drag-region)

      .titlebar-controls.titlebar-no-drag.flex.items-center.gap-1.pr-1
        // Dock Mobile Simulator to main window
        button#titlebar-pin-mobile-simulator(
          v-if="isMobileSimulator"
          @click="toggleDockSimulator()"
          :class="isSimulatorDocked ? 'bg-blue-600 text-white shadow-sm ring-1 ring-blue-400' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2'"
          class="p-1 rounded transition-all cursor-pointer flex items-center justify-center"
          :title="isSimulatorDocked ? 'Unpin from Main Window (Currently Docked)' : 'Dock to Main Window (Companion Mode)'"
        )
          svg(class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor")
            path(d="M4 4h16a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2m11 2v12h5V6h-5z")

        // Always on Top (Mobile Simulator & Streaming)
        button#titlebar-pin-always-on-top(
          v-if="supportsAlwaysOnTop"
          @click="toggleAlwaysOnTop()"
          :class="isAlwaysOnTop ? 'bg-amber-500 text-white shadow-sm ring-1 ring-amber-300' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2'"
          class="p-1 rounded transition-all cursor-pointer flex items-center justify-center"
          :title="isAlwaysOnTop ? 'Always on Top: Active (Floating over all windows)' : 'Always on Top: Inactive (Click to float over all windows)'"
        )
          svg(class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor")
            path(d="M16 12V4h1V2H7v2h1v8l-2 2v2h5v6l1 1l1-1v-6h5v-2l-2-2z")

        // AI Assistant button
        button#titlebar-assistant(
          @click="openAssistant()"
          class="p-1 rounded text-themeText2 hover:text-themePrimary hover:bg-themePrimary/10 transition-colors cursor-pointer flex items-center justify-center"
          title="Studio Copilot (AI Assistant)"
        )
          svg(class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor")
            path(d="M12 2a2 2 0 0 1 2 2c0 .74-.4 1.38-1 1.72V7h4a3 3 0 0 1 3 3v8a3 3 0 0 1-3 3H7a3 3 0 0 1-3-3v-8a3 3 0 0 1 3-3h4V5.72c-.6-.34-1-.98-1-1.72a2 2 0 0 1 2-2M7.5 13a1.5 1.5 0 1 0 0 3a1.5 1.5 0 0 0 0-3m9 0a1.5 1.5 0 1 0 0 3a1.5 1.5 0 0 0 0-3")

        // Settings button (hidden on mobile simulator)
        button#titlebar-settings(
          v-if="!isMobileSimulator"
          @click="openSettings()"
          class="p-1 rounded text-themeText2 hover:text-themeText hover:bg-themeBackground2 transition-colors cursor-pointer flex items-center justify-center"
          title="Settings (Ctrl+,)"
        )
          svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z")

        // Theme toggle
        button#titlebar-theme-toggle(
          @click="mainStore.toggleTheme()"
          class="p-1 rounded text-themeText2 hover:text-themeText hover:bg-themeBackground2 transition-colors cursor-pointer flex items-center justify-center"
          :title="isDark ? 'Switch to Light Mode' : 'Switch to Dark Mode'"
        )
          svg(v-if="isDark" class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z")
          svg(v-else class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z")

        // Minimize
        button#titlebar-minimize(
          v-if="showMinimize"
          @click="appWindow.minimize()"
          class="p-1 rounded text-themeText2 hover:text-themeText hover:bg-themeBackground2 transition-colors cursor-pointer flex items-center justify-center"
          title="Minimize"
        )
          svg(class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor")
            path(d="M20 14H4v2h16v-2z")

        // Maximize
        button#titlebar-maximize(
          v-if="showMaximize"
          @click="appWindow.toggleMaximize()"
          class="p-1 rounded text-themeText2 hover:text-themeText hover:bg-themeBackground2 transition-colors cursor-pointer flex items-center justify-center"
          title="Maximize"
        )
          svg(class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor")
            rect(x="4" y="4" width="16" height="16" rx="2" stroke-width="2")

        // Close
        button#titlebar-close(
          @click="closeWindow"
          class="p-1 rounded text-themeText2 hover:text-white hover:bg-red-500 transition-colors cursor-pointer flex items-center justify-center"
          title="Close"
        )
          svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

  // Transparent top drag overlay over sidebar and navbar
  .titlebar-overlay(
    class="fixed top-0 left-0 w-[calc(100vw-220px)] h-[30px] z-[10000] pointer-events-auto bg-transparent"
    data-tauri-drag-region
  )
</template>

<script setup lang="ts">
import { emitTo } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useMainStore } from "@/stores/Main";
import { useSettingsModal } from "../composables/useSettingsModal";

const mainStore = useMainStore();
const { openSettings } = useSettingsModal();
defineOptions({ inheritAttrs: false });

const appWindow = getCurrentWindow();
const currentLabel = appWindow.label;
const isMobileSimulator = currentLabel === "mobile_simulator";
const supportsAlwaysOnTop = ["mobile_simulator", "streaming"].includes(currentLabel);
const showMinimize = !isMobileSimulator;
const showMaximize = !isMobileSimulator;

const isDark = computed(() => {
  const mode = mainStore.getColorMode;
  if (mode === "system") {
    return typeof window !== "undefined" && window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  }
  return mode === "dark";
});

const isSimulatorDocked = ref(false);
const isAlwaysOnTop = ref(false);

// Initialize window states on mount
onMounted(async () => {
  if (isMobileSimulator) {
    try {
      isSimulatorDocked.value = await invoke("get_mobile_simulator_pinned");
      isAlwaysOnTop.value = await invoke("get_simulator_always_on_top");
    } catch (_) {}
  } else if (currentLabel === "streaming") {
    try {
      isAlwaysOnTop.value = await invoke("get_streaming_window_always_on_top");
    } catch (_) {}
  }
});

const toggleDockSimulator = async () => {
  try {
    await invoke("toggle_simulator_pin");
    isSimulatorDocked.value = await invoke("get_mobile_simulator_pinned");
  } catch (error) {
    console.error("Failed to toggle simulator dock:", error);
  }
};

const toggleAlwaysOnTop = async () => {
  try {
    if (isMobileSimulator) {
      isAlwaysOnTop.value = await invoke<boolean>("toggle_simulator_always_on_top");
    } else if (currentLabel === "streaming") {
      isAlwaysOnTop.value = await invoke<boolean>("toggle_streaming_window_always_on_top");
    }
  } catch (error) {
    console.error("Failed to toggle always on top:", error);
  }
};

const openAssistant = async () => {
  try {
    await invoke("open_assistant_window", {
      context: isMobileSimulator ? "mobile_simulator" : "",
      contextInfo: isMobileSimulator ? "Mobile Simulator" : "",
      currentValue: "",
      formData: {},
    });
  } catch (error) {
    console.error("Failed to open AI Assistant:", error);
  }
};

const closeWindow = async () => {
  if (isMobileSimulator) {
    try {
      await appWindow.hide();
      await new Promise((resolve) => setTimeout(resolve, 150));
      await emitTo("main", "toggle_is_simulator", false);
      await appWindow.close();
    } catch (_) {
      await appWindow.close();
    }
  } else {
    await appWindow.close();
  }
};
</script>

<style lang="scss">
// Windows Titlebar configuration
.titlebar-shell {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 30px;
  z-index: 30;
  pointer-events: none;
}

.titlebar-shell--flow {
  z-index: 30;
}

.titlebar {
  height: 30px;
  user-select: none;
  display: flex;
  align-items: center;
  pointer-events: auto;
  position: relative;
  width: 100%;
  background: hsl(var(--background));
  z-index: 1;
}

.titlebar-drag-fill {
  flex: 1 1 auto;
  min-width: 0.5rem;
  height: 100%;
}

.titlebar-controls {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
}

.titlebar-no-drag,
.titlebar-no-drag * {
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.titlebar-button {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  width: 30px;
  height: 30px;
  user-select: none;
  -webkit-user-select: none;
  border-radius: 4px;
  opacity: 0.85;
  transition: background-color 0.15s ease, opacity 0.15s ease;

  img {
    color: currentColor;
  }

  svg {
    color: var(--theme-text, #1f2328);
    filter: none !important;
  }

  &:hover {
    opacity: 1;
    background: rgba(125, 125, 125, 0.18);
  }
}

.dark .titlebar-button {
  img {
    filter: invert(1);
  }
  svg {
    color: var(--theme-text, #f0f6fc);
    filter: none !important;
  }
}

#titlebar-close:hover {
  background: #e81123 !important;
  opacity: 1;
  img, svg {
    filter: invert(1) !important;
  }
}

.titlebar-overlay {
  position: fixed;
  top: 0;
  left: 0;
  height: 30px;
  background: transparent;
  pointer-events: auto;
  z-index: 10000;
}
</style>
