<template lang="pug">
div.action-picker-wrap(
  ref="pickerRoot"
  :class="{ 'action-picker-wrap--selected': Boolean(selected) }"
  @pointerdown.stop
  @mousedown.stop="handlePickerMouseDown"
  @click.stop
)
  Multiselect(
    ref="multiselect"
    :model-value="selected"
    @update:model-value="onSelect"
    :options="grouped"
    :searchable="true"
    :close-on-select="true"
    :show-labels="false"
    :loading="loading"
    :max-height="360"
    :use-teleport="true"
    :content-wrapper-class="dropdownClass"
    label="label"
    track-by="value"
    group-values="items"
    group-label="group"
    :group-select="false"
    :placeholder="selected ? '' : placeholder"
    @open="handleDropdownOpen"
    @close="handleDropdownClose"
  )
    template(#noResult) No actions found
    template(#noOptions) Loading actions...
</template>

<script lang="ts">
export interface DeckAction {
  label: string;
  value: string;
  type: string;
  id: string;
}

// Module-level cache with TTL (5 min)
let _cache: DeckAction[] | null = null;
let _promise: Promise<DeckAction[]> | null = null;
let _cacheTime = 0;
const CACHE_TTL = 5 * 60 * 1000;

/** Clear the action cache so next mount re-fetches. */
export function invalidateActionCache() {
  _cache = null;
  _promise = null;
  _cacheTime = 0;
}
</script>

<script setup lang="ts">
import Multiselect from "vue-multiselect";
import "vue-multiselect/dist/vue-multiselect.css";
import { computed, nextTick, onBeforeUnmount, ref, useId } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  modelValue?: { type: string; id: string } | null;
  placeholder?: string;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: null,
  placeholder: "Search actions...",
});

const emit = defineEmits<{
  (e: "update:modelValue", val: { type: string; id: string; label: string; value: string } | null): void;
}>();

