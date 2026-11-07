<template lang="pug">
div(class="quick-access-shell flex flex-col gap-y-3")
  div(class="quick-access-header flex w-full justify-between items-center gap-x-2")
    div(class="flex items-center gap-x-3")
      Fa6SolidLayerGroup(class="quick-access-heading-icon")
      p(class="text-[1.04rem] font-semibold text-themeText") Quick Access
    button(v-if="!isEditMode" class="quick-access-edit-btn ml-auto py-1" @click="toggleEditMode") Edit Widgets
    button(v-else class="text-sm bg-themeButton text-white px-3 py-1 rounded-full ml-auto" @click="saveAndExit") Done Editing

  //- Edit Mode Indicator
  div(v-if="isEditMode" class="rounded-lg p-3 mb-3" style="background: hsl(var(--button) / 0.08); border: 1px solid hsl(var(--button) / 0.2)")
    p(class="text-sm" style="color: hsl(var(--button))") 🎯 Drag widgets to reorder • Click ✕ to remove • Click + to add widgets

  div#swapy-container(ref="container" class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 2xl:grid-cols-6 items-stretch gap-3 lg:gap-4" :class="{'edit-mode': isEditMode}")
    //- Simple approach - just render active widgets
    template(v-for="(widget, index) in activeWidgets" :key="`slot-${index}`")
      div(:data-swapy-slot="index" class="h-full min-w-0")
        div(:data-swapy-item="widget.id" class="relative h-full min-w-0 w-full")
          //- Remove button in edit mode (outside of the link/div)
          button(
            v-if="isEditMode"
            @click.prevent="removeWidget(widget.id)"
            class="absolute -top-2 -right-2 w-6 h-6 bg-themeError rounded-full flex items-center justify-center hover:brightness-90 transition-colors z-10"
          )
            Fa6SolidXmark(class="w-3 h-3 text-white")

          //- Widget content - NuxtLink for route navigation only
          NuxtLink(
            v-if="widget.route && !isEditMode"
            :to="widget.route"
            class="quick-widget-card group flex min-h-[7.25rem] w-full min-w-0 flex-col items-center p-3 rounded-xl cursor-pointer h-full"
          )
            div(class="quick-widget-icon w-11 h-11 rounded-full flex items-center justify-center mb-2" :style="{ borderColor: widget.color + '22', background: widget.color + '0d' }")
              component(:is="getIconComponent(widget.icon)" class="w-5 h-5" :style="{ color: widget.color }")
            span(class="text-[0.98rem] leading-tight font-semibold text-center text-themeText") {{ widget.name }}
            span(class="text-[0.78rem] text-themeText2 text-center mt-0.5") {{ widget.description }}

          //- For Tauri commands or edit mode, use clickable div
          button(
            v-else
            type="button"
            class="quick-widget-card group flex min-h-[7.25rem] w-full min-w-0 flex-col items-center p-3 rounded-xl cursor-pointer h-full"
            :class="{'ring-2 ring-themeButton': isEditMode, 'cursor-grab': isEditMode}"
            @click="handleWidgetClick(widget, $event)"
          )
            div(class="quick-widget-icon w-11 h-11 rounded-full flex items-center justify-center mb-2" :style="{ borderColor: widget.color + '22', background: widget.color + '0d' }")
              component(:is="getIconComponent(widget.icon)" class="w-5 h-5" :style="{ color: widget.color }")
            span(class="text-[0.98rem] leading-tight font-semibold text-center text-themeText") {{ widget.name }}
            span(class="text-[0.78rem] text-themeText2 text-center mt-0.5") {{ widget.description }}

    //- Add Widget button in edit mode
    button(
      v-if="isEditMode && inactiveWidgets.length > 0"
      type="button"
      @click="showAddWidgetModal = true"
      class="group flex min-h-[7.25rem] w-full min-w-0 flex-col items-center p-3 bg-themeBackground3/30 hover:bg-themeBackground3/50 rounded-lg transition-all duration-200 cursor-pointer border-2 border-dashed border-themeButton"
    )
      div(class="w-12 h-12 rounded-full flex items-center justify-center mb-2 transition-colors" style="background: hsl(var(--button) / 0.1)")
        Fa6SolidPlus(class="w-5 h-5 text-themeButton")
      span(class="text-sm font-medium text-center text-themeButton") Add Widget
      span(class="text-[10px] text-themeButton/60 text-center mt-1") Click to add

  //- Add Widget Modal
  BaseModal(:is-open="showAddWidgetModal" :z-index="12070" @close="showAddWidgetModal = false")
    div(class="vd-modal-card p-6 max-w-md w-full mx-4")
      div(class="flex items-center justify-between mb-4")
        h3(class="text-lg font-semibold") Add Widgets
        button(@click="showAddWidgetModal = false" class="vd-modal-close-btn")
          Fa6SolidXmark(class="w-5 h-5")

      div(class="space-y-3 max-h-96 overflow-y-auto")
        p(class="text-sm text-themeText/70 mb-3") Select widgets to add to your quick access panel:

        div(
          v-for="widget in inactiveWidgets"
          :key="widget.id"
          @click="addWidget(widget.id)"
          class="flex items-center justify-between p-3 bg-themeBackground3/30 rounded-lg hover:bg-themeBackground3/50 cursor-pointer "
        )
          div(class="flex items-center gap-x-3")
            div(class="w-10 h-10 rounded-full flex items-center justify-center" :style="{ background: widget.color + '18', border: '1px solid ' + widget.color + '28' }")
              component(:is="getIconComponent(widget.icon)" class="w-5 h-5" :style="{ color: widget.color }")
            div
              p(class="text-sm font-medium") {{widget.name}}
              p(class="text-[10px] text-themeText/60") {{widget.description}}
          Fa6SolidPlus(class="w-4 h-4 text-themeText3")
