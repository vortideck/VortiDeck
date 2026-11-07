<template lang="pug">
  div(class="flow-header px-4 pt-3 pb-2.5 z-[1000] border-b border-themeBorder bg-themeBackground" data-tauri-drag-region)
    div(class="flow-header-inner flex justify-between items-center")
      // Left side - Flow identification
      div(class="flow-header-no-drag flex items-center gap-3")
        div(class="w-8 h-8 rounded-xl bg-themePrimary/12 border border-themePrimary/25 flex items-center justify-center text-themePrimary shrink-0 shadow-2xs")
          HugeiconsAiBrain02(class="w-4.5 h-4.5")
        
        div(class="flex flex-col")
          div(class="flex items-center gap-2")
            h1(
              class="text-sm font-bold text-themeText tracking-tight cursor-pointer hover:text-themePrimary transition-colors flex items-center gap-1.5"
              @click="editFlowInfo"
              title="Click to edit flow name and description"
            )
              span {{ currentFlowName }}
              SolarPenBold(class="w-3 h-3 text-themeText3 opacity-60 hover:opacity-100")
            
            span(
              class="px-2 py-0.5 rounded-full text-[10px] font-semibold border"
              :class="isNewFlow ? 'bg-themePrimary/12 text-themePrimary border-themePrimary/25' : hasUnsavedChanges ? 'bg-amber-500/15 text-amber-400 border-amber-500/25' : 'bg-emerald-500/12 text-emerald-500 border-emerald-500/25'"
            )
              span(v-if="isNewFlow") New Flow
              span(v-else-if="hasUnsavedChanges") Unsaved Changes
              span(v-else) Saved
          
          span(class="text-[11px] text-themeText3") Visual Logic & Automation Node Graph
      
      // Right side - Actions
      div(class="flow-header-no-drag flex items-center gap-2")
        // Primary Execute action
        button(
          @click="executeFlowWithUnified"
          :disabled="isFlowRunning || flowStartPending"
          class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all shadow-xs cursor-pointer"
          :class="isFlowRunning || flowStartPending ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30 cursor-not-allowed' : 'bg-themeButton hover:bg-themeButtonHover text-white'"
        )
          SolarPlayBold(v-if="!isFlowRunning && !flowStartPending" class="w-3.5 h-3.5")
          EvaLoaderOutline(v-else class="w-3.5 h-3.5 animate-spin")
          span {{ flowStartPending ? 'Checking...' : isFlowRunning ? 'Executing...' : 'Run Flow' }}
        
        // Cancel button (only when running)
        button(
          v-if="activeFlowExecution && isFlowRunning"
          class="flex items-center gap-1.5 px-3 py-1.5 bg-red-500/15 text-red-400 border border-red-500/30 hover:bg-red-500/25 rounded-xl text-xs font-semibold transition-colors cursor-pointer"
          @click="cancelFlowUnified"
        )
          EvaCloseCircleOutline(class="w-3.5 h-3.5")
          span Cancel
        
        // Save button
        button(
          class="flex items-center gap-1.5 px-3 py-1.5 bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder rounded-xl text-xs font-semibold transition-colors shadow-2xs cursor-pointer"
          @click="saveFlow"
        )
          HugeiconsFloppyDisk(class="w-3.5 h-3.5 text-themeText2")
          span Save
        
        // Actions dropdown
        div(class="flow-header-actions-menu relative")
          button(
            class="flex items-center justify-center w-8 h-8 bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder rounded-xl text-xs font-semibold transition-colors shadow-2xs cursor-pointer"
            @click="toggleActionsMenu"
          )
            HugeiconsMoreHorizontal(class="w-4 h-4 text-themeText2")
          
          // Actions dropdown menu
          div(
            v-if="showActionsMenu"
            class="absolute right-0 top-full mt-2 w-48 rounded-2xl bg-themeBackground2 border border-themeBorder shadow-xl z-50 overflow-hidden py-1.5"
            @click.stop=""
          )
            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="newFlow"
            )
              EvaFileTextOutline(class="w-4 h-4 text-themeText3")
              span New Flow
            
            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="saveFlowAs"
            )
              EvaCopyOutline(class="w-4 h-4 text-themeText3")
              span Save As...
            
            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="editFlowInfo"
            )
              SolarPenBold(class="w-3.5 h-3.5 text-themeText3")
              span Edit Flow Details
            
            div(class="border-t border-themeBorder my-1")

            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="showRuntimeOperations"
            )
              HugeiconsActivity03(class="w-4 h-4 text-themeText3")
              span Runtime diagnostics
              span(
                v-if="recoveryQueueCount > 0"
                class="ml-auto rounded-full bg-amber-500/20 px-2 py-0.5 text-[10px] font-bold text-amber-300"
                :aria-label="`${recoveryQueueCount} workflow runs require recovery review`"
              ) {{ recoveryQueueCount }}

            div(class="border-t border-themeBorder my-1")

            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="rehearseFlow"
            )
              HugeiconsTestTube01(class="w-4 h-4 text-themeText3")
              span Rehearse safely

            div(class="border-t border-themeBorder my-1")
            
            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="exportFlowToFile"
            )
              EvaUploadOutline(class="w-4 h-4 text-themeText3")
              span Export JSON

            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="exportRecipe"
            )
              EvaCopyOutline(class="w-4 h-4 text-themeText3")
              span Export portable recipe
            
            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-themeText2 hover:text-themeText hover:bg-themeBackground3 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="triggerImportFlow"
            )
              EvaDownloadOutline(class="w-4 h-4 text-themeText3")
              span Import JSON
            
            div(class="border-t border-themeBorder my-1")
            
            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-red-400 hover:text-red-300 hover:bg-red-500/10 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="deleteSelection"
            )
              EvaTrash2Outline(class="w-4 h-4 text-red-400")
              span Delete Selected
            
            button(
              class="w-full px-3.5 py-2 text-left text-xs font-medium text-red-400 hover:text-red-300 hover:bg-red-500/10 flex items-center gap-2.5 transition-colors cursor-pointer"
              @click="clearAll"
            )
              EvaCloseCircleOutline(class="w-4 h-4 text-red-400")
              span Clear Canvas
        
        // Settings toggle button
        button(
          id="settings-button"
          class="flex items-center gap-1.5 px-3 py-1.5 bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder rounded-xl text-xs font-semibold transition-colors shadow-2xs cursor-pointer"
          :class="showSettings ? 'border-themePrimary text-themePrimary bg-themePrimary/10' : ''"
          @click.stop="toggleSettings"
        )
          HugeiconsSettings01(class="w-3.5 h-3.5 text-themeText2")
          span Settings

  // Hidden file input
  input(
    class="flow-header-no-drag"
    ref="fileInput"
    type="file"
    accept=".json,.vdrecipe.json"
    style="display: none"
    @change="handleFileImport"
  )
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import HugeiconsAiBrain02 from "~icons/hugeicons/ai-brain-02";
import SolarPlayBold from "~icons/solar/play-bold";
import HugeiconsFloppyDisk from "~icons/hugeicons/floppy-disk";
import HugeiconsMoreHorizontal from "~icons/hugeicons/more-horizontal";
import HugeiconsSettings01 from "~icons/hugeicons/settings-01";
import HugeiconsActivity03 from "~icons/hugeicons/activity-03";
import HugeiconsTestTube01 from "~icons/hugeicons/test-tube-01";
import SolarPenBold from "~icons/solar/pen-bold";
import EvaFileTextOutline from "~icons/eva/file-text-outline";
import EvaCopyOutline from "~icons/eva/copy-outline";
import EvaTrash2Outline from "~icons/eva/trash-2-outline";
import EvaCloseCircleOutline from "~icons/eva/close-circle-outline";
import EvaUploadOutline from "~icons/eva/upload-outline";
import EvaDownloadOutline from "~icons/eva/download-outline";
import EvaLoaderOutline from "~icons/eva/loader-outline";

