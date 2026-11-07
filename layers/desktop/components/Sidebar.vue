<template lang="pug">


div#sidebar(
  :class="[is_sidebar_open && desktopUtils.getWasClicked ? 'show-right' : '',!is_sidebar_open && !desktopUtils.getWasClicked ? 'hidden 2xl:block' : 'hide-left',desktopUtils.getWasClicked ? '' : breakpoints.greater('2xl') ? '' : 'hidden']"
  class="z-[100] fixed w-[20rem] lg:w-[23rem] bottom-0 left-0 h-[100lvh] bg-themeBackground2 pb-4 shadow-lg rounded-br-xl text-themeText flex flex-col"
) 
  div#sidebar-content(class="flex-1 overflow-y-auto relative w-full flex flex-col items-start rounded-md px-2 p-6 mt-[5rem]")
    ButtonSidebar(
      v-for="(button, index) in sidebarButtons"
      :key="button.to"
      :text="button.text"
      :to="button.to"
      :fn="button.fn"
      :window-fn="button.windowFn"
      :notifications_count="button.notifications_count"
      @click="desktopUtils.toggleSidebar"
    )
        template(v-slot:icon)
          component(:is="button.icon" class="w-[1.4rem] h-[1.4rem]")

  
</template>

<script setup lang="ts">
import DashboardIcon from "~icons/ant-design/home-outlined";
import SubscriptionIcon from "~icons/fluent-mdl2/recurring-event";
import FluentCommunication24Regular from "~icons/fluent/communication-24-regular?width=24px&height=24px";
import RiApps2Line from "~icons/ri/apps-2-line?width=24px&height=24px";
import FluentPremium28Regular from "~icons/fluent/premium-28-regular?width=28px&height=28px";
import PepiconsPencilKeyboard from "~icons/pepicons-pencil/keyboard?width=20px&height=20px";
import IconParkOutlineLog from "~icons/icon-park-outline/log";
import MaterialSymbolsBugReportOutline from "~icons/material-symbols/bug-report-outline";
import MdiWebhook from "~icons/mdi/webhook?width=24px&height=24px";
import MaterialSymbolsCameraVideoOutline from "~icons/material-symbols/camera-video-outline?width=24px&height=24px";

import { invoke } from "@tauri-apps/api/core";
import { breakpointsTailwind, useBreakpoints } from "@vueuse/core";

const breakpoints = useBreakpoints(breakpointsTailwind);

import { useDesktopUtils } from "../stores/desktopUtils";
const desktopUtils = useDesktopUtils();

// Development mode detection
const isDev = process.env.NODE_ENV === "development";

const is_sidebar_open = computed(() => {
  return desktopUtils.getIsSidebar;
});

const openLogsWindow = async () => {
  await invoke("open_logs");
};

const openStreamingWindow = async () => {
  await invoke("open_streaming_window");
};
const isMounted = ref(false);

onMounted(() => {
  // Wait for the component to load before allowing animations
  isMounted.value = true;
  console.log(is_sidebar_open.value);
});

// Base sidebar buttons
const baseSidebarButtons = [
  { text: "Home", to: "/", icon: DashboardIcon },
  { text: "Deck", to: "/deck", icon: PepiconsPencilKeyboard },
  {
    text: "Streaming",
    icon: MaterialSymbolsCameraVideoOutline,
    fn: openStreamingWindow,
  },
  // { text: "Games and Applications", to: "/applications", icon: RiApps2Line },
  { text: "API Management", to: "/webhooks", icon: MdiWebhook },
  { text: "Community", to: "/community", icon: FluentCommunication24Regular },
  // {
  //   text: "Premium",
  //   to: "/premium",
  //   icon: FluentPremium28Regular,
  // },
  { text: "Logs", fn: openLogsWindow, icon: IconParkOutlineLog },
];

// Add debug button only in development mode
const sidebarButtons = computed(() => {
  const buttons = [...baseSidebarButtons];

  if (isDev) {
    buttons.push({
      text: "Debug",
      to: "/debug",
      icon: MaterialSymbolsBugReportOutline,
    });
  }

  return buttons;
});

function enter(el, done) {
  // Transition in
  el.style.opacity = 0;
  el.style.height = "0";

  requestAnimationFrame(() => {
    el.style.transition = "opacity 0.2s, height 0.2s";
    el.style.opacity = "1";
    el.style.height = "190px"; // Adjust the height you want the submenu to expand to

    // Wait for the transition to finish
    el.addEventListener("transitionend", done);
  });
}

function leave(el, done) {
  // Transition out
  el.style.transition = "opacity 0.2s, height 0.2s";
  el.style.opacity = "0";
  el.style.height = "0";

  // Wait for the transition to finish
  el.addEventListener("transitionend", done);
}
</script>

<style lang="scss" scoped>
.hide-left {
  animation: hide-left 0.5s ease-in-out forwards;
  //2xl
  @media (min-width: 1536px) {
    animation: none;
  }
}
.fade-enter {
}
.fade-enter-active {
  animation: fade-in 0.3s;
}
.fade-leave {
}
.fade-leave-active {
  animation: fade-out 0.3s;
}
@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
@keyframes fade-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}

@keyframes hide-left {
  from {
    transform: translateX(0rem);
  }

  to {
    transform: translateX(-100%);
  }
}
.show-right {
  animation: show-right 0.5s ease-in-out forwards;

  //2xl
  @media (min-width: 1536px) {
    animation: none;
  }
}

@keyframes show-right {
  from {
    transform: translateX(-25rem);
  }
  to {
    transform: translateX(0rem);
  }
}
</style>