</template>

<script setup lang="ts">
import BaseModal from "../Modal/BaseModal.vue";
import { ref, computed, onMounted, onUnmounted, nextTick, type Component } from "vue";
import { createSwapy, type Swapy, type SwapEvent } from "swapy";
import { load, type Store } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";

import Fa6SolidTv from "~icons/fa6-solid/tv?width=16px&height=16px";
import Fa6SolidSitemap from "~icons/fa6-solid/sitemap?width=16px&height=16px";
import Fa6SolidMobileScreenButton from "~icons/fa6-solid/mobile-screen-button?width=16px&height=16px";
import Fa6SolidLayerGroup from "~icons/fa6-solid/layer-group?width=16px&height=16px";
import Fa6SolidGrip from "~icons/fa6-solid/grip?width=16px&height=16px";
import Fa6SolidGear from "~icons/fa6-solid/gear?width=16px&height=16px";
import Fa6SolidCubes from "~icons/fa6-solid/cubes?width=16px&height=16px";
import Fa6SolidUserGroup from "~icons/fa6-solid/user-group?width=16px&height=16px";
import Fa6SolidXmark from "~icons/fa6-solid/xmark?width=16px&height=16px";
import Fa6SolidPlus from "~icons/fa6-solid/plus?width=16px&height=16px";

const isEditMode = ref(false);
const showAddWidgetModal = ref(false);
interface WidgetDefinition {
  id: string;
  name: string;
  description: string;
  icon: string;
  color: string;
  tauriCommand?: string;
  route?: string;
}

interface WidgetPreferences {
  visibleWidgets: string[];
}

const container = ref<HTMLElement | null>(null);

let swapyInstance: Swapy | null = null;
let widgetStore: Store | null = null;

// Hardcoded widget definitions - these are the available widgets in the app
const WIDGET_DEFINITIONS: WidgetDefinition[] = [
  {
    id: "overlay",
    name: "Overlay",
    description: "Stream Controls",
    icon: "Fa6SolidTv",
    color: "#8b5cf6",
    tauriCommand: "open_overlay_window",
  },
  {
    id: "flows",
    name: "Flows",
    description: "Automation",
    icon: "Fa6SolidSitemap",
    color: "#3b82f6",
    tauriCommand: "open_flow_editor",
  },
  {
    id: "simulator",
    name: "Simulator",
    description: "Mobile Test",
    icon: "Fa6SolidMobileScreenButton",
    color: "#06b6d4",
    tauriCommand: "open_mobile_simulator",
  },
  {
    id: "deck",
    name: "Deck",
    description: "Main Control",
    icon: "Fa6SolidGrip",
    color: "#6366f1",
    route: "/deck",
  },
  {
    id: "settings",
    name: "Settings",
    description: "Preferences",
    icon: "Fa6SolidGear",
    color: "#64748b",
    route: "/settings",
  },
  {
    id: "extensions",
    name: "Extensions",
    description: "Plugins",
    icon: "Fa6SolidCubes",
    color: "#f59e0b",
    route: "/deck/store/plugins",
  },
  {
    id: "community",
    name: "Community",
    description: "Connect",
    icon: "Fa6SolidUserGroup",
    color: "#10b981",
    route: "/community",
  },
];

