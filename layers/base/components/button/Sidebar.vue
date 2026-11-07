<template lang="pug">
NuxtLink.menu-button(
  @click="fn && fn()"
  @contextmenu="handleRightClick"
  :to="props.to"
  :class="isActive? 'font-bold': ''"
  class="relative whitespace-nowrap text-xl text-ellipsis flex w-full group justify-between rounded-md  hover:bg-themeBackground  md:h-[2.7rem] px-4 py-7 items-center hover:cursor-pointer"
)
    div(class="flex gap-x-4 items-center w-full")
        div(class="w-6 md:w-7 h-6 md:h-7 flex justify-center items-center")
          slot(name="icon")
        p(v-if="props.text") {{ props.text }}
    div(v-if="props.notifications_count >= 0" :class="props.notifications_count > 0? 'moving-flag-bg': 'bg-slate-50'" class="h-full w-[2rem]  flex items-center justify-center rounded-md")
        p() {{props.notifications_count}}

    //- div(class="w-5 md:w-7 h-5 md:h-7 flex justify-center items-center")
    //-   slot(name="icon-2")
    .active-pointer(v-if="isActive" class="absolute -left-[3.5rem]  w-[0.65rem] h-11 bg-violet-600 rounded-r-md")

// Global context menu is handled separately
</template>

<script setup lang="ts">
import { computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useGlobalContextMenu } from "../../composables/useGlobalContextMenu";

interface ContextMenuItem {
  id: string;
  label: string;
  action: () => void;
}

const props = defineProps({
  text: {
    type: String,
    required: false,
  },
  to: {
    type: String,
    required: false,
  },

  notifications_count: {
    type: Number,
    required: false,
  },
  fn: {
    type: Function,
    required: false,
  },
  windowFn: {
    type: Function,
    required: false,
  },
});

const route = useRoute();
const isActive = computed(() => {
  const path = route.path.split("/");
  return route.path.split("/")[path.length - 1] === props.text?.toLowerCase()
    ? true
    : false;
});

// Use global context menu
const {
  globalContextMenu,
  showContextMenu: showGlobalContextMenu,
  handleItemClick,
} = useGlobalContextMenu();

const contextMenuItems = computed((): ContextMenuItem[] => {
  const items: ContextMenuItem[] = [];

  // Only show "Open in New Window" for pages that have a 'to' prop (not functions like Logs)
  if (props.to) {
    items.push({
      id: "open-in-new-window",
      label: "Open in New Window",
      action: () => openInNewWindow(),
    });
  }

  return items;
});

const handleRightClick = (event: MouseEvent) => {
  event.preventDefault();

  // Only show context menu if we have items
  if (contextMenuItems.value.length === 0) {
    return;
  }

  showGlobalContextMenu(event, contextMenuItems.value);
};

const openInNewWindow = async () => {
  // If a custom windowFn is provided, use it (e.g., for Streaming window)
  if (props.windowFn) {
    try {
      await props.windowFn();
    } catch (error) {
      console.error("Failed to open window:", error);
    }
    return;
  }

  // Otherwise use the generic open_in_new_window command
  if (props.to && props.text) {
    try {
      await invoke("open_in_new_window", {
        pagePath: props.to,
        windowTitle: `VortiDeck - ${props.text}`,
      });
    } catch (error) {
      console.error("Failed to open in new window:", error);
    }
  }
};

// Context menu is managed by the global composable
</script>

<style lang="scss">
.menu-button {
  .active-pointer {
    animation: slideIn 0.6s forwards;
  }
}
@keyframes slideIn {
  0% {
    transform: translateX(-2rem);
  }
  100% {
    transform: translateX(-0.6rem);
  }
}

.moving-flag-bg {
  animation: colorChange 2s infinite;
}

@keyframes colorChange {
  0% {
    background: rgb(241 245 249);
  }
  10% {
    background: #7c3aed;
    color: white;
  }
  30% {
    background: #7c3aed;
    color: white;
  }
  40% {
    background: rgb(241 245 249);
  }
  100% {
    background: rgb(241 245 249);
  }
}
</style>