// Props
interface Props {
  showSettings: boolean;
  nodes?: any[];
  connections?: any[];
  activeFlowExecution?: any;
  isFlowRunning?: boolean;
  flowStartPending?: boolean;
  camera?: { x: number; y: number; scale: number };
  selectedNodes?: Set<number>;
  selectedNode?: any;
  currentFlowName?: string;
  hasUnsavedChanges?: boolean;
  isNewFlow?: boolean;
  recoveryQueueCount?: number;
}

const props = withDefaults(defineProps<Props>(), {
  nodes: () => [],
  connections: () => [],
  activeFlowExecution: null,
  isFlowRunning: false,
  flowStartPending: false,
  camera: () => ({ x: 0, y: 0, scale: 1 }),
  selectedNodes: () => new Set(),
  selectedNode: null,
  currentFlowName: "Untitled Flow",
  hasUnsavedChanges: false,
  isNewFlow: true,
  recoveryQueueCount: 0,
});

// Emits
const emit = defineEmits<{
  toggleSettings: [];
  executeFlow: [];
  cancelFlow: [];
  deleteSelection: [];
  clearAll: [];
  exportFlow: [];
  exportRecipe: [];
  rehearseFlow: [];
  importFlow: [file: File];
  saveFlow: [];
  saveFlowAs: [];
  newFlow: [];
  editFlowInfo: [];
  showRuntimeOperations: [];
}>();

