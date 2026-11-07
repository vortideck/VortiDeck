<template lang="pug">
BaseModal(:is-open="isOpen" @close="$emit('close')")
  div(class="vd-modal-card relative w-full max-w-[40rem] max-h-[90vh] overflow-y-auto")
        //- Header
        div(class="ctx-modal-header")
          div(class="ctx-modal-header-main")
            h3(class="ctx-modal-title") {{ isEdit ? 'Edit Menu Item' : 'Add Menu Item' }}
            p(class="ctx-modal-subtitle") {{ isEdit ? 'Update your Windows Explorer context menu entry' : 'Add a new Windows Explorer context menu entry' }}
          button(@click="$emit('close')" class="vd-modal-close-btn")
            span(class="sr-only") Close
            svg(class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        //- Body
        div(class="p-6 space-y-5")
          //- Title
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Menu Title *
            input(
              v-model="formData.title"
              placeholder="e.g., Open with VortiDeck, Convert Image, Run Script..."
              class="vd-modal-input w-full px-3 py-2"
              @input="validateForm"
            )

          //- Group
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Group
            input(
              v-model="formData.group"
              placeholder="Leave empty for no group, or enter group name e.g. 'Image Tools'"
              class="vd-modal-input w-full px-3 py-2"
            )
            p(class="ctx-modal-help") Items with the same group name appear together, separated by a divider in the "Show more options" menu.

          //- Icon Path
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Icon Path
            div(class="flex gap-2")
              input(
                v-model="formData.icon"
                placeholder="Path to .ico file (optional)"
                class="vd-modal-input flex-1 px-3 py-2"
              )
              button(@click="browseIcon" class="ctx-browse-btn")
                svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z")

          //- Dark Icon Path
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Dark Mode Icon
            div(class="flex gap-2")
              input(
                v-model="formData.iconDark"
                placeholder="Icon for dark theme (optional, falls back to main icon)"
                class="vd-modal-input flex-1 px-3 py-2"
              )
              button(@click="browseIconDark" class="ctx-browse-btn")
                svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z")

          //- File Types
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Match File Types
            input(
              v-model="fileTypesInput"
              placeholder="* for all, or comma-separated: .png, .jpg, .mp4"
              class="vd-modal-input w-full px-3 py-2"
            )
            div(class="flex flex-wrap gap-1.5 mt-2")
              button(
                v-for="preset in fileTypePresets"
                :key="preset.label"
                @click="applyFileTypePreset(preset.value)"
                class="ctx-preset-btn"
              ) {{ preset.label }}

          //- File Names
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Match File Names
            input(
              v-model="formData.fileNames"
              placeholder="Makefile, *.config, readme* (comma-separated, optional)"
              class="vd-modal-input w-full px-3 py-2"
            )
            p(class="ctx-modal-help") Only show for files matching these names. Supports wildcards (*.config, readme*). Leave empty to match by file type only.

          //- Match Targets
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Also Show For
            div(class="flex flex-wrap gap-4")
              div(class="flex items-center gap-2")
                input(id="ctx-match-folders" type="checkbox" v-model="formData.matchFolders" class="ctx-checkbox")
                label(for="ctx-match-folders" class="ctx-check-label") Folders
              div(class="flex items-center gap-2")
                input(id="ctx-match-background" type="checkbox" v-model="formData.matchBackground" class="ctx-checkbox")
                label(for="ctx-match-background" class="ctx-check-label") Directory Background
              div(class="flex items-center gap-2")
                input(id="ctx-match-desktop" type="checkbox" v-model="formData.matchDesktop" class="ctx-checkbox")
                label(for="ctx-match-desktop" class="ctx-check-label") Desktop Background
              div(class="flex items-center gap-2")
                input(id="ctx-match-drive" type="checkbox" v-model="formData.matchDrive" class="ctx-checkbox")
                label(for="ctx-match-drive" class="ctx-check-label") Drive Roots
            p(class="ctx-modal-help") Items always show for matching file types. Check above to also show for folders, directory/desktop backgrounds, or drive roots.

          //- Action section
          div(class="space-y-1.5")
            label(class="ctx-modal-label") Action *
            select(
              v-model="formData.actionType"
              class="vd-modal-select w-full px-3 py-2"
              @change="onActionTypeChange"
            )
              option(value="assignment") Deck Assignment
              option(value="page") Open VortiDeck Page
              option(value="custom") Custom Deep Link

          //- Assignment picker (searchable dropdown)
          div(v-if="formData.actionType === 'assignment'" class="space-y-2")
            ActionPicker(
              :model-value="assignmentPickerValue"
              @update:model-value="onActionSelected"
              placeholder="Search flows, programs, macros..."
            )
            p(class="ctx-modal-help") Search for flows, programs, macros and more. You can also drag from the assignments panel.

          //- Page selector
          div(v-if="formData.actionType === 'page'" class="space-y-1.5")
            label(class="ctx-modal-label") Page
            select(
              v-model="formData.pagePath"
              class="vd-modal-select w-full px-3 py-2"
              @change="validateForm"
            )
              option(value="assistant") Assistant
              option(value="settings") Settings
              option(value="flows") Flows
              option(value="plugins") Plugins
              option(value="streaming") Streaming

          //- Custom deep link
          div(v-if="formData.actionType === 'custom'" class="space-y-1.5")
            label(class="ctx-modal-label") Deep Link URL *
            input(
              v-model="formData.customAction"
              placeholder="vortideck://context-menu/my-action"
              class="vd-modal-input w-full px-3 py-2"
              @input="validateForm"
            )

          //- Enabled toggle
          div(class="flex items-center gap-2")
            input(id="ctx-menu-enabled" type="checkbox" v-model="formData.enabled" class="ctx-checkbox")
            label(for="ctx-menu-enabled" class="ctx-check-label") Enable this menu item

        //- Footer
        div(class="vd-modal-footer ctx-modal-footer")
          button(@click="$emit('close')" class="ctx-cancel-btn") Cancel
          button(
            @click="submitForm"
            :disabled="!isFormValid"
            class="vd-modal-primary-btn ctx-submit-btn"
            :class="{'ctx-submit-disabled': !isFormValid}"
          ) {{ isEdit ? 'Update' : 'Add' }} Item
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import ActionPicker from "../ActionPicker.vue";
import { ref, watch, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

interface ContextMenuItem {
  id: string;
  title: string;
  icon: string;
  file_types: string[];
  action: string;
  enabled: boolean;
  order: number;
  group: string;
  root_level: boolean;
  working_dir: string;
  show_window: string;
  match_folders: boolean;
  match_background: boolean;
  use_cmd: boolean;
  icon_dark: string;
  match_desktop: boolean;
  match_drive: boolean;
  file_names: string[];
}

interface ActionPrefill {
  title?: string;
  actionType?: "assignment" | "page" | "custom";
  assignmentType?: string;
  assignmentId?: string;
  assignmentLabel?: string;
  customAction?: string;
  pagePath?: string;
}

interface Props {
  isOpen: boolean;
  isEdit?: boolean;
  item?: ContextMenuItem | null;
  prefill?: ActionPrefill | null;
}

interface Emits {
  (e: "close"): void;
  (e: "submit", item: ContextMenuItem): void;
}

const props = withDefaults(defineProps<Props>(), {
  isEdit: false,
  item: null,
  prefill: null,
});

const emit = defineEmits<Emits>();

const isFormValid = ref(false);
const fileTypesInput = ref("*");

const formData = ref({
  title: "",
  icon: "",
  iconDark: "",
  group: "",
  actionType: "assignment" as "assignment" | "page" | "custom",
  // Assignment fields (deck action reference)
  assignmentType: "",   // e.g. "flow", "program", "macro"
  assignmentId: "",     // the entity ID
  assignmentLabel: "",  // human-readable name from the drag
  // Page fields
  pagePath: "assistant",
  // Custom fields
  customAction: "",
  enabled: true,
  matchFolders: false,
  matchBackground: false,
  matchDesktop: false,
  matchDrive: false,
  fileNames: "",
});

// ActionPicker v-model bridge
const assignmentPickerValue = computed(() => {
  if (!formData.value.assignmentType || !formData.value.assignmentId) return null;
  return { type: formData.value.assignmentType, id: formData.value.assignmentId };
});

const onActionSelected = (selected: { type: string; id: string; label: string } | null) => {
  if (selected) {
    formData.value.assignmentType = selected.type;
    formData.value.assignmentId = selected.id;
    formData.value.assignmentLabel = selected.label;
  } else {
    formData.value.assignmentType = "";
    formData.value.assignmentId = "";
    formData.value.assignmentLabel = "";
  }
  validateForm();
};

const fileTypePresets = [
  { label: "All Files", value: "*" },
  { label: "Images", value: ".png, .jpg, .jpeg, .gif, .bmp, .webp, .svg" },
  { label: "Videos", value: ".mp4, .mkv, .avi, .mov, .webm" },
  { label: "Audio", value: ".mp3, .wav, .flac, .ogg, .aac" },
  { label: "Documents", value: ".pdf, .doc, .docx, .txt, .md" },
];

const applyFileTypePreset = (value: string) => {
  fileTypesInput.value = value;
};

const onActionTypeChange = () => {
  validateForm();
};

const buildAction = (): string => {
  switch (formData.value.actionType) {
    case "assignment": {
      // Store as Rust Action enum JSON — executed by UniversalExecutionEngine
      const aType = formData.value.assignmentType;
      const aId = formData.value.assignmentId;
      if (aType === "flow") return JSON.stringify({ FlowId: aId });
      if (aType === "program") return JSON.stringify({ Program: parseInt(aId, 10) });
      if (aType === "macro") return JSON.stringify({ MacroId: parseInt(aId, 10) });
      if (aType === "obs") return JSON.stringify({ SavedActionId: aId });
      if (aType === "http") return JSON.stringify({ HttpRequestId: parseInt(aId, 10) });
      if (aType === "focus") return JSON.stringify({ FocusWindowId: parseInt(aId, 10) });
      if (aType === "system") return JSON.stringify({ SystemControl: aId });
      if (aType === "app") return JSON.stringify({ App: aId });
      // Fallback for unknown types
      return JSON.stringify({ FlowId: aId });
    }
    case "page":
      return `open-page-${formData.value.pagePath}`;
    case "custom":
      return formData.value.customAction;
    default:
      return "";
  }
};

const validateForm = () => {
  const hasTitle = formData.value.title.trim().length > 0;
  let hasAction = false;
  switch (formData.value.actionType) {
    case "assignment":
      hasAction = !!formData.value.assignmentType && !!formData.value.assignmentId;
      break;
    case "page":
      hasAction = !!formData.value.pagePath;
      break;
    case "custom":
      hasAction = formData.value.customAction.trim().length > 0;
      break;
  }
  isFormValid.value = hasTitle && hasAction;
};

const browseIcon = async () => {
  try {
    const selected = await open({ multiple: false, filters: [{ name: "Icons", extensions: ["ico", "png", "exe"] }] });
    if (selected) formData.value.icon = selected as string;
  } catch (e) { console.error("Failed to browse icon:", e); }
};

const browseIconDark = async () => {
  try {
    const selected = await open({ multiple: false, filters: [{ name: "Icons", extensions: ["ico", "png", "exe"] }] });
    if (selected) formData.value.iconDark = selected as string;
  } catch (e) { console.error("Failed to browse dark icon:", e); }
};

const parseFileTypes = (): string[] => {
  const raw = fileTypesInput.value.trim();
  if (raw === "*" || raw === "") return ["*"];
  return raw.split(",").map((t) => t.trim()).filter((t) => t.length > 0).map((t) => (t.startsWith(".") ? t : `.${t}`));
};

const parseFileNames = (): string[] => {
  const raw = formData.value.fileNames.trim();
  if (raw === "") return [];
  return raw.split(",").map((t) => t.trim()).filter((t) => t.length > 0);
};

const generateId = (): string => `ctx-${Date.now()}-${Math.random().toString(36).substring(2, 8)}`;

const submitForm = () => {
  if (!isFormValid.value) return;
  const item: ContextMenuItem = {
    id: props.isEdit && props.item ? props.item.id : generateId(),
    title: formData.value.title,
    icon: formData.value.icon,
    file_types: parseFileTypes(),
    action: buildAction(),
    enabled: formData.value.enabled,
    order: props.isEdit && props.item ? props.item.order : 999,
    group: formData.value.group.trim(),
    root_level: false,
    working_dir: "",
    show_window: "normal",
    match_folders: formData.value.matchFolders,
    match_background: formData.value.matchBackground,
    use_cmd: false,
    icon_dark: formData.value.iconDark,
    match_desktop: formData.value.matchDesktop,
    match_drive: formData.value.matchDrive,
    file_names: parseFileNames(),
  };
  emit("submit", item);
  emit("close");
};

const parseExistingItem = (item: ContextMenuItem) => {
  formData.value.title = item.title;
  formData.value.icon = item.icon;
  formData.value.iconDark = item.icon_dark || "";
  formData.value.group = item.group || "";
  formData.value.enabled = item.enabled;
  formData.value.matchFolders = item.match_folders || false;
  formData.value.matchBackground = item.match_background || false;
  formData.value.matchDesktop = item.match_desktop || false;
  formData.value.matchDrive = item.match_drive || false;
  formData.value.fileNames = (item.file_names || []).join(", ");
  fileTypesInput.value = item.file_types.join(", ");

  const action = item.action;
  if (action.startsWith("open-page-")) {
    formData.value.actionType = "page";
    formData.value.pagePath = action.split("open-page-")[1]?.split("?")[0] || "assistant";
  } else if (action.startsWith("{")) {
    // Action JSON (Rust Action enum) — parse to extract assignment type and ID
    try {
      const parsed = JSON.parse(action);
      formData.value.actionType = "assignment";
      if (parsed.FlowId !== undefined) { formData.value.assignmentType = "flow"; formData.value.assignmentId = String(parsed.FlowId); }
      else if (parsed.Program !== undefined) { formData.value.assignmentType = "program"; formData.value.assignmentId = String(parsed.Program); }
      else if (parsed.MacroId !== undefined) { formData.value.assignmentType = "macro"; formData.value.assignmentId = String(parsed.MacroId); }
      else if (parsed.SavedActionId !== undefined) { formData.value.assignmentType = "obs"; formData.value.assignmentId = String(parsed.SavedActionId); }
      else if (parsed.HttpRequestId !== undefined) { formData.value.assignmentType = "http"; formData.value.assignmentId = String(parsed.HttpRequestId); }
      else if (parsed.FocusWindowId !== undefined) { formData.value.assignmentType = "focus"; formData.value.assignmentId = String(parsed.FocusWindowId); }
      else if (parsed.SystemControl !== undefined) { formData.value.assignmentType = "system"; formData.value.assignmentId = String(parsed.SystemControl); }
      else if (parsed.App !== undefined) { formData.value.assignmentType = "app"; formData.value.assignmentId = String(parsed.App); }
      formData.value.assignmentLabel = ""; // label not stored, show ID as fallback
    } catch {
      formData.value.actionType = "custom";
      formData.value.customAction = action;
    }
  } else {
    formData.value.actionType = "custom";
    formData.value.customAction = action;
  }

  validateForm();
};

const resetForm = () => {
  formData.value = {
    title: "", icon: "", iconDark: "", group: "",
    actionType: "assignment",
    assignmentType: "", assignmentId: "", assignmentLabel: "",
    pagePath: "assistant", customAction: "",
    enabled: true, matchFolders: false, matchBackground: false,
    matchDesktop: false, matchDrive: false, fileNames: "",
  };
  fileTypesInput.value = "*";
  isFormValid.value = false;
};

watch(
  () => props.item,
  (newItem) => { if (newItem && props.isEdit) parseExistingItem(newItem); },
  { immediate: true }
);

watch(
  [() => props.isOpen, () => props.prefill],
  ([isOpen]) => {
    if (!isOpen) return;
    if (props.isEdit) return;
    resetForm();
    if (props.prefill) {
      if (props.prefill.title) formData.value.title = props.prefill.title;
      if (props.prefill.actionType) formData.value.actionType = props.prefill.actionType;
      if (props.prefill.assignmentType) formData.value.assignmentType = props.prefill.assignmentType;
      if (props.prefill.assignmentId) formData.value.assignmentId = props.prefill.assignmentId;
      if (props.prefill.assignmentLabel) formData.value.assignmentLabel = props.prefill.assignmentLabel;
      if (props.prefill.pagePath) formData.value.pagePath = props.prefill.pagePath;
      if (props.prefill.customAction) formData.value.customAction = props.prefill.customAction;
      validateForm();
    }
  }
);
</script>

<style lang="scss" scoped>
@use "./modalTheme.scss" as *;

.ctx-modal-header {
  padding: 1.35rem 1.55rem 1.1rem;
  border-bottom: 1px solid hsl(var(--border2) / 0.72);
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.9rem;
}

.ctx-modal-header-main { min-width: 0; }

.ctx-modal-title {
  font-size: 1.65rem;
  line-height: 1.05;
  font-weight: 800;
  color: hsl(var(--text));
  letter-spacing: -0.02em;
}

.ctx-modal-subtitle {
  margin-top: 0.28rem;
  color: hsl(var(--text2));
  font-size: 0.8rem;
}

.ctx-modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1rem 1.55rem;
}

.ctx-modal-label {
  display: block;
  font-size: 0.8rem;
  font-weight: 500;
  color: hsl(var(--text));
  margin-bottom: 0.35rem;
}

.ctx-check-label {
  font-size: 0.875rem;
  color: hsl(var(--text2));
  cursor: pointer;
}

.ctx-modal-help {
  font-size: 0.72rem;
  color: hsl(var(--text2) / 0.65);
  margin-top: 0.3rem;
}

.ctx-checkbox {
  width: 0.95rem;
  height: 0.95rem;
  border-radius: 0.25rem;
  accent-color: hsl(var(--editor-accent));
  cursor: pointer;
}

.ctx-browse-btn {
  padding: 0.5rem 0.7rem;
  border-radius: 0.72rem;
  border: 1px solid hsl(var(--editor-accent) / 0.28);
  background: hsl(var(--editor-accent) / 0.07);
  color: hsl(var(--editor-accent));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.2s ease, border-color 0.2s ease;

  &:hover {
    background: hsl(var(--editor-accent) / 0.15);
    border-color: hsl(var(--editor-accent) / 0.5);
  }
}

.ctx-preset-btn {
  padding: 0.2rem 0.55rem;
  font-size: 0.72rem;
  border-radius: 0.45rem;
  border: 1px solid hsl(var(--border2) / 0.65);
  background: hsl(var(--background2));
  color: hsl(var(--text2));
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;

  &:hover {
    background: hsl(var(--editor-accent) / 0.08);
    border-color: hsl(var(--editor-accent) / 0.4);
    color: hsl(var(--editor-accent));
  }
}

.ctx-cancel-btn {
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
  font-weight: 500;
  color: hsl(var(--text2));
  border-radius: 0.72rem;
  border: 1px solid hsl(var(--border2) / 0.65);
  transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;

  &:hover {
    color: hsl(var(--text));
    border-color: hsl(var(--border2));
    background: hsl(var(--background2));
  }
}

.ctx-submit-btn {
  padding: 0.5rem 1.5rem;
  font-size: 0.875rem;
  font-weight: 600;
}

.ctx-submit-disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}
</style>
