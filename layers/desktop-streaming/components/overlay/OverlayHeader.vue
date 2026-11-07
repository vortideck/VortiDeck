<template lang="pug">
header(class="h-10 bg-themeBackground border-b border-themeBorder px-3 flex items-center justify-between flex-shrink-0 relative z-30")
  //- Left side
  div(class="flex items-center gap-3")
    //- Title
    div(class="flex items-center gap-2")
      span(class="text-themeText font-semibold text-sm") Overlay Editor

    //- Separator
    div(class="h-4 w-px bg-themeBorder")

    //- Project dropdown
    div(class="relative project-dropdown")
      button(
        @click="showProjectDropdown = !showProjectDropdown"
        class="flex items-center gap-1.5 px-2.5 py-1 bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 rounded-md text-xs text-themeText transition-colors"
      )
        span(class="text-themeText") {{ currentProject?.name || 'No Project' }}
        span(v-if="hasUnsavedChanges" class="w-1.5 h-1.5 bg-orange-400 rounded-full" title="Unsaved changes")
        svg(class="w-3.5 h-3.5 text-themeText2" :class="showProjectDropdown ? 'rotate-180' : ''" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7")
      
      //- Dropdown menu
      div(
        v-if="showProjectDropdown"
        class="absolute top-full left-0 mt-1 w-80 bg-themeBackground2 border border-themeBorder2 rounded-lg shadow-lg z-50 text-themeText"
        @click.stop
      )
        div(class="p-3 border-b border-themeBorder")
          div(class="flex items-center justify-between mb-2")
            //- Project name (editable when clicked)
            div(v-if="!isEditingProjectName" @click="startEditingProjectName" class="text-sm font-medium text-themeText hover:text-blue-500 cursor-pointer transition-colors") {{ currentProject?.name || 'No Project' }}
            input(
              v-else
              v-model="editingProjectName"
              @keyup.enter="saveProjectName"
              @keyup.escape="cancelEditingProjectName"
              @blur="saveProjectName"
              ref="projectNameInput"
              class="text-sm font-medium text-themeText bg-transparent border-b border-blue-500 outline-none focus:border-blue-600 transition-colors"
            )
            span(v-if="hasUnsavedChanges" class="flex items-center gap-1 text-xs text-orange-400")
              div(class="w-1.5 h-1.5 bg-orange-400 rounded-full")
              | Unsaved changes
          
          div(v-if="lastSaved" class="text-xs text-themeText2")
            | Last saved {{ new Date(lastSaved).toLocaleTimeString() }}
          
          div(class="flex items-center justify-between mt-2")
            span(class="text-xs text-themeText2") Auto-save
            button(
              @click="autoSaveEnabled = !autoSaveEnabled"
              class="text-xs px-2 py-1 rounded font-medium"
              :class="autoSaveEnabled ? 'bg-green-500/20 text-green-400 border border-green-500/30' : 'bg-themeBackground3 text-themeText2'"
            ) {{ autoSaveEnabled ? 'ON' : 'OFF' }}
        
        div(class="p-2")
          button(
            @click="$emit('save-project'); showProjectDropdown = false"
            class="w-full flex items-center gap-2 px-2.5 py-1.5 text-xs text-themeText hover:bg-themeBackground3 rounded transition-colors"
          )
            svg(class="w-3.5 h-3.5 text-themeText2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4")
            span Save Project
          
          button(
            @click="$emit('create-project'); showProjectDropdown = false"
            class="w-full flex items-center gap-2 px-2.5 py-1.5 text-xs text-themeText hover:bg-themeBackground3 rounded transition-colors"
          )
            svg(class="w-3.5 h-3.5 text-themeText2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4")
            span New Project
          
          button(
            @click="$emit('show-project-manager'); showProjectDropdown = false"
            class="w-full flex items-center gap-2 px-2.5 py-1.5 text-xs text-themeText hover:bg-themeBackground3 rounded transition-colors"
          )
            svg(class="w-3.5 h-3.5 text-themeText2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16")
            span Manage Projects

    //- Separator
    div(class="h-4 w-px bg-themeBorder")

    //- Status badges
    div(class="flex items-center gap-2")
      //- Server Status
      div(class="flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium border" :class="serverStatus.running ? 'bg-green-500/10 text-green-500 border-green-500/20' : 'bg-themeBackground2 text-themeText2 border-themeBorder'")
        div(class="w-1.5 h-1.5 rounded-full" :class="serverStatus.running ? 'bg-green-500' : 'bg-neutral-400'")
        span(v-if="serverLoading") Starting...
        span(v-else-if="serverStatus.running") Server
        span(v-else) Offline

      //- OBS Status
      div(class="flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium border" :class="obsStatus.connected ? 'bg-green-500/10 text-green-500 border-green-500/20' : 'bg-themeBackground2 text-themeText2 border-themeBorder'")
        div(class="w-1.5 h-1.5 rounded-full" :class="obsStatus.connected ? 'bg-green-500' : 'bg-neutral-400'")
        span(v-if="obsLoading") Connecting...
        span(v-else-if="obsStatus.connected")
          span(v-if="obsStatus.streaming") Streaming
          span(v-else-if="obsStatus.recording") Recording
          span(v-else) OBS
        span(v-else) No OBS
  
  //- Right side
  div(class="flex items-center gap-2")
    //- Undo/Redo
    div(class="flex items-center gap-0.5 px-1 py-0.5 bg-themeBackground2 border border-themeBorder rounded-lg")
      button(@click="$emit('undo')" :disabled="!canUndo" class="p-1 hover:bg-themeBackground3 text-themeText2 hover:text-themeText rounded disabled:opacity-40 disabled:cursor-not-allowed transition-colors cursor-pointer" title="Undo (Ctrl+Z)")
        svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6")
      button(@click="$emit('redo')" :disabled="!canRedo" class="p-1 hover:bg-themeBackground3 text-themeText2 hover:text-themeText rounded disabled:opacity-40 disabled:cursor-not-allowed transition-colors cursor-pointer" title="Redo (Ctrl+Y)")
        svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6")

    //- Grid toggle
    button(@click="$emit('toggle-grid')" :class="showGrid ? 'bg-themePrimary/15 text-themePrimary border-themePrimary/30' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2 border-transparent'" class="p-1.5 rounded-lg border transition-colors cursor-pointer" title="Toggle Grid")
      svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z")

    //- Snap to grid
    button(@click="$emit('toggle-snap-to-grid')" :class="snapToGrid ? 'bg-themePrimary/15 text-themePrimary border-themePrimary/30' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2 border-transparent'" class="p-1.5 rounded-lg border transition-colors cursor-pointer" title="Snap to Grid")
      svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 8l4-4m0 0l-4-4m4 4H3")

    //- Toggle preview elements
    button(@click="$emit('toggle-preview-elements')" :class="showPreviewElements ? 'bg-themePrimary/15 text-themePrimary border-themePrimary/30' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2 border-transparent'" class="p-1.5 rounded-lg border transition-colors cursor-pointer" title="Toggle Preview Elements")
      svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z")

    button(@click="$emit('open-settings')" class="p-1.5 text-themeText2 hover:text-themeText hover:bg-themeBackground2 rounded-lg transition-colors cursor-pointer" title="Overlay & OBS Settings")
      svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z")

    //- Separator
    div(class="h-4 w-px bg-themeBorder")

    //- OBS Controls (compact)
    div(class="flex items-center gap-1.5")
      button(
        @click="$emit('sync-obs')"
        :disabled="!obsStatus.connected || obsLoading"
        class="px-3 py-1 bg-themeBackground2 hover:bg-themeBackground3 text-themeText border border-themeBorder rounded-lg text-xs font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed relative cursor-pointer"
        :title="autoSyncEnabled ? 'Manual sync (Auto-sync via OBS events)' : 'Sync canvas with OBS'"
      )
        span(v-if="!obsLoading") {{ autoSyncEnabled ? 'Sync' : 'Sync OBS' }}
        span(v-else) ...
        //- Auto-sync indicator
        span(v-if="autoSyncEnabled" class="absolute -top-0.5 -right-0.5 w-1.5 h-1.5 bg-green-500 rounded-full animate-pulse" title="Auto-sync enabled")

      button(
        @click="$emit('export-obs')"
        :disabled="exportLoading"
        class="px-3.5 py-1 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed shadow-xs cursor-pointer"
        title="Publish the saved draft and send its browser source to OBS"
      )
        span(v-if="!exportLoading") Publish
        span(v-else) .....
</template>

<script setup lang="ts">
import { ref, computed, nextTick } from "vue";

interface Project {
  id: string;
  name: string;
}

interface ServerStatus {
  running: boolean;
  url: string;
}

interface OBSStatus {
  connected: boolean;
  streaming: boolean;
  recording: boolean;
  canvasWidth?: number;
  canvasHeight?: number;
}

const props = withDefaults(
  defineProps<{
    currentProject?: Project | null;
    hasUnsavedChanges?: boolean;
    lastSaved?: string | number | null;
    autoSaveEnabled?: boolean;
    showPreviewElements?: boolean;
    showGrid?: boolean;
    snapToGrid?: boolean;
    canUndo?: boolean;
    canRedo?: boolean;
    serverStatus?: ServerStatus;
    obsStatus?: OBSStatus;
    serverLoading?: boolean;
    obsLoading?: boolean;
    exportLoading?: boolean;
    autoSyncEnabled?: boolean;
  }>(),
  {
    currentProject: null,
    hasUnsavedChanges: false,
    lastSaved: null,
    autoSaveEnabled: true,
    showPreviewElements: true,
    showGrid: false,
    snapToGrid: false,
    canUndo: false,
    canRedo: false,
    serverStatus: () => ({ running: false, url: '' }),
    obsStatus: () => ({ connected: false, streaming: false, recording: false }),
    serverLoading: false,
    obsLoading: false,
    exportLoading: false,
    autoSyncEnabled: true,
  }
);

const emit = defineEmits<{
  "update:autoSaveEnabled": [value: boolean];
  "update:showPreviewElements": [value: boolean];
  "save-project": [];
  "create-project": [];
  "show-project-manager": [];
  "update-project-name": [name: string];
  "open-add-element": [];
  "open-settings": [];
  undo: [];
  redo: [];
  "toggle-grid": [];
  "toggle-snap-to-grid": [];
  "toggle-preview-elements": [];
  "sync-obs": [];
  "export-obs": [];
}>();

const showProjectDropdown = ref(false);
const isEditingProjectName = ref(false);
const editingProjectName = ref("");
const projectNameInput = ref<HTMLInputElement | null>(null);

const autoSaveEnabled = computed({
  get: () => props.autoSaveEnabled,
  set: (value) => emit("update:autoSaveEnabled", value),
});

const showPreviewElements = computed({
  get: () => props.showPreviewElements,
  set: (value) => emit("update:showPreviewElements", value),
});

const startEditingProjectName = async () => {
  isEditingProjectName.value = true;
  editingProjectName.value = props.currentProject?.name || "";
  await nextTick();
  projectNameInput.value?.focus();
};

const saveProjectName = () => {
  if (
    editingProjectName.value.trim() &&
    editingProjectName.value !== props.currentProject?.name
  ) {
    emit("update-project-name", editingProjectName.value.trim());
  }
  isEditingProjectName.value = false;
};

const cancelEditingProjectName = () => {
  isEditingProjectName.value = false;
  editingProjectName.value = "";
};

const saveCurrentProject = () => {
  emit("save-project");
};

const createProject = () => {
  emit("create-project");
};
</script>