// Local state
const fileInput = ref<HTMLInputElement | null>(null);
const showActionsMenu = ref(false);

// Computed
const currentFlowName = computed(() => {
  return props.currentFlowName || "Untitled Flow";
});

// Methods
const toggleSettings = () => {
  emit("toggleSettings");
  showActionsMenu.value = false;
};

const toggleActionsMenu = () => {
  showActionsMenu.value = !showActionsMenu.value;
};

const deleteSelection = () => {
  emit("deleteSelection");
  showActionsMenu.value = false;
};

const clearAll = () => {
  emit("clearAll");
  showActionsMenu.value = false;
};

const triggerImportFlow = () => {
  if (fileInput.value) {
    fileInput.value.click();
  }
  showActionsMenu.value = false;
};

const handleFileImport = async (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];

  if (!file) {
    console.warn("⚠️ No file selected");
    return;
  }

  if (!file.name.endsWith(".json")) {
    alert("Please select a JSON file");
    return;
  }

  emit("importFlow", file);

  if (fileInput.value) {
    fileInput.value.value = "";
  }
};

const exportFlowToFile = () => {
  emit("exportFlow");
  showActionsMenu.value = false;
};

const exportRecipe = () => {
  emit("exportRecipe");
  showActionsMenu.value = false;
};

const rehearseFlow = () => {
  emit("rehearseFlow");
  showActionsMenu.value = false;
};

const saveFlow = () => {
  emit("saveFlow");
};

const saveFlowAs = () => {
  emit("saveFlowAs");
  showActionsMenu.value = false;
};

const newFlow = () => {
  emit("newFlow");
  showActionsMenu.value = false;
};

const editFlowInfo = () => {
  emit("editFlowInfo");
  showActionsMenu.value = false;
};

const showRuntimeOperations = () => {
  emit("showRuntimeOperations");
  showActionsMenu.value = false;
};

const executeFlowWithUnified = async () => {
  emit("executeFlow");
};

const cancelFlowUnified = () => {
  emit("cancelFlow");
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as Element;
  if (!target.closest(".flow-header-actions-menu")) {
    showActionsMenu.value = false;
  }
};

onMounted(() => {
  document.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", handleClickOutside);
});
</script>

<style lang="scss" scoped>
.flow-header-no-drag,
.flow-header-no-drag * {
  -webkit-app-region: no-drag;
  app-region: no-drag;
}
</style>