// Widget preferences - only visible widgets in order, and future settings
const widgetPrefs = ref<WidgetPreferences>({
  visibleWidgets: ["overlay", "flows", "simulator", "deck", "settings"], // Visible widgets in display order
  // Future settings can go here:
  // openInNewWindow: {},
  // customSettings: {}
});

// Computed properties for active and inactive widgets
const activeWidgets = computed(() => {
  return widgetPrefs.value.visibleWidgets
    .map((id) => WIDGET_DEFINITIONS.find((w) => w.id === id))
    .filter((widget): widget is WidgetDefinition => widget !== undefined);
});

const inactiveWidgets = computed(() => {
  return WIDGET_DEFINITIONS.filter(
    (w) => !widgetPrefs.value.visibleWidgets.includes(w.id)
  );
});

// Icon mapping
const iconMap: Record<string, Component> = {
  Fa6SolidTv,
  Fa6SolidSitemap,
  Fa6SolidMobileScreenButton,
  Fa6SolidGrip,
  Fa6SolidGear,
  Fa6SolidCubes,
  Fa6SolidUserGroup,
};
const getIconComponent = (iconName: string) => iconMap[iconName] || Fa6SolidGrip;

// Edit mode functions
const toggleEditMode = async () => {
  isEditMode.value = true;
  await nextTick();
  if (swapyInstance) {
    swapyInstance.update();
    swapyInstance.enable(true);
  }
};

const saveAndExit = async () => {
  if (swapyInstance) {
    swapyInstance.enable(false);
  }
  isEditMode.value = false;
  await nextTick();
  swapyInstance?.update();
  await saveWidgetPreferences();
};

// Swap handler - reorder widgets based on the swap
const handleSwap = async (event: SwapEvent) => {
  console.log("Swap event:", event);

  // Wait for animation to complete
  await new Promise((resolve) => setTimeout(resolve, 250));

  const fromSlot = Number.parseInt(event.fromSlot, 10);
  const toSlot = Number.parseInt(event.toSlot, 10);
  if (!Number.isInteger(fromSlot) || !Number.isInteger(toSlot)) return;

  console.log(`Swapping slot ${fromSlot} with slot ${toSlot}`);

  // Get the visible widgets array
  const visibleWidgets = [...widgetPrefs.value.visibleWidgets];

  // Find the widget IDs to swap
  const fromWidgetId = visibleWidgets[fromSlot];
  const toWidgetId = visibleWidgets[toSlot];

  if (fromWidgetId && toWidgetId) {
    console.log(`Swapping ${fromWidgetId} with ${toWidgetId}`);

    // Swap positions in the visible widgets array
    visibleWidgets[fromSlot] = toWidgetId;
    visibleWidgets[toSlot] = fromWidgetId;

    // Update the visible widgets order
    widgetPrefs.value.visibleWidgets = visibleWidgets;
  }

  if (swapyInstance) {
    swapyInstance.update();
  }
};

// Widget management
const removeWidget = async (widgetId: string) => {
  // Remove from visible widgets
  widgetPrefs.value.visibleWidgets = widgetPrefs.value.visibleWidgets.filter(
    (id) => id !== widgetId
  );

  await nextTick();
  if (swapyInstance) {
    swapyInstance.update();
  }
};

const addWidget = async (widgetId: string) => {
  // Add to visible widgets if not already there
  if (!widgetPrefs.value.visibleWidgets.includes(widgetId)) {
    widgetPrefs.value.visibleWidgets.push(widgetId);
  }

  showAddWidgetModal.value = false;
  await nextTick();
  if (swapyInstance) {
    swapyInstance.update();
  }
};

