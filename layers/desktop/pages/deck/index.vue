<template lang="pug">
div(ref="deckEditorPage" class="deck-editor-page flex justify-start flex-row w-full h-full min-h-0 overflow-hidden")
    div#drag-icon(class="fixed top-[-1000px] left-[-1000px] h-[2rem] w-[2rem]  flex items-center justify-center aspect-square bg-black  text-white font-bold pointer-events-none	")
      HugeiconsCommandLine(class="w-6 h-6 text-white")
    div(ref="assignmentPanel" :style="assignmentPanelStyle" class="assignment-panel flex flex-shrink-0 flex-col gap-y-5 w-[36%] min-w-[20rem] max-w-[28rem] h-full pl-5 pr-4 py-5 overflow-hidden")
        div(class="flex flex-col gap-y-4")
          div(class="assignment-tabs flex flex-nowrap w-full gap-x-1 font-semibold overflow-x-auto")
              button.assignment-tab(@click="desktopUtils.setDeckEditorMenu('keys')" :class="menu === 'keys' ? 'assignment-tab--active' : ''") {{$t('deck_editor.navigation.keys.title')}}
              button.assignment-tab(@click="desktopUtils.setDeckEditorMenu('programs')" :class="menu === 'programs' ? 'assignment-tab--active' : ''") Programs
              button.assignment-tab(@click="desktopUtils.setDeckEditorMenu('actions')" :class="menu === 'actions' ? 'assignment-tab--active' : ''") {{$t('deck_editor.navigation.actions.title')}}
              button.assignment-tab(@click="desktopUtils.setDeckEditorMenu('macros')" :class="menu === 'macros' ? 'assignment-tab--active' : ''") {{$t('deck_editor.navigation.macros.title')}}
              button.assignment-tab(@click="desktopUtils.setDeckEditorMenu('flows')" :class="menu === 'flows' ? 'assignment-tab--active' : ''") {{$t('deck_editor.navigation.flows.title')}}
              button.assignment-tab(@click="desktopUtils.setDeckEditorMenu('utilities')" :class="menu === 'utilities' ? 'assignment-tab--active' : ''") {{$t('deck_editor.navigation.utilities.title')}}
        div(class="flex flex-col gap-y-4 h-full flex-grow overflow-y-hidden" v-if="menu === 'keys'")
          DeckEditorLeftPanelMenuTools(:tip="$t('deck_editor.navigation.keys.tip')" @search="setSearchPrompt" :searchPlaceholder="$t('deck_editor.navigation.keys.search_keys')" )
            
          div(class="assignment-list flex flex-col h-full overflow-y-auto pr-1")
              
              div.assignment-row(draggable="true" v-for="key in allKeys" :key="key.name")
                  div(class="assignment-icon assignment-icon--key") K
                  p(class="assignment-name") {{key.name}}
                  p(class="assignment-attached") {{key.attachedButtons}}
              div(v-if="allKeys.length === 0" class="flex flex-col items-center justify-center h-full text-center")
                TeenyiconsButtonOutline(class="w-12 h-12 mb-4 text-themeText3")
                p(class="text-themeText3 mb-2") {{$t('deck_editor.keys.no_keys_found')}}
                p(class="text-sm text-themeText3") {{$t('deck_editor.keys.create_first_key')}}
        div(class="flex flex-col gap-y-4 h-full flex-grow overflow-y-hidden" v-if="menu === 'programs'")
          DeckEditorLeftPanelMenuTools(:tip="'Create and manage program actions'" @search="setSearchPrompt" :searchPlaceholder="'Search programs...'" )
            template(v-slot:button)
              NuxtLink(to="/deck/new/program" class="deck-primary-btn") Create Program

          div(class="assignment-list flex flex-col h-full overflow-y-auto pr-1")
            // Loading state for programs
            div(v-if="loadingProgress.programs === 'loading'" class="space-y-3 p-4")
              div(class="text-sm text-themeText3 flex items-center gap-2 mb-4")
                div(class="animate-spin w-4 h-4 border-2 border-themeButton border-t-transparent rounded-full")
                span Loading programs...
              div(v-for="n in 4" :key="n" class="animate-pulse flex items-center gap-x-3 p-2")
                div(class="w-8 h-8 bg-themeBackground3 rounded")
                div(class="flex-1 space-y-1")
                  div(class="h-4 bg-themeBackground3 rounded w-3/4")
                  div(class="h-3 bg-themeBackground3/60 rounded w-1/2")

            // Error state for programs
            div(v-else-if="loadingProgress.programs === 'error'" class="flex flex-col items-center justify-center h-full text-center p-4")
              HugeiconsCommandLine(class="w-12 h-12 mb-4 text-themeError")
              p(class="text-themeError mb-2") Failed to load programs
              button(@click="retryLoad('programs')" class="px-4 py-2 bg-themeButton text-white rounded hover:brightness-110 transition-colors") Retry

            // Empty state for programs
            div(v-else-if="allProgramsWithButtons.length === 0 && loadingProgress.programs === 'loaded'" class="flex flex-col items-center justify-center h-full text-center")
              HugeiconsCommandLine(class="w-12 h-12 mb-4 text-themeText3")
              p(class="text-themeText3 mb-2") No programs found
              p(class="text-sm text-themeText3") Create your first program to get started

            // Programs list
            div(v-else-if="allProgramsWithButtons.length > 0")
              div.assignment-row(@click="editProgram(program)" @contextmenu="handleProgramContextMenu($event, program)" draggable="true" @dragstart="useHandleDragAction($event, program.id, 'program', program.name)" v-for="program in allProgramsWithButtons" :key="program.id" :title="'Click to edit program'")
                // Program icon
                div(class="assignment-icon")
                  // Show custom icon from HTTP service if available
                  nuxt-img(v-if="getCustomIconUrl(program)" class="w-full h-full object-cover" :src="getCustomIconUrl(program)")
                  // Show extracted program icon if available (fallback)
                  nuxt-img(v-else-if="program.icon_data" class="w-full h-full object-cover" :src="program.icon_data")
                  // Show default command icon if no icon
                  HugeiconsCommandLine(v-else class="w-4 h-4 text-themeText3")

                div(class="flex flex-col flex-1 min-w-0")
                  p(class="font-medium truncate text-themeText") {{ program.name }}
                  p(class="text-xs text-themeText2 truncate") {{ program.description || program.executable }}
                p(class="assignment-attached") {{ program.attachedButtons }}
                button(class="invisible group-hover:visible hover:scale-110 hover:text-themeError transition-transform transition-colors duration-150" @click.stop="removeProgram(program.id)" :title="'Delete program'")
                  MdiRemove(class="w-5 h-5")
        div(class="flex flex-col gap-y-4 h-full flex-grow overflow-y-hidden" v-if="menu === 'actions'")
          //- DeckEditorScript
          DeckEditorLeftPanelMenuTools(:tip="$t('deck_editor.navigation.actions.tip')" @search="setSearchPrompt" :searchPlaceholder="$t('deck_editor.navigation.actions.search_actions')" )
          

          Submenu(v-for="group in savedActionGroups" :key="group.integrationId" :items="group.items" :actionType="group.integrationId" routeTo="/deck/new/action" @removeAction="removeAction")
            template(v-slot:context)
              SimpleIconsObsstudio(v-if="group.integrationId === 'obs'" class="submenu-context-icon")
              SimpleIconsDiscord(v-else-if="group.integrationId === 'discord'" class="submenu-context-icon")
              MdiConnection(v-else class="submenu-context-icon")
              p(class="submenu-context-label") {{ group.label }}
              NuxtLink(:to="{path:'/deck/new/action', query: {integrationType: group.integrationId}}" @click.stop="" class="deck-mini-btn")
                Fa6SolidPlus(class="w-4 h-4")
                span(class="text-sm") Add

          Submenu(:items="allSystemControlsWithButtons" actionType="system_control" :editable="false")
            template(v-slot:context)
              MdiTuner(class="submenu-context-icon" )
              p(class="submenu-context-label") System controls
              span(class="submenu-built-in-badge") Built in

        div(class="flex flex-col gap-y-4 flex-grow overflow-y-hidden "  v-if="menu === 'macros'")
          DeckEditorLeftPanelMenuTools(:tip="$t('deck_editor.navigation.macros.tip')" @search="setSearchPrompt" :searchPlaceholder="$t('deck_editor.navigation.macros.search_macros')" )
            template(v-slot:button)
              NuxtLink(to="/deck/new/macro" class="deck-primary-btn") {{$t('deck_editor.macros.create_new_macro')}}
          div(class="assignment-list flex flex-col h-full overflow-y-auto pr-1")
            // Loading state for macros
            div(v-if="loadingProgress.macros === 'loading'" class="space-y-3 p-4")
              div(class="text-sm text-themeText3 flex items-center gap-2 mb-4")
                div(class="animate-spin w-4 h-4 border-2 border-themeButton border-t-transparent rounded-full")
                span Loading macros...
              div(v-for="n in 3" :key="n" class="animate-pulse flex items-center gap-x-3 p-2")
                div(class="w-8 h-8 bg-themeBackground3 rounded")
                div(class="flex-1 h-4 bg-themeBackground3 rounded")

            // Error state for macros
            div(v-else-if="loadingProgress.macros === 'error'" class="flex flex-col items-center justify-center h-full text-center p-4")
              div(class="assignment-icon assignment-icon--macro w-12 h-12 mb-4 font-bold text-lg") M
              p(class="text-themeError mb-2") Failed to load macros
              button(@click="retryLoad('macros')" class="px-4 py-2 bg-themeButton text-white rounded hover:brightness-110 transition-colors") Retry

            // Empty state for macros
            div(v-else-if="allMacrosWithButtons.length === 0 && loadingProgress.macros === 'loaded'" class="flex flex-col items-center justify-center h-full text-center")
              div(class="assignment-icon assignment-icon--macro w-12 h-12 mb-4 font-bold text-lg") M
              p(class="text-themeText3 mb-2") No macros found
              p(class="text-sm text-themeText3") Create your first macro to get started

            // Macros list
            div(v-else-if="allMacrosWithButtons.length > 0")
              div.assignment-row(@click="router.push({path:'deck/new/macro', query: {macroId: macro.id, step: '1'} })" @contextmenu="handleMacroContextMenu($event, macro)" draggable="true" @dragstart="useHandleDragAction($event, macro.id, 'macro', macro.name)" v-for="macro in allMacrosWithButtons")
                  div(class="assignment-icon assignment-icon--macro") M
                  p(class="assignment-name") {{macro.name}}
                  p(class="assignment-attached") {{macro.attachedButtons}}
                  button(class="invisible group-hover:visible hover:scale-110 hover:text-themeError transition-transform transition-colors duration-150" @click.stop="removeMacro(macro.id)" :title="'Delete macro'")
                    MdiRemove(class="w-5 h-5")
        div(class="flex flex-col gap-y-4 flex-grow overflow-hidden "  v-if="menu === 'flows'")

          DeckEditorLeftPanelMenuTools(:tip="$t('deck_editor.navigation.flows.tip')" @search="setSearchPrompt" :searchPlaceholder="$t('deck_editor.navigation.flows.search_flows')" )
            template(v-slot:button)
                button(@click="openFlowEditor" class="deck-primary-btn") {{$t('deck_editor.flows.create_new_flow')}}

          div(class="assignment-list flex flex-col h-full overflow-y-auto pr-1")
            // Loading state for flows
            div(v-if="loadingProgress.flows === 'loading'" class="space-y-3 p-4")
              div(class="text-sm text-themeText3 flex items-center gap-2 mb-4")
                div(class="animate-spin w-4 h-4 border-2 border-themeButton border-t-transparent rounded-full")
                span Loading flows...
              div(v-for="n in 3" :key="n" class="animate-pulse flex items-center gap-x-3 p-2")
                div(class="w-8 h-8 bg-themeBackground3 rounded")
                div(class="flex-1 space-y-1")
                  div(class="h-4 bg-themeBackground3 rounded w-3/4")
                  div(class="h-3 bg-themeBackground3/60 rounded w-1/2")

            // Error state for flows
            div(v-else-if="loadingProgress.flows === 'error'" class="flex flex-col items-center justify-center h-full text-center p-4")
              svg(class="w-12 h-12 mb-4 text-themeError" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                circle(cx="6" cy="6" r="3" stroke-width="2")
                circle(cx="6" cy="18" r="3" stroke-width="2")
                circle(cx="18" cy="12" r="3" stroke-width="2")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.5 7.5l7 3M8.5 16.5l7-3")
              p(class="text-themeError mb-2") Failed to load flows
              button(@click="retryLoad('flows')" class="px-4 py-2 bg-themeButton text-white rounded hover:brightness-110 transition-colors") Retry

            // Empty state for flows
            div(v-else-if="allFlowsWithButtons.length === 0 && loadingProgress.flows === 'loaded'" class="flex flex-col items-center justify-center h-full text-center")
              svg(class="w-12 h-12 mb-4 text-themeText3" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                circle(cx="6" cy="6" r="3" stroke-width="2")
                circle(cx="6" cy="18" r="3" stroke-width="2")
                circle(cx="18" cy="12" r="3" stroke-width="2")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.5 7.5l7 3M8.5 16.5l7-3")
              p(class="text-themeText3 mb-2") {{$t('deck_editor.flows.no_flows_found')}}
              p(class="text-sm text-themeText3") {{$t('deck_editor.flows.create_first_flow')}}

            // Flows list
            div(v-else-if="allFlowsWithButtons.length > 0")
              div.assignment-row(@click="openFlowInEditor(flow)" @contextmenu="handleFlowContextMenu($event, flow)" draggable="true" @dragstart="useHandleDragAction($event, flow.id, 'flow', flow.name)" v-for="flow in allFlowsWithButtons" :key="flow.id" :title="'Click to open in flow editor'")
                div(class="assignment-icon assignment-icon--flow")
                  svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                    circle(cx="6" cy="6" r="3" stroke-width="2")
                    circle(cx="6" cy="18" r="3" stroke-width="2")
                    circle(cx="18" cy="12" r="3" stroke-width="2")
                    path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.5 7.5l7 3M8.5 16.5l7-3")
                div(class="flex flex-col flex-1 min-w-0")
                  p(class="font-medium truncate text-themeText") {{flow.name}}
                  p(class="text-xs text-themeText2 truncate") {{ flow.description || formatFlowUpdatedAt(flow.updatedAt) }}
                p(class="assignment-attached") {{flow.attachedButtons}}

                button(class="invisible group-hover:visible hover:scale-110 hover:text-themeError transition-transform transition-colors duration-150" @click.stop="removeFlow(flow.id)" :title="'Delete flow'")
                  MdiRemove(class="w-5 h-5")
        
        div(class="flex flex-col gap-y-4 flex-grow overflow-hidden "  v-if="menu === 'utilities'")

          DeckEditorLeftPanelMenuTools(:tip="$t('deck_editor.navigation.utilities.tip')" @search="setSearchPrompt" :searchPlaceholder="$t('deck_editor.navigation.utilities.search_utilities')" )

          Submenu(:items="allHttpRequestsWithButtons" actionType="http_request" routeTo="/deck/new/http-request" @removeAction="removeHttpRequest")
            template(v-slot:context)
              HeroiconsGlobeAlt(class="submenu-context-icon" )
              p(class="submenu-context-label") HTTP Requests

              // add action
              NuxtLink(:to="{path:'/deck/new/http-request' }" @click.stop="" class="deck-mini-btn")
                Fa6SolidPlus(class="w-4 h-4")
                span(class="text-sm") Add

          Submenu(:items="allFocusWindowsWithButtons" actionType="focus_window" routeTo="/deck/new/focus-window" @removeAction="removeFocusWindow")
            template(v-slot:context)
              svg(class="submenu-context-icon submenu-context-icon--padded" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2")
              p(class="submenu-context-label") Focus Windows

              // add action
              NuxtLink(:to="{path:'/deck/new/focus-window' }" @click.stop="" class="deck-mini-btn")
                Fa6SolidPlus(class="w-4 h-4")
                span(class="text-sm") Add

    button(
      type="button"
      role="separator"
      aria-label="Resize assignments panel"
      aria-orientation="vertical"
      :aria-valuenow="Math.round(assignmentPanelWidth || assignmentPanel?.getBoundingClientRect().width || 0)"
      :class="{ 'deck-panel-resizer--active': isResizingAssignmentPanel }"
      class="deck-panel-resizer"
      @pointerdown="startAssignmentPanelResize"
      @keydown="handleAssignmentPanelResizeKeydown"
    )
    div(class="configuration-shell scrollbar-visible rounded-tl-2xl rounded-tr-none rounded-br-none rounded-bl-none flex flex-col flex-1 w-full min-w-0 h-full min-h-0 overflow-y-auto shadow-[-12px_0_28px_hsl(var(--text)/0.06)]")
      div(class="config-menu-shell")
        div(class="config-menu")
          button(
            v-for="tab in configTabs"
            :key="tab.key"
            type="button"
            class="config-menu-tab"
            :class="activeConfigTab === tab.key ? 'config-menu-tab--active' : ''"
            @click="activeConfigTab = tab.key"
          ) {{ tab.label }}

      div(class="config-body")
        DeckEditorContextMenuPanel(v-if="activeConfigTab === 'context-menu'")
        DeckEditorDragDropTipsPanel(v-else-if="activeConfigTab === 'tips'")
        template(v-else-if="activeConfigTab === 'settings'")
          div(class="config-section")
            p(class="config-label") Settings
            p(class="text-sm text-themeText2 opacity-60") Coming soon
</template>

<script setup lang="ts">
definePageMeta({
  layout: "deck-editor",
  contentPaddingPreset: "none",
  title: "Assignments",
  description: "Configure deck inputs and automated actions",
});

import MdiTuner from "~icons/mdi/tuner?width=24px&height=24px";
import MdiRemove from "~icons/mdi/remove?width=24px&height=24px";
import Fa6SolidPlus from "~icons/fa6-solid/plus?width=392px&height=448px";
import HugeiconsAiBrain02 from "~icons/hugeicons/ai-brain-02?width=24px&height=24px";
import MaterialSymbolsWarningRounded from "~icons/material-symbols/warning-rounded?width=24px&height=24px";
import SimpleIconsObsstudio from "~icons/simple-icons/obsstudio?width=48px&height=48px";
import SimpleIconsDiscord from "~icons/simple-icons/discord?width=48px&height=48px";
import MdiConnection from "~icons/mdi/connection?width=24px&height=24px";
import HugeiconsCommandLine from "~icons/hugeicons/command-line?width=24px&height=24px";
import HeroiconsGlobeAlt from "~icons/heroicons/globe-alt";
import TeenyiconsButtonOutline from "~icons/teenyicons/button-outline";

import { listen, emitTo } from "@tauri-apps/api/event";
import { useDeckStore } from "@/stores/Deck";
import { invoke } from "@tauri-apps/api/core";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { useMainStore } from "@/stores/Main";

interface IntegrationLifecyclePayload {
  integration_id: string;
  status: {
    status: "Connected" | "Disconnected" | "Connecting" | { Error: string };
  };
}

const deckStore = useDeckStore();
const desktopUtils = useDesktopUtils();
const mainStore = useMainStore();
const { showContextMenu } = useGlobalContextMenu();
const searchPrompt = ref("");
const setSearchPrompt = (event) => {
  searchPrompt.value = event;
};
const openFlowEditor = async () => {
  try {
    await invoke("open_flow_editor", { flowId: null });
  } catch {
    return;
  }
};

const openFlowInEditor = async (flow) => {
  try {
    await invoke("open_flow_editor", { flowId: flow.id });
  } catch (error) {
    console.error("Error opening flow in editor:", error);
  }
};

const themes = ref(deckStore.getThemes);

const menu = computed(() => {
  return desktopUtils.getDeckEditorMenu;
});

const buttonGridSpacing = ref(10);

const { t } = useI18n();

const router = useRouter();

const selectedButtonId = ref();
const configTabs = [
  { key: "context-menu", label: "Context Menu" },
  { key: "tips", label: "Tips" },
  { key: "settings", label: "Settings" },
];
const activeConfigTab = ref("context-menu");
const deckEditorPage = ref<HTMLElement | null>(null);
const assignmentPanel = ref<HTMLElement | null>(null);
const assignmentPanelWidth = ref<number | null>(null);
const isResizingAssignmentPanel = ref(false);
const assignmentPanelStyle = computed(() =>
  assignmentPanelWidth.value === null
    ? undefined
    : {
        width: `${assignmentPanelWidth.value}px`,
        flexBasis: `${assignmentPanelWidth.value}px`,
      },
);

const ASSIGNMENT_PANEL_STORAGE_KEY = "deck-editor-assignment-panel-width";
const ASSIGNMENT_PANEL_MIN_WIDTH = 320;
const ASSIGNMENT_PANEL_MAX_WIDTH = 448;
const CONFIGURATION_PANEL_MIN_WIDTH = 440;
let stopAssignmentPanelResize: (() => void) | null = null;

const clampAssignmentPanelWidth = (width: number) => {
  const availableWidth = deckEditorPage.value?.clientWidth ?? window.innerWidth;
  const responsiveMaximum = Math.max(
    ASSIGNMENT_PANEL_MIN_WIDTH,
    availableWidth - CONFIGURATION_PANEL_MIN_WIDTH,
  );
  return Math.round(
    Math.min(
      Math.max(width, ASSIGNMENT_PANEL_MIN_WIDTH),
      Math.min(ASSIGNMENT_PANEL_MAX_WIDTH, responsiveMaximum),
    ),
  );
};

const persistAssignmentPanelWidth = () => {
  if (assignmentPanelWidth.value !== null) {
    localStorage.setItem(
      ASSIGNMENT_PANEL_STORAGE_KEY,
      String(assignmentPanelWidth.value),
    );
  }
};

const resizeAssignmentPanelBy = (delta: number) => {
  const currentWidth =
    assignmentPanelWidth.value ??
    assignmentPanel.value?.getBoundingClientRect().width ??
    ASSIGNMENT_PANEL_MIN_WIDTH;
  assignmentPanelWidth.value = clampAssignmentPanelWidth(currentWidth + delta);
  persistAssignmentPanelWidth();
};

const handleAssignmentPanelResizeKeydown = (event: KeyboardEvent) => {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  resizeAssignmentPanelBy(event.key === "ArrowLeft" ? -16 : 16);
};

const startAssignmentPanelResize = (event: PointerEvent) => {
  if (!assignmentPanel.value) return;

  stopAssignmentPanelResize?.();
  const startX = event.clientX;
  const startWidth = assignmentPanel.value.getBoundingClientRect().width;
  isResizingAssignmentPanel.value = true;

  const handlePointerMove = (moveEvent: PointerEvent) => {
    assignmentPanelWidth.value = clampAssignmentPanelWidth(
      startWidth + moveEvent.clientX - startX,
    );
  };
  const finishResize = () => {
    persistAssignmentPanelWidth();
    isResizingAssignmentPanel.value = false;
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", finishResize);
    window.removeEventListener("pointercancel", finishResize);
    stopAssignmentPanelResize = null;
  };

  stopAssignmentPanelResize = finishResize;
  window.addEventListener("pointermove", handlePointerMove);
  window.addEventListener("pointerup", finishResize);
  window.addEventListener("pointercancel", finishResize);
};

onMounted(() => {
  const savedWidth = Number.parseFloat(
    localStorage.getItem(ASSIGNMENT_PANEL_STORAGE_KEY) ?? "",
  );
  if (Number.isFinite(savedWidth)) {
    assignmentPanelWidth.value = clampAssignmentPanelWidth(savedWidth);
  }
});

onUnmounted(() => stopAssignmentPanelResize?.());

// Background preloader for all menu data
const {
  preloadedData,
  loadingProgress,
  startBackgroundPreload,
  getMenuData,
  retryLoad,
  invalidateCache,
  invalidateAll,
} = useBackgroundPreloader();

// Start background preloading after page is interactive
onMounted(() => {
  nextTick(() => {
    const currentMenu = desktopUtils.getDeckEditorMenu;
    startBackgroundPreload(currentMenu);
  });
});

// Watch for menu changes and prioritize loading for new menu
watch(menu, (newMenu) => {
  // Only reload if we don't have data for this menu yet
  const menuData = getMenuData(newMenu);
  if (!menuData.isLoaded && !menuData.isLoading) {
    console.log(`🔄 Menu changed to ${newMenu}, priority loading...`);
    retryLoad(newMenu);
  }
});

// Check OBS integration status without coupling the saved-action catalog to
// connection state. Saved actions remain assignable and visible while OBS is off.
const initialObsConnected = await invoke<boolean>("is_integration_connected", {
  integrationId: "obs",
});
desktopUtils.setIntegrationStatus("obs", initialObsConnected);

// Universal integration event listener
const unlistenIntegrationStatusChanged = await listen<IntegrationLifecyclePayload>(
  "integration_status_changed",
  (event) => {
    console.log("Integration status changed:", event.payload);

    // Update the store for any integration
    const integrationId = event.payload.integration_id;
    const isConnected = event.payload.status.status === "Connected";

    desktopUtils.setIntegrationStatus(integrationId, isConnected);

    invalidateCache("savedActions");
  }
);

const unlistenIntegrationRegistered = await listen<IntegrationLifecyclePayload>(
  "integration_registered",
  (event) => {
    desktopUtils.setIntegrationStatus(
      event.payload.integration_id,
      event.payload.status.status === "Connected",
    );
    invalidateCache("savedActions");
  },
);

onUnmounted(() => {
  unlistenIntegrationStatusChanged();
  unlistenIntegrationRegistered();
});

const removeAction = async (item: Record<string, any>, actionType: string) => {
  if (item?.stable_id) {
    try {
      await invoke<void>("remove_integration_action_from_db", {
        integrationId: actionType,
        actionId: item.id,
      });
    } catch (error) {
      mainStore.addNotification({
        title: "Action is still in use",
        message: String(error),
        type: "error",
        autoClose: false,
      });
      return;
    }
    const grouped = preloadedData.value.savedActions || {};
    grouped[actionType] = (grouped[actionType] || []).filter((entry) => entry.id !== item.id);
    preloadedData.value.savedActions = { ...grouped };
    mainStore.addNotification({
      title: "Action removed",
      message: `${item.name} was removed`,
      type: "success",
      autoClose: true,
    });
    return;
  }
  switch (actionType) {
    case "macro":
      console.log(item);
      break;
    case "system_control":
      console.log(item);
      break;
    case "script":
      console.log(item);
      break;
    case "app":
      console.log(item);
      break;
  }
};

// Use preloaded system controls data
const getAssignedSystemControlId = (action: unknown) => {
  try {
    const parsed = typeof action === "string" ? JSON.parse(action) : action;
    if (!parsed || typeof parsed !== "object" || !("SystemControl" in parsed)) {
      return null;
    }
    const command = String(parsed.SystemControl);
    return command.split(":", 1)[0] || null;
  } catch {
    return null;
  }
};

const allSystemControlsWithButtons = computed(() => {
  if (!deckStore.activeDeck) return []; // If no active deck, return an empty array

  const systemControls = preloadedData.value.systemControls || [];

  // Retrieve all macros and buttons from the active deck
  const activeDeckButtons = deckStore.getActiveDeckById?.buttons || [];

  // Map through each macro and find the button with the matching action
  return systemControls.map((control) => {
    // Find the button whose action matches the macro ID
    const attachedButtons = activeDeckButtons.filter(
      (button) => getAssignedSystemControlId(button.action) === String(control.id),
    );

    // Add attached button positions to the macro
    const buttonPositions = attachedButtons
      .map((button) => `G${button.position}`)
      .join(", ");

    return {
      ...control,
      attachedButtons: buttonPositions, // Add the attachedButtons field as an array of button IDs and their positions
    };
  });
});

const savedActionGroups = computed(() => {
  if (!deckStore.activeDeck) return [];

  const savedActions = preloadedData.value.savedActions || {};
  const activeDeckButtons = deckStore.getActiveDeckById?.buttons || [];

  return Object.entries(savedActions)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([integrationId, providerActions]) => ({
      integrationId,
      label: `${integrationId.toUpperCase()} actions`,
      items: (Array.isArray(providerActions) ? providerActions : []).map((action) => ({
        ...action,
        attachedButtons: activeDeckButtons
          .filter(
            (button) =>
              button.action === JSON.stringify({ SavedActionId: action.stable_id }),
          )
          .map((button) => `G${button.position}`)
          .join(", "),
      })),
    }));
});

//TODO: MAKE IT ALLWAYS ON TOP
const allKeys = ref([
  // {
  //   name: "G-Switch",
  //   attachedButtons: "",
  // },
]);

const openMacro = () => {
  router.push({ name: "deck/new/macro", params: { macroId: 1 } });
};

// Use preloaded macros data
const allMacrosWithButtons = computed(() => {
  if (!deckStore.activeDeck) return []; // If no active deck, return an empty array

  const macros = preloadedData.value.macros || [];

  // Retrieve all macros and buttons from the active deck
  const activeDeckButtons = deckStore.getActiveDeckById?.buttons || [];

  // Map through each macro and find the button with the matching action
  return macros.map((macro) => {
    // Find the button whose action matches the macro ID
    const attachedButtons = activeDeckButtons.filter(
      (button) => button.action === `{"MacroId":${macro.id}}`
    );

    // Add attached button positions to the macro
    const buttonPositions = attachedButtons
      .map((button) => `G${button.position}`)
      .join(", ");

    return {
      ...macro,
      attachedButtons: buttonPositions, // Add the attachedButtons field as an array of button IDs and their positions
    };
  });
});

const removeMacro = async (id) => {
  try {
    console.log("Attempting to remove macro with ID:", id);

    // Call the backend to remove the macro
    const result = await invoke("remove_macro", { macroId: id });

    console.log(result); // Handle result

    // If successful, remove the macro from the preloaded data
    if (preloadedData.value.macros) {
      preloadedData.value.macros = preloadedData.value.macros.filter(
        (macro) => macro.id !== id
      );
    }

    // Optionally show a success message, refresh the UI, or take other actions
    console.log(`Successfully removed macro with ID: ${id}`);
  } catch (error) {
    console.error("Error removing macro:", error);
  }
};

const selectButton = (buttonId) => {
  selectedButtonId.value = buttonId.payload;
  console.log(buttonId.payload);
};

await listen("ms_select_button", selectButton);

// Listen for flow updates from flow editor
await listen("flows_updated", async (event) => {
  console.log("📡 Flows updated event received:", event);
  invalidateCache("flows");
});

// Listen for macro updates
await listen("macro_created", async () => {
  console.log("📡 Macro created event received");
  invalidateCache("macros");
});

await listen("macro_updated", async () => {
  console.log("📡 Macro updated event received");
  invalidateCache("macros");
});

// Listen for program updates
await listen("program_created", async () => {
  console.log("📡 Program created event received");
  invalidateCache("programs");
});

await listen("program_updated", async () => {
  console.log("📡 Program updated event received");
  invalidateCache("programs");
});

// Listen for HTTP request updates
await listen("http_request_created", async () => {
  console.log("📡 HTTP request created event received");
  invalidateCache("httpRequests");
});

await listen("http_request_updated", async () => {
  console.log("📡 HTTP request updated event received");
  invalidateCache("httpRequests");
});

// Listen for focus window updates
await listen("focus_window_created", async () => {
  console.log("📡 Focus window created event received");
  invalidateCache("focusWindows");
});

await listen("focus_window_updated", async () => {
  console.log("📡 Focus window updated event received");
  invalidateCache("focusWindows");
});

onMounted(async () => {
  await invoke("open_mobile_simulator");
});

const checkPremiumStatus = async () => {
  await invoke("check_premium_status");
};

// const setBrightness = async (brightness_level: number) => {
//   const result = await invoke("set_brightness", { brightness: brightness_level });
//   console.log(result);
// };

// Function to update button action

// Use preloaded flows data
const allFlowsWithButtons = computed(() => {
  const flows = preloadedData.value.flows || [];
  if (!Array.isArray(flows)) return [];

  // Retrieve all flows and buttons from the active deck
  const activeDeckButtons = deckStore.getActiveDeckById?.buttons || [];

  // Map through each flow and find the button with the matching action
  return flows.map((flow) => {
    // Find the button whose action matches the flow ID
    const attachedButtons = activeDeckButtons.filter(
      (button) => button.action === `{"FlowId":"${flow.id}"}`
    );

    // Add attached button positions to the flow
    const buttonPositions = attachedButtons
      .map((button) => `G${button.position}`)
      .join(", ");

    return {
      ...flow,
      attachedButtons: buttonPositions,
    };
  });
});

const formatFlowUpdatedAt = (updatedAt?: string) => {
  if (!updatedAt) return "Saved flow";
  const updatedDate = new Date(updatedAt);
  if (Number.isNaN(updatedDate.getTime())) return "Saved flow";

  const today = new Date();
  const startOfToday = new Date(
    today.getFullYear(),
    today.getMonth(),
    today.getDate(),
  );
  const startOfUpdatedDate = new Date(
    updatedDate.getFullYear(),
    updatedDate.getMonth(),
    updatedDate.getDate(),
  );
  const daysAgo = Math.round(
    (startOfToday.getTime() - startOfUpdatedDate.getTime()) / 86_400_000,
  );

  if (daysAgo === 0) return "Updated today";
  if (daysAgo === 1) return "Updated yesterday";
  return `Updated ${new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
  }).format(updatedDate)}`;
};

const assignFlowToButton = async (flow) => {
  try {
    if (!selectedButtonId.value) {
      mainStore.addNotification({
        title: "No button selected",
        message: "Please select a button first",
        type: "warning",
        autoClose: true,
      });
      return;
    }

    const result = await invoke("assign_flow_to_button", {
      buttonId: selectedButtonId.value,
      flowId: flow.id,
    });

    mainStore.addNotification({
      title: "Flow assigned",
      message: `Flow "${flow.name}" assigned to button successfully`,
      type: "success",
      autoClose: true,
    });

    // Refresh flows data
    retryLoad("flows");
  } catch (error) {
    console.error("Error assigning flow to button:", error);
    mainStore.addNotification({
      title: "Error assigning flow",
      message: error.toString(),
      type: "error",
      autoClose: true,
    });
  }
};

const removeFlow = async (flowId) => {
  try {
    // Find the flow before deletion for event payload
    const flowToDelete = preloadedData.value.flows?.find((flow) => flow.id === flowId);

    await invoke("delete_flow", { flowId });

    // Remove from preloaded data
    if (preloadedData.value.flows) {
      preloadedData.value.flows = preloadedData.value.flows.filter(
        (flow) => flow.id !== flowId
      );
    }

    // Emit event to flow editor to update flows list
    await emitTo("main", "flows_updated", { action: "deleted", flow: flowToDelete });

    mainStore.addNotification({
      title: "Flow deleted",
      message: `Flow deleted successfully`,
      type: "success",
      autoClose: true,
    });
  } catch (error) {
    console.error("Error deleting flow:", error);
    mainStore.addNotification({
      title: "Error deleting flow",
      message: "Failed to delete flow.",
      type: "error",
      autoClose: true,
    });
  }
};

// Use preloaded programs data with progressive icon loading
const allProgramsWithButtons = computed(() => {
  const programs = preloadedData.value.programs || [];
  if (!Array.isArray(programs)) return [];

  // Retrieve all programs and buttons from the active deck
  const activeDeckButtons = deckStore.getActiveDeckById?.buttons || [];

  // Map through each program and find the button with the matching action
  return programs.map((program) => {
    // Find the button whose action matches the program ID
    const attachedButtons = activeDeckButtons.filter(
      (button) => button.action === `{"Program":${program.id}}`
    );

    // Add attached button positions to the program
    const buttonPositions = attachedButtons
      .map((button) => `G${button.position}`)
      .join(", ");

    return {
      ...program,
      attachedButtons: buttonPositions || "No buttons",
    };
  });
});

const editProgram = (program) => {
  router.push({
    path: "/deck/new/program",
    query: { programId: program.id },
  });
};

// Context menu handlers
const handleProgramContextMenu = (event: MouseEvent, program) => {
  event.preventDefault();
  showContextMenu(event, [
    {
      id: 'edit',
      label: 'Edit',
      action: () => editProgram(program)
    },
    {
      id: 'copy',
      label: 'Copy/Duplicate',
      action: () => copyProgram(program)
    },
    {
      id: 'delete',
      label: 'Delete',
      action: () => removeProgram(program.id)
    }
  ]);
};

const handleMacroContextMenu = (event: MouseEvent, macro) => {
  event.preventDefault();
  showContextMenu(event, [
    {
      id: 'edit',
      label: 'Edit',
      action: () => router.push({path:'deck/new/macro', query: {macroId: macro.id, step: '1'}})
    },
    {
      id: 'copy',
      label: 'Copy/Duplicate',
      action: () => copyMacro(macro)
    },
    {
      id: 'delete',
      label: 'Delete',
      action: () => removeMacro(macro.id)
    }
  ]);
};

const handleFlowContextMenu = (event: MouseEvent, flow) => {
  event.preventDefault();
  showContextMenu(event, [
    {
      id: 'edit',
      label: 'Edit',
      action: () => openFlowInEditor(flow)
    },
    {
      id: 'copy',
      label: 'Copy/Duplicate',
      action: () => copyFlow(flow)
    },
    {
      id: 'delete',
      label: 'Delete',
      action: () => removeFlow(flow.id)
    }
  ]);
};

// Copy handlers
const copyProgram = (program) => {
  router.push({
    path: "/deck/new/program",
    query: { copyFrom: program.id },
  });
};

const copyMacro = (macro) => {
  router.push({
    path: "/deck/new/macro",
    query: { copyFrom: macro.id, step: '1' },
  });
};

const copyFlow = (flow) => {
  // For flows, we'll copy by opening the flow editor and loading the flow data
  // The flow editor will need to handle copyFrom parameter
  router.push({
    path: "/deck/new/flow",
    query: { copyFrom: flow.id },
  });
};

// Icon base URL (same as simulator)
const iconUrlBase = ref("");

// Get custom icon URL from HTTP service
const getCustomIconUrl = (program) => {
  if (!iconUrlBase.value || !program.custom_icon) return null;
  // Add cache buster to force reload when icon changes
  const timestamp = Date.now();
  return `${iconUrlBase.value}program_${program.id}/icon.webp?t=${timestamp}`;
};

// Initialize icon base URL
onMounted(async () => {
  try {
    const serverInfo = await invoke("get_server_info_new");

    if (serverInfo?.ip && serverInfo?.iconsServerPort) {
      iconUrlBase.value = `http://${serverInfo.ip}:${serverInfo.iconsServerPort}/icons/`;
    }
  } catch (error) {
    console.error("Failed to get server info:", error);
  }
});

const removeProgram = async (programId) => {
  try {
    // Remove from database first
    await invoke("remove_program_from_db", { programId: programId });

    // Then remove from UI
    if (preloadedData.value.programs) {
      preloadedData.value.programs = preloadedData.value.programs.filter(
        (program) => program.id !== programId
      );
    }


    // Clean up icon in background
    invoke("remove_icon", {
      buttonId: `program_${programId}`,
    }).catch((error) => {
      console.error("Error removing program icon:", error);
    });

    mainStore.addNotification({
      title: "Program deleted",
      message: "Program deleted successfully",
      type: "success",
      autoClose: true,
    });
  } catch (error) {
    console.error("Error deleting program:", error);
    mainStore.addNotification({
      title: "Error deleting program",
      message: "Failed to delete program.",
      type: "error",
      autoClose: true,
    });
  }
};

// HTTP Requests (in Utilities submenu)
const allHttpRequestsWithButtons = computed(() => {
  const httpRequests = preloadedData.value.httpRequests || [];
  if (!Array.isArray(httpRequests)) return [];

  const activeDeckButtons = deckStore.getActiveDeckById?.buttons || [];

  return httpRequests.map((request) => {
    const attachedButtons = activeDeckButtons.filter(
      (button) => button.action === `{"HttpRequestId":${request.id}}`
    );

    const buttonPositions = attachedButtons
      .map((button) => `G${button.position}`)
      .join(", ");

    return {
      ...request,
      attachedButtons: buttonPositions || "No buttons",
    };
  });
});

const editHttpRequest = (request) => {
  router.push({
    path: "/deck/new/http-request",
    query: { requestId: request.id },
  });
};

const removeHttpRequest = async (item) => {
  try {
    await invoke("delete_http_request", { requestId: item.id });

    if (preloadedData.value.httpRequests) {
      preloadedData.value.httpRequests = preloadedData.value.httpRequests.filter(
        (request) => request.id !== item.id
      );
    }

    mainStore.addNotification({
      title: "HTTP Request deleted",
      message: `${item.name} deleted successfully`,
      type: "success",
      autoClose: true,
    });
  } catch (error) {
    console.error("Error deleting HTTP request:", error);
    mainStore.addNotification({
      title: "Error deleting HTTP request",
      message: "Failed to delete HTTP request.",
      type: "error",
      autoClose: true,
    });
  }
};

const getMethodClass = (method) => {
  const classes = {
    GET: "bg-blue-500/20 text-blue-400 border border-blue-500/30",
    POST: "bg-green-500/20 text-green-400 border border-green-500/30",
    PUT: "bg-yellow-500/20 text-yellow-400 border border-yellow-500/30",
    PATCH: "bg-purple-500/20 text-purple-400 border border-purple-500/30",
    DELETE: "bg-red-500/20 text-red-400 border border-red-500/30",
  };
  return classes[method] || "bg-gray-500/20 text-gray-400 border border-gray-500/30";
};

// Focus Windows
const allFocusWindowsWithButtons = computed(() => {
  const focusWindows = preloadedData.value.focusWindows || [];
  if (!Array.isArray(focusWindows)) return [];

  const activeDeckButtons = deckStore.getActiveDeckById?.buttons || [];

  return focusWindows.map((window) => {
    const attachedButtons = activeDeckButtons.filter(
      (button) => button.action === `{"FocusWindowId":${window.id}}`
    );

    const buttonPositions = attachedButtons
      .map((button) => `G${button.position}`)
      .join(", ");

    return {
      ...window,
      attachedButtons: buttonPositions || "No buttons",
    };
  });
});

const removeFocusWindow = async (item) => {
  try {
    await invoke("delete_focus_window", { id: item.id });

    if (preloadedData.value.focusWindows) {
      preloadedData.value.focusWindows = preloadedData.value.focusWindows.filter(
        (window) => window.id !== item.id
      );
    }

    mainStore.addNotification({
      title: "Focus window deleted",
      message: `${item.name} deleted successfully`,
      type: "success",
      autoClose: true,
    });
  } catch (error) {
    console.error("Error deleting focus window:", error);
    mainStore.addNotification({
      title: "Error deleting focus window",
      message: "Failed to delete focus window.",
      type: "error",
      autoClose: true,
    });
  }
};

</script>

<style lang="scss" scoped>
.deck-editor-page {
  --deck-bg: hsl(var(--background) / 1);
  --deck-surface: hsl(var(--editor-surface, 0 0% 100%) / 1);
  --deck-surface-soft: hsl(var(--editor-surface-soft, 210 33% 98%) / 1);
  --deck-surface-muted: hsl(var(--editor-surface-muted, 210 20% 94%) / 1);
  --deck-border: hsl(var(--border2) / 0.72);
  --deck-muted: hsl(var(--text2) / 1);
  --deck-text: hsl(var(--text) / 1);
  --deck-accent: hsl(var(--button));
  --deck-accent-2: hsl(var(--buttonHover));
  color: var(--deck-text);
}

.assignment-tabs {
  border-bottom: 1px solid var(--deck-border);
  padding-bottom: 0.35rem;
  scrollbar-width: none;

  &::-webkit-scrollbar {
    display: none;
  }
}

.assignment-tab {
  flex: 0 0 auto;
  white-space: nowrap;
  font-size: 0.7rem;
  color: var(--deck-muted);
  padding: 0.35rem 0.16rem;
  border-bottom: 2px solid transparent;
  transition: color 0.2s ease, border-color 0.2s ease;
}

.assignment-tab:hover {
  color: var(--deck-text);
}

.assignment-tab--active {
  color: var(--deck-accent);
  border-bottom-color: var(--deck-accent);
}

.assignment-list {
  padding-right: 0.1rem;
}

.assignment-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.55rem 0.55rem;
  border: 1px solid var(--deck-border);
  border-radius: 0.7rem;
  background: var(--deck-surface-soft);
  transition: background 0.2s ease, border-color 0.2s ease;
  margin-bottom: 0.35rem;
  position: relative;
}

.assignment-row:hover {
  background: var(--deck-surface-muted);
  border-color: var(--deck-border);
}

.assignment-icon {
  height: 1.85rem;
  width: 1.85rem;
  min-width: 1.85rem;
  border-radius: 0.5rem;
  background: var(--deck-surface-muted);
  border: 1px solid var(--deck-border);
  color: var(--deck-text);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.76rem;
  font-weight: 700;
  flex-shrink: 0;
  overflow: hidden;

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  svg {
    width: 1rem;
    height: 1rem;
  }
}

.assignment-icon--key {
  background: hsl(var(--primary) / 0.12);
  border-color: hsl(var(--primary) / 0.25);
  color: hsl(var(--primary));
}

.assignment-icon--macro {
  background: hsl(var(--secondary) / 0.12);
  border-color: hsl(var(--secondary) / 0.25);
  color: hsl(var(--secondary));
}

.assignment-icon--flow {
  background: hsl(var(--primary) / 0.12);
  border-color: hsl(var(--primary) / 0.25);
  color: hsl(var(--primary));
}

.assignment-name {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.86rem;
  font-weight: 600;
  color: var(--deck-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.assignment-attached {
  margin-left: auto;
  max-width: 30%;
  text-wrap: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 0.7rem;
  font-weight: 700;
  color: var(--deck-muted);
}

.deck-panel-resizer {
  position: relative;
  width: 0.75rem;
  height: 100%;
  flex: 0 0 0.75rem;
  cursor: col-resize;
  touch-action: none;
  background: transparent;
  outline: none;
}

.deck-panel-resizer::before {
  content: "";
  position: absolute;
  inset: 0 0.2rem;
  border-radius: 999px;
  background: transparent;
  transition: background 0.15s ease;
}

.deck-panel-resizer::after {
  content: "";
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 1px;
  transform: translateX(-50%);
  background: hsl(var(--border2) / 0.72);
  transition: width 0.15s ease, background 0.15s ease;
}

.deck-panel-resizer:hover::before,
.deck-panel-resizer:focus-visible::before,
.deck-panel-resizer--active::before {
  background: hsl(var(--button) / 0.08);
}

.deck-panel-resizer:hover::after,
.deck-panel-resizer:focus-visible::after,
.deck-panel-resizer--active::after {
  width: 2px;
  background: hsl(var(--button) / 0.7);
}

.deck-primary-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.75rem;
  padding: 0.55rem 0.9rem;
  font-size: 0.8rem;
  font-weight: 700;
  color: white;
  background: hsl(var(--button));
  box-shadow: 0 4px 12px hsl(var(--button) / 0.18);
  transition: transform 0.15s ease, background-color 0.15s ease, box-shadow 0.15s ease;

  &:hover {
    background: hsl(var(--buttonHover));
    box-shadow: 0 6px 16px hsl(var(--button) / 0.28);
    transform: translateY(-1px);
  }
  &:active {
    transform: translateY(0);
  }
}

.deck-mini-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.24rem;
  border-radius: 0.55rem;
  padding: 0.3rem 0.62rem;
  color: var(--deck-text);
  border: 1px solid var(--deck-border);
  background: var(--deck-surface);
  font-size: 0.76rem;
  font-weight: 700;
  transition: background 0.2s ease, border-color 0.2s ease, color 0.2s ease;
}

.deck-mini-btn:hover {
  background: hsl(var(--editor-accent) / 0.08);
  border-color: hsl(var(--editor-accent) / 0.5);
  color: var(--deck-accent);
}

.deck-mini-btn :deep(svg) {
  width: 0.86rem;
  height: 0.86rem;
}

.submenu-context-icon {
  width: 1.55rem;
  height: 1.55rem;
  min-width: 1.55rem;
  min-height: 1.55rem;
  flex: 0 0 1.55rem;
  aspect-ratio: 1 / 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.24rem;
  border-radius: 999px;
  border: 1px solid var(--deck-border);
  background: var(--deck-surface);
  color: var(--deck-text);
  overflow: hidden;
}

.submenu-context-icon--padded {
  padding: 0.2rem;
}

.submenu-context-label {
  flex: 1 1 auto;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.submenu-built-in-badge {
  margin-left: auto;
  padding: 0.2rem 0.45rem;
  border: 1px solid hsl(var(--border2));
  border-radius: 999px;
  background: hsl(var(--background));
  color: hsl(var(--text3));
  font-size: 0.6rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.obs-status-warning {
  position: relative;
  display: inline-flex;
  align-items: center;
  margin-left: 0.45rem;
  color: hsl(var(--error));
}

.obs-offline-badge {
  margin-left: 0.25rem;
  font-size: 0.58rem;
  font-weight: 750;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.obs-status-tooltip {
  position: absolute;
  z-index: 40;
  top: calc(100% + 0.45rem);
  right: 0;
  width: max-content;
  max-width: 14rem;
  padding: 0.45rem 0.6rem;
  border: 1px solid hsl(var(--border2));
  border-radius: 0.5rem;
  background: hsl(var(--background3));
  box-shadow: 0 0.55rem 1.4rem hsl(var(--text) / 0.14);
  color: hsl(var(--text));
  font-size: 0.7rem;
  font-weight: 500;
  line-height: 1.35;
  opacity: 0;
  pointer-events: none;
  transform: translateY(-0.2rem);
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.obs-status-warning:hover .obs-status-tooltip,
.obs-status-warning:focus-within .obs-status-tooltip {
  opacity: 1;
  transform: translateY(0);
}

.configuration-shell {
  background: var(--deck-surface);
}

.config-menu-shell {
  position: sticky;
  top: 0;
  z-index: 5;
  background: var(--deck-surface);
  padding: 0.95rem 1.35rem 0;
  border-bottom: 1px solid var(--deck-border);
}

.config-menu {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0 0 0.45rem;
  width: fit-content;
}

.config-menu-tab {
  border: none;
  background: transparent;
  font-size: 0.8rem;
  color: var(--deck-muted);
  padding: 0.22rem 0.02rem 0.45rem;
  border-bottom: 2px solid transparent;
  font-weight: 600;
  transition: color 0.2s ease, border-color 0.2s ease;
}

.config-menu-tab:hover {
  color: var(--deck-text);
}

.config-menu-tab--active {
  color: var(--deck-accent);
  border-bottom-color: var(--deck-accent);
}

.config-body {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  padding: 1rem 1.35rem 1.35rem;
}

.config-section {
  border: 1px solid var(--deck-border);
  background: var(--deck-surface);
  border-radius: 0.85rem;
  padding: 0.95rem 1rem;
}

.config-label {
  font-size: 0.68rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--deck-muted);
  font-weight: 700;
  margin-bottom: 0.6rem;
}

.input-mode-card {
  border: 1px solid hsl(var(--border2) / 0.72);
  border-radius: 0.75rem;
  padding: 0.8rem;
  background: var(--deck-surface);
  min-height: 8.1rem;
}

.input-mode-card--active {
  border-color: var(--deck-accent);
  background: var(--deck-surface);
  box-shadow: none;
}

.input-mode-icon {
  width: 3rem;
  height: 3rem;
  border-radius: 999px;
  border: 1px solid hsl(var(--border2) / 0.72);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--deck-muted);
  background: var(--deck-surface);
}

.config-field {
  border: 1px solid var(--deck-border);
  border-radius: 0.6rem;
  background: var(--deck-surface);
  padding: 0.5rem 0.65rem;
  margin-top: 0.55rem;
}

.config-field-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.65rem;
}

.config-field-label {
  display: block;
  font-size: 0.67rem;
  color: var(--deck-muted);
  margin-bottom: 0.25rem;
  font-weight: 700;
}

.config-field-value {
  font-size: 0.8rem;
  color: var(--deck-text);
  min-height: 1.65rem;
  display: flex;
  align-items: center;
}

.color-dot {
  width: 1rem;
  height: 1rem;
  border-radius: 999px;
  box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.75);
}

.sequence-list {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.sequence-item {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  border: 1px solid var(--deck-border);
  background: var(--deck-surface);
  border-radius: 0.7rem;
  padding: 0.6rem;
}

.sequence-dot {
  width: 1.35rem;
  height: 1.35rem;
  border-radius: 999px;
  color: var(--deck-text);
  font-size: 0.62rem;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.sequence-title {
  font-size: 0.7rem;
  text-transform: uppercase;
  font-weight: 800;
  letter-spacing: 0.03em;
  color: var(--deck-muted);
}

.sequence-subtitle {
  font-size: 0.8rem;
  color: var(--deck-text);
  margin-top: 0.08rem;
}

.sequence-add-btn {
  border: 1px dashed var(--deck-border);
  border-radius: 0.6rem;
  color: var(--deck-muted);
  background: var(--deck-surface-soft);
  padding: 0.55rem;
  font-size: 0.78rem;
  font-weight: 600;
}

.configuration-shell,
.assignment-list {
  scrollbar-gutter: stable;

  &::-webkit-scrollbar {
    width: 6px;
    height: 6px;
    display: block;
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  &::-webkit-scrollbar-thumb {
    background: hsl(var(--border2));
    border-radius: 999px;
  }
  &::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--text3) / 0.5);
  }
  &::-webkit-scrollbar-button {
    display: none !important;
    width: 0 !important;
    height: 0 !important;
  }
}
</style>