const pickerRoot = ref<HTMLElement | null>(null);
const multiselect = ref<{ activate?: () => void } | null>(null);
const pickerUid = `action-picker-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
const dropdownClass = `action-picker-dropdown context-menu-floating scrollbar-visible ${pickerUid}`;
let dropdownOpen = false;
let positionFrame: number | null = null;
let dropdownPlacement: "above" | "below" | null = null;
let dropdownStyleObserver: MutationObserver | null = null;
let dropdownGeometry: {
  left: string;
  top: string;
  width: string;
  maxHeight: string;
} | null = null;

const handlePickerMouseDown = (event: MouseEvent) => {
  const target = event.target;
  if (!(target instanceof Element)) return;

  // vue-multiselect already handles these controls. Its surrounding tags
  // container does not, even though it visually occupies the same field.
  if (target.closest(
    ".multiselect__select, .multiselect__input, .multiselect__placeholder, .multiselect__single",
  )) return;

  multiselect.value?.activate?.();
};

const dropdownElement = () => document.querySelector<HTMLElement>(`.${pickerUid}`);

const enforceDropdownGeometry = (dropdown: HTMLElement) => {
  if (!dropdownGeometry) return;
  const declarations: Array<[string, string]> = [
    ["position", "fixed"],
    ["top", dropdownGeometry.top],
    ["right", "auto"],
    ["bottom", "auto"],
    ["left", dropdownGeometry.left],
    ["width", dropdownGeometry.width],
    ["max-height", dropdownGeometry.maxHeight],
    ["z-index", "100001"],
  ];
  for (const [property, value] of declarations) {
    if (
      dropdown.style.getPropertyValue(property) !== value
      || dropdown.style.getPropertyPriority(property) !== "important"
    ) dropdown.style.setProperty(property, value, "important");
  }
};

const guardDropdownGeometry = (dropdown: HTMLElement) => {
  dropdownStyleObserver?.disconnect();
  dropdownStyleObserver = new MutationObserver(() => enforceDropdownGeometry(dropdown));
  dropdownStyleObserver.observe(dropdown, { attributes: true, attributeFilter: ["style"] });
};

const positionDropdown = () => {
  if (!dropdownOpen || !pickerRoot.value) return false;
  const dropdown = dropdownElement();
  if (!dropdown) return false;

  const viewportPadding = 8;
  const gap = 4;
  const desiredHeight = 360;
  const anchor = pickerRoot.value.getBoundingClientRect();
  const roomBelow = Math.max(0, window.innerHeight - anchor.bottom - gap - viewportPadding);
  const roomAbove = Math.max(0, anchor.top - gap - viewportPadding);
  const shouldOpenAbove = roomBelow < Math.min(220, desiredHeight) && roomAbove > roomBelow;
  dropdownPlacement ??= shouldOpenAbove ? "above" : "below";
  const openAbove = dropdownPlacement === "above";
  const availableHeight = openAbove ? roomAbove : roomBelow;
  const maxHeight = Math.min(desiredHeight, availableHeight);
  const width = Math.min(anchor.width, window.innerWidth - viewportPadding * 2);
  const left = Math.max(
    viewportPadding,
    Math.min(anchor.left, window.innerWidth - width - viewportPadding),
  );

  // vue-multiselect owns and rewrites its inline position styles whenever its
  // pointer state renders. Store geometry in independent custom properties so
  // option hover cannot restore the package's absolute/below positioning.
  dropdown.style.setProperty("--action-picker-left", `${left}px`);
  dropdown.style.setProperty("--action-picker-width", `${width}px`);
  dropdown.style.setProperty("--action-picker-max-height", `${maxHeight}px`);

  const renderedHeight = Math.min(dropdown.scrollHeight, maxHeight);
  const top = openAbove
    ? anchor.top - gap - renderedHeight
    : anchor.bottom + gap;
  const resolvedTop = `${Math.max(
    viewportPadding,
    Math.min(top, window.innerHeight - renderedHeight - viewportPadding),
  )}px`;
  dropdown.style.setProperty(
    "--action-picker-top",
    resolvedTop,
  );
  dropdownGeometry = {
    left: `${left}px`,
    top: resolvedTop,
    width: `${width}px`,
    maxHeight: `${maxHeight}px`,
  };
  enforceDropdownGeometry(dropdown);
  guardDropdownGeometry(dropdown);
  dropdown.classList.toggle("action-picker-dropdown--above", openAbove);
  dropdown.classList.toggle("action-picker-dropdown--below", !openAbove);
  dropdown.classList.add("action-picker-dropdown--positioned");
  return true;
};

const scheduleDropdownPosition = async () => {
  await nextTick();
  if (positionDropdown()) return;
  positionFrame = requestAnimationFrame(() => {
    positionFrame = null;
    positionDropdown();
  });
};

const handleDropdownOpen = () => {
  dropdownOpen = true;
  dropdownPlacement = null;
  dropdownGeometry = null;
  window.addEventListener("resize", positionDropdown);
  window.addEventListener("scroll", handleViewportScroll, true);
  void scheduleDropdownPosition();
};

const handleViewportScroll = (event: Event) => {
  const target = event.target;
  if (target instanceof Element && target.closest(`.${pickerUid}`)) return;
  positionDropdown();
};

const handleDropdownClose = () => {
  dropdownOpen = false;
  dropdownPlacement = null;
  dropdownGeometry = null;
  dropdownStyleObserver?.disconnect();
  dropdownStyleObserver = null;
  if (positionFrame !== null) cancelAnimationFrame(positionFrame);
  positionFrame = null;
  dropdownElement()?.classList.remove(
    "action-picker-dropdown--positioned",
    "action-picker-dropdown--above",
    "action-picker-dropdown--below",
  );
  window.removeEventListener("resize", positionDropdown);
  window.removeEventListener("scroll", handleViewportScroll, true);
};

onBeforeUnmount(handleDropdownClose);

const actions = ref<DeckAction[]>(_cache || []);
const loading = ref(false);

const GROUP_LABELS: Record<string, string> = {
  flow: "Flows", program: "Programs", macro: "Macros",
  obs: "OBS Actions", discord: "Discord Actions",
  http: "HTTP Requests", focus: "Focus Windows", system: "System Actions",
};

const grouped = computed(() => {
  const groups: Record<string, DeckAction[]> = {};
  for (const a of actions.value) {
    const g = GROUP_LABELS[a.type] || a.type;
    if (!groups[g]) groups[g] = [];
    groups[g].push(a);
  }
  return Object.entries(groups).map(([group, items]) => ({ group, items }));
});

const selected = computed(() => {
  if (!props.modelValue?.type || !props.modelValue?.id) return null;
  return actions.value.find(
    a => a.type === props.modelValue!.type && a.id === props.modelValue!.id
  ) || null;
});

const onSelect = (action: DeckAction | null) => {
  emit("update:modelValue", action ? {
    type: action.type,
    id: action.id,
    label: action.label,
    value: action.value,
  } : null);
};

async function loadActions(): Promise<DeckAction[]> {
  if (_cache && (Date.now() - _cacheTime) < CACHE_TTL) return _cache;
  if (_promise) return _promise;

  _promise = (async () => {
    const result: DeckAction[] = [];

    try {
      const systemActions = await invoke<Array<{ id: string; name: string }>>("get_available_actions");
      if (Array.isArray(systemActions)) systemActions.forEach(action => result.push({
        label: action.name || action.id,
        value: JSON.stringify({ SystemControl: action.id }),
        type: "system",
        id: action.id,
      }));
    } catch {}
    try {
      const programs = await invoke<any[]>("get_programs_with_meta", { includeIcons: false });
      if (Array.isArray(programs)) programs.forEach(p => result.push({ label: p.name || `Program ${p.id}`, value: `{"Program":${p.id}}`, type: "program", id: String(p.id) }));
    } catch {}
    try {
      const macros = await invoke<any[]>("get_all_macros");
      if (Array.isArray(macros)) macros.forEach(m => result.push({ label: m.name || `Macro ${m.id}`, value: `{"MacroId":${m.id}}`, type: "macro", id: String(m.id) }));
    } catch {}
    try {
      const requests = await invoke<any[]>("list_http_requests");
      if (Array.isArray(requests)) requests.forEach(r => result.push({ label: r.name || `HTTP Request ${r.id}`, value: `{"HttpRequestId":${r.id}}`, type: "http", id: String(r.id) }));
    } catch {}
    try {
      const savedActions = await invoke<Record<string, any[]>>("get_all_saved_integration_actions");
      for (const [integrationId, providerActions] of Object.entries(savedActions || {})) {
        if (!Array.isArray(providerActions)) continue;
        providerActions.forEach(a => result.push({
          label: a.name || `${integrationId.toUpperCase()} Action ${a.id}`,
          value: JSON.stringify({ SavedActionId: a.stable_id }),
          type: integrationId,
          id: String(a.stable_id),
        }));
      }
    } catch {}
    try {
      const windows = await invoke<any[]>("get_focus_windows");
      if (Array.isArray(windows)) windows.forEach(w => result.push({ label: w.name || `Focus Window ${w.id}`, value: `{"FocusWindowId":${w.id}}`, type: "focus", id: String(w.id) }));
    } catch {}
    try {
      const flows = await invoke<any[]>("list_flows");
      if (Array.isArray(flows)) flows.forEach(f => result.push({ label: f.name || `Flow ${f.id}`, value: `{"FlowId":"${f.id}"}`, type: "flow", id: String(f.id) }));
    } catch {}

    _cache = result;
    _cacheTime = Date.now();
    _promise = null;
    return result;
  })();
  return _promise;
}

// Load on mount
loading.value = true;
loadActions().then(result => { actions.value = result; }).finally(() => { loading.value = false; });
</script>

<style lang="scss">
.action-picker-wrap {
  height: 38px;
  position: relative;

  .multiselect {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2));
    border-radius: 0.72rem;
    height: 38px !important;
    min-height: 38px !important;
    max-height: 38px !important;
    font-size: 0.825rem;
    box-sizing: border-box;
    /* The list is teleported; clipping removes overlapping inner corners. */
    overflow: hidden;
  }

  .multiselect--active {
    height: 38px !important;
    min-height: 38px !important;
    max-height: 38px !important;
  }

  .multiselect__tags {
    position: relative;
    background: hsl(var(--background2));
    border: none;
    border-radius: calc(0.72rem - 1px) !important;
    height: 36px !important;
    min-height: 36px !important;
    max-height: 36px !important;
    padding: 0 40px 0 10px;
    display: flex;
    align-items: center;
    box-sizing: border-box;
    overflow: hidden;
    cursor: text;
  }

  .multiselect__single,
  .multiselect__placeholder {
    font-size: 0.825rem;
    margin: 0 !important;
    padding: 0 !important;
    line-height: 36px !important;
    height: 36px !important;
    box-sizing: border-box;
    cursor: text;
  }

  .multiselect__single {
    position: absolute;
    z-index: 3;
    inset: 0 40px 0 10px;
    width: auto;
  }

  /* Keep the real searchable input wider than the package's text-only target. */
  .multiselect__placeholder {
    display: none !important;
  }

  .multiselect__input {
    position: absolute !important;
    z-index: 2;
    top: 0;
    right: 40px;
    bottom: 0;
    left: 10px;
    width: calc(100% - 50px) !important;
    font-size: 0.825rem;
    margin: 0 !important;
    padding: 0 !important;
    line-height: 36px !important;
    height: 36px !important;
    box-sizing: border-box;
    cursor: text;
    border-radius: 0 !important;
  }

  .multiselect__single {
    background: transparent;
    color: hsl(var(--text));
  }

  .multiselect__input {
    background: hsl(var(--background2));
    color: hsl(var(--text));
  }

  .multiselect__placeholder {
    color: hsl(var(--text2) / 0.6);
  }

  .multiselect__select {
    z-index: 4;
    top: 0;
    right: 0;
    height: 38px;
    padding: 0;
    border-left: 0 !important;
    border-radius: 0 !important;
    background: transparent !important;
    cursor: pointer;
    transform: none !important;

    &:before {
      position: absolute;
      top: 50% !important;
      right: auto !important;
      left: 50% !important;
      display: block;
      margin: 0 !important;
      border-color: hsl(var(--text2)) transparent transparent;
      transform: translate(-50%, -50%);
      transform-origin: center;
      transition: transform 0.16s ease;
    }
  }

  .multiselect--active .multiselect__select {
    transform: none !important;
  }

  .multiselect--active .multiselect__select:before {
    transform: translate(-50%, -50%) rotate(180deg);
  }

  .multiselect--active .multiselect__tags {
    border-radius: calc(0.72rem - 1px) !important;
    border-color: hsl(var(--editor-accent) / 0.5);
  }

  .multiselect--active .multiselect__input {
    border-radius: 0 !important;
  }

  .multiselect--active .multiselect__single {
    display: none;
  }

  .multiselect__spinner {
    background: hsl(var(--background2));
  }

  .multiselect__spinner:before,
  .multiselect__spinner:after {
    border-top-color: hsl(var(--editor-accent));
  }
}

.action-picker-wrap--selected .multiselect:not(.multiselect--active) .multiselect__input {
  width: 1px !important;
  opacity: 0;
  pointer-events: none;
}

.action-picker-dropdown {
  position: fixed !important;
  top: var(--action-picker-top, 0) !important;
  right: auto !important;
  bottom: auto !important;
  left: var(--action-picker-left, 0) !important;
  width: var(--action-picker-width, 0) !important;
  max-height: var(--action-picker-max-height, 0) !important;
  z-index: 100001 !important;
  visibility: hidden;
  transition: opacity 0.1s ease !important;
  will-change: opacity;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  border: 1px solid hsl(var(--border2)) !important;
  border-radius: .5rem !important;
  background: hsl(var(--background2)) !important;
  box-shadow: 0 18px 42px -10px hsl(var(--text) / .35);
  scrollbar-color: hsl(var(--border2)) transparent;
  scrollbar-width: thin;
}

.action-picker-dropdown.action-picker-dropdown--positioned {
  visibility: visible;
}

.action-picker-dropdown .multiselect__content,
.action-picker-dropdown .multiselect__element {
  background: hsl(var(--background2));
}

.action-picker-dropdown .multiselect__option {
  min-height: unset;
  padding: 6px 10px;
  color: hsl(var(--text));
  font-size: .76rem;
  line-height: 1.3;
}

.action-picker-dropdown .multiselect__option--highlight {
  background: hsl(var(--background3, var(--background2)));
  color: hsl(var(--text));
}

.action-picker-dropdown .multiselect__option--highlight::after {
  display: none;
}

.action-picker-dropdown .multiselect__option--selected {
  background: hsl(var(--editor-accent) / .15);
  color: hsl(var(--editor-accent));
  font-weight: 500;
}

.action-picker-dropdown .multiselect__option--selected.multiselect__option--highlight {
  background: hsl(var(--editor-accent) / .25);
  color: hsl(var(--editor-accent));
}

.action-picker-dropdown .multiselect__option--group {
  cursor: default;
  background: hsl(var(--background)) !important;
  padding: 4px 10px;
  color: hsl(var(--text2)) !important;
  font-size: .68rem;
  font-weight: 600;
  letter-spacing: .05em;
  pointer-events: none;
  text-transform: uppercase;
}

.action-picker-dropdown::-webkit-scrollbar {
  width: 8px;
  background: transparent;
}

.action-picker-dropdown::-webkit-scrollbar-track,
.action-picker-dropdown::-webkit-scrollbar-corner {
  background: transparent;
}

.action-picker-dropdown::-webkit-scrollbar-thumb {
  border: 2px solid transparent;
  border-radius: 8px;
  background: hsl(var(--border2));
  background-clip: padding-box;
}

.action-picker-dropdown::-webkit-scrollbar-button,
.action-picker-dropdown::-webkit-scrollbar-button:single-button,
.action-picker-dropdown::-webkit-scrollbar-button:vertical:decrement,
.action-picker-dropdown::-webkit-scrollbar-button:vertical:increment {
  display: none;
  width: 0;
  height: 0;
}
</style>