const saveWidgetPreferences = async () => {
  if (widgetStore) {
    await widgetStore.set("preferences", widgetPrefs.value);
    await widgetStore.save();
  }
};

// Handle widget clicks
const handleWidgetClick = async (widget: WidgetDefinition, event: Event) => {
  // In edit mode, prevent any navigation
  if (isEditMode.value) {
    event.preventDefault();
    event.stopPropagation();
    return;
  }

  // Handle Tauri commands
  if (widget.tauriCommand) {
    try {
      await invoke(
        widget.tauriCommand,
        widget.tauriCommand === "open_flow_editor" ? { flowId: null } : undefined,
      );
      console.log(
        `Successfully opened ${widget.name} via Tauri command: ${widget.tauriCommand}`
      );
    } catch (error) {
      console.error(`Failed to execute Tauri command ${widget.tauriCommand}:`, error);
    }
  } else if (!widget.route) {
    console.log(`Widget ${widget.name} clicked - no action defined`);
  }
};

// Initialize Swapy and load saved preferences on mount
onMounted(async () => {
  // Initialize Tauri store for widgets
  try {
    widgetStore = await load("widgets.json", { autoSave: false, defaults: {} });
    const saved = await widgetStore.get<WidgetPreferences>("preferences");
    if (saved && Array.isArray(saved.visibleWidgets)) {
      widgetPrefs.value.visibleWidgets = saved.visibleWidgets.filter(
        (id): id is string => typeof id === "string" && WIDGET_DEFINITIONS.some((widget) => widget.id === id),
      );
      // Ignore old widgetOrder field if it exists
    }
  } catch (error) {
    console.error("Failed to load widget preferences:", error);
  }

  await nextTick();

  // Initialize Swapy with proper configuration
  const swapyContainer = container.value;
  if (swapyContainer) {
    swapyInstance = createSwapy(swapyContainer, {
      enabled: false, // Start disabled, enable in edit mode
      animation: "dynamic",
      swapMode: "drop",
    });

    swapyInstance.onSwap(handleSwap);
  }
});

// Clean up on unmount
onUnmounted(() => {
  swapyInstance?.destroy();
  swapyInstance = null;
  void widgetStore?.close();
  widgetStore = null;
});
</script>

<style scoped lang="scss">
@use "../Modal/modalTheme.scss" as *;

.quick-access-shell {
  border-radius: 0;
  border: none;
  background: transparent;
  box-shadow: none;
}

.quick-access-heading-icon {
  width: 1.55rem;
  height: 1.55rem;
  padding: 0.34rem;
  color: hsl(var(--button));
  border: 1px solid hsl(var(--button) / 0.2);
  border-radius: 0.5rem;
  background: hsl(var(--button) / 0.1);
}

.quick-access-edit-btn {
  color: hsl(var(--button));
  font-size: 0.8rem;
  font-weight: 600;
  line-height: 1;
  padding: 0.22rem 0.5rem;
  border-radius: 999px;
  transition: color 0.2s ease, background 0.2s ease;
}

.quick-access-edit-btn:hover {
  color: hsl(var(--button) / 0.85);
  background: hsl(var(--button) / 0.1);
}

.quick-widget-card {
  border: 1px solid hsl(var(--border2) / 0.62);
  background: linear-gradient(
    160deg,
    hsl(var(--background2) / 0.95),
    hsl(var(--background3) / 0.72)
  );
  box-shadow: 0 8px 18px -18px hsl(var(--text) / 0.2);
  transition:
    transform 0.2s ease,
    border-color 0.2s ease,
    background 0.2s ease,
    box-shadow 0.2s ease;
}

.quick-widget-card:hover {
  transform: translateY(-1px);
  border-color: hsl(var(--editor-accent, 20 96% 55%) / 0.32);
  background: linear-gradient(
    160deg,
    hsl(var(--background2) / 0.98),
    hsl(var(--background3) / 0.82)
  );
  box-shadow: 0 12px 24px -20px hsl(var(--text) / 0.25);
}

.quick-widget-icon {
  border: 1px solid hsl(var(--border2) / 0.7);
  background: hsl(var(--background2) / 0.92);
}
</style>
