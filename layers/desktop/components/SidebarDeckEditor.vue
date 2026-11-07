<template lang="pug">


  div#deck-editor-sidebar(class="deck-rail z-[85] fixed w-[5.2rem] top-[30px] left-0 h-[calc(100vh-30px)] text-themeText flex flex-col" ) 

    div#sidebar-content(class="flex-1 overflow-y-auto gap-y-1 relative w-full flex flex-col items-center px-2 pt-2.5 pb-3")
      button(type="button" @click="handleBack" class="rail-back-btn mb-4")
        SolarArrowLeftLinear(class="w-4.5 h-4.5")

      ButtonDeckEditorSidebar(
        v-for="(button, index) in sidebarButtons"
        :key="button.to"
        :to="button.to"
        :text="button.text"
        :notifications_count="button.notifications_count"
        draggable="false"
       
      )
          template(v-slot:icon)
              component(:is="button.icon" class="h-full")

      ButtonDeckEditorSidebar(
        @click="openFlowEditor"
        text="Flows"
        draggable="false"
       
      )
          template(v-slot:icon)
              component(:is="HugeiconsAiBrain02" class="h-full")

       
          
    
   
  </template>

<script setup lang="ts">
import HugeiconsAiBrain02 from "~icons/hugeicons/ai-brain-02";
import SolarArrowLeftLinear from "~icons/solar/arrow-left-linear?width=24px&height=24px";
import DashboardIcon from "~icons/ant-design/home-outlined";
import MdiScriptOutline from "~icons/mdi/script-outline";
import PepiconsPencilKeyboard from "~icons/pepicons-pencil/keyboard?width=27px&height=27px";
import MingcutePluginLine from "~icons/mingcute/plugin-line";
import StreamlineEmojisMoneyBag from "~icons/streamline-emojis/money-bag";
import HealthiconsMoneyBagOutline from "~icons/healthicons/money-bag-outline";

import HugeiconsPaintBoard from "~icons/hugeicons/paint-board?width=24px&height=24px";
import PhMouseLeftClick from "~icons/ph/mouse-left-click?width=256px&height=256px";
import MdiMarketplaceOutline from "~icons/mdi/marketplace-outline?width=24px&height=24px";

import { invoke } from "@tauri-apps/api/core";

import { useDesktopUtils } from "@desktop/stores/desktopUtils";

const desktopUtils = useDesktopUtils();
const router = useRouter();

const is_sidebar_open = computed(() => {
  return desktopUtils.getIsSidebar;
});

const openFlowEditor = async () => {
  try {
    await invoke("open_flow_editor", { flowId: null });
  } catch {
    return;
  }
};

const handleBack = () => {
  if (typeof window !== "undefined" && window.history.length > 1) {
    router.back();
    return;
  }
  router.push("/");
};

const sidebarButtons = ref([
  { text: "Buttons", to: "/deck", icon: PhMouseLeftClick },
  { text: "Themes", to: "/deck/store/themes", icon: HugeiconsPaintBoard },
  { text: "Plugins", to: "/deck/store/plugins", icon: MingcutePluginLine },
  // { text: "Marketplace", to: "/store/marketplace", icon: MdiMarketplaceOutline },
  // { text: "Scripts", to: "/deck/store/scripts", icon: MdiScriptOutline },
  // { text: "Ready Decks", to: "/deck/store/ready-decks", icon: PepiconsPencilKeyboard },
  // { text: "Plugins", to: "/deck/store/plugins", icon: MingcutePluginLine },
  // { text: "Earn", to: "/deck/store/earn", icon: StreamlineEmojisMoneyBag },
]);
</script>

<style lang="scss" scoped>
.deck-rail {
  background: hsl(var(--background));
  border-right: 1px solid hsl(var(--border2) / 0.72);
}

.rail-back-btn {
  width: 2.1rem;
  height: 2.1rem;
  border-radius: 0.58rem;
  border: none;
  background: transparent;
  color: hsl(var(--text3));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease, color 0.2s ease, transform 0.2s ease;
}

.rail-back-btn:hover {
  background: hsl(var(--hover));
  color: hsl(var(--text));
  transform: translateY(-1px);
}
</style>
