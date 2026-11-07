<template lang="pug">
div(
  class="ctx-panel"
  @dragover="onExternalDragOver"
  @dragleave="onExternalDragLeave"
  @drop="onExternalDrop"
)
  //- Drop zone overlay
  Transition(name="ctx-drop-fade")
    div(v-if="isExternalDragOver" class="ctx-drop-overlay")
      div(class="ctx-drop-zone")
        svg(class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4")
        span Drop to create context menu item

  div(class="ctx-panel-header")
    div(class="ctx-panel-toggle-row")
      div
        p(class="ctx-panel-section-label") Windows Explorer right-click menu
        p(class="ctx-panel-section-hint") Adds VortiDeck actions to the Windows 11 context menu
      div(class="ctx-panel-actions")
        label(class="ctx-toggle" :title="contextMenuStatus.dll_registered ? 'Disable context menu' : 'Enable context menu'")
          input(type="checkbox" :checked="contextMenuStatus.dll_registered" @change="toggleContextMenuFeature" class="sr-only")
          span(class="ctx-toggle-track" :class="{'ctx-toggle-on': contextMenuStatus.dll_registered}")
            span(class="ctx-toggle-thumb")
        button(v-if="contextMenuStatus.dll_registered" @click="showContextMenuModal = true; editingContextMenuItem = null" class="deck-primary-btn ctx-add-item-btn")
          svg(class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v14m-7-7h14")
          span Add item

  div(class="ctx-panel-list")
    div(v-if="!contextMenuStatus.dll_registered" class="ctx-empty-state")
      svg(class="ctx-empty-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 6h16M4 12h16m-7 6h7")
      p(class="ctx-empty-title") Context menu is disabled
      p(class="ctx-empty-hint") Enable it with the toggle above

    div(v-else-if="contextMenuItems.length === 0" class="ctx-empty-state")
      svg(class="ctx-empty-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 4v16m8-8H4")
      p(class="ctx-empty-title") No context menu items yet
      p(class="ctx-empty-hint") Click "+ Add Item" to create your first entry

    template(v-else v-for="entry in flatContextMenuList" :key="entry.type === 'item' ? entry.item.id : entry.groupKey")
      div(
        v-if="entry.type === 'header'"
        draggable="true"
        @dragstart="onGroupDragStart($event, entry.groupKey)"
        @dragover.prevent="onGroupDragOver($event, entry.groupKey)"
        @dragleave="onGroupDragLeave(entry.groupKey)"
        @drop="onGroupDrop($event, entry.groupKey)"
        @dragend="onCtxDragEnd"
        :class="groupHeaderClass(entry.groupKey)"
      )
        svg(class="ctx-drag-handle" viewBox="0 0 16 16" fill="currentColor")
          circle(cx="5" cy="4" r="1.5")
          circle(cx="11" cy="4" r="1.5")
          circle(cx="5" cy="10" r="1.5")
          circle(cx="11" cy="10" r="1.5")
        span(class="ctx-group-label" :class="entry.groupKey === '_default' ? 'ctx-group-default' : 'ctx-group-named'") {{ entry.label }}
        span(class="ctx-group-count") {{ entry.count }}
        div(class="ctx-group-line")

      div(
        v-else
        draggable="true"
        @dragstart="onCtxDragStart($event, entry.item)"
        @dragover.prevent="onCtxDragOver($event, entry.item.id)"
        @dragleave="onCtxDragLeave(entry.item.id)"
        @drop="onCtxDrop($event, entry.item)"
        @dragend="onCtxDragEnd"
        :class="ctxItemClass(entry.item)"
      )
        div(class="flex items-center gap-2.5 flex-1 min-w-0")
          svg(class="ctx-item-handle" viewBox="0 0 16 16" fill="currentColor")
            circle(cx="5" cy="3" r="1.5")
            circle(cx="11" cy="3" r="1.5")
            circle(cx="5" cy="8" r="1.5")
            circle(cx="11" cy="8" r="1.5")
            circle(cx="5" cy="13" r="1.5")
            circle(cx="11" cy="13" r="1.5")
          div(class="min-w-0")
            div(class="flex items-center gap-1.5 flex-wrap")
              span(class="ctx-item-title") {{ entry.item.title }}
              span(v-if="entry.item.group && !hasMultipleGroups" class="ctx-item-group-badge") {{ entry.item.group }}
              span(class="ctx-item-action-badge") {{ getActionBadge(entry.item) }}
            div(class="ctx-item-subtitle") {{ entry.item.file_types.join(', ') || 'All files' }}
        div(class="flex items-center gap-1.5 shrink-0")
          label(class="ctx-mini-toggle" :title="entry.item.enabled ? 'Disable' : 'Enable'")
            input(type="checkbox" :checked="entry.item.enabled" @change="toggleMenuItem(entry.item.id, !entry.item.enabled)" class="sr-only")
            span(class="ctx-mini-track" :class="{'ctx-mini-on': entry.item.enabled}")
              span(class="ctx-mini-thumb")
          button(@click="editMenuItem(entry.item)" class="ctx-item-btn ctx-item-edit-btn" title="Edit")
            svg(class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z")
          button(@click="deleteMenuItem(entry.item.id)" class="ctx-item-btn ctx-item-delete-btn" title="Delete")
            svg(class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16")

  ModalContextMenuItem(
    :isOpen="showContextMenuModal"
    :isEdit="!!editingContextMenuItem"
    :item="editingContextMenuItem"
    :prefill="externalPrefill"
    @close="showContextMenuModal = false; externalPrefill = null"
    @submit="handleContextMenuSubmit"
  )

  //- Delete confirmation
  Transition(name="ctx-drop-fade")
    div(v-if="pendingDeleteId" class="ctx-confirm-overlay" @click.self="pendingDeleteId = null")
      div(class="ctx-confirm-card")
        p(class="ctx-confirm-title") Delete this menu item?
        p(class="ctx-confirm-hint") This action cannot be undone.
        div(class="ctx-confirm-actions")
          button(@click="pendingDeleteId = null" class="ctx-cancel-btn-sm") Cancel
          button(@click="confirmDelete" class="ctx-delete-confirm-btn") Delete
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useMainStore } from "@/stores/Main";

const mainStore = useMainStore();

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

const contextMenuItems = ref<ContextMenuItem[]>([]);
const contextMenuStatus = ref({
  config_exists: false,
  dll_registered: false,
  sparse_package_registered: false,
  item_count: 0,
});
const showContextMenuModal = ref(false);
const editingContextMenuItem = ref<ContextMenuItem | null>(null);

const ctxDragState = ref<{ itemId: string } | null>(null);
const ctxDragOverId = ref<string | null>(null);
const groupDragState = ref<string | null>(null);
const ctxDragOverGroupKey = ref<string | null>(null);

// External drag-drop (from assignments panel)
const isExternalDragOver = ref(false);
const externalPrefill = ref<{
  title?: string;
  actionType?: 'assignment' | 'page' | 'custom';
  assignmentType?: string;
  assignmentId?: string;
  assignmentLabel?: string;
  customAction?: string;
  pagePath?: string;
} | null>(null);

const isExternalActionDrag = (e: DragEvent) =>
  !!e.dataTransfer?.types.includes('application/json') && !e.dataTransfer?.types.includes('text/plain');

const onExternalDragOver = (e: DragEvent) => {
  if (!isExternalActionDrag(e)) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
  isExternalDragOver.value = true;
};

const onExternalDragLeave = (e: DragEvent) => {
  // Only clear if leaving the panel entirely (not entering a child)
  if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) {
    isExternalDragOver.value = false;
  }
};

const onExternalDrop = (e: DragEvent) => {
  isExternalDragOver.value = false;
  if (!e.dataTransfer?.types.includes('application/json')) return;
  e.preventDefault();
  e.stopPropagation();

  try {
    const payload = JSON.parse(e.dataTransfer.getData('application/json'));
    if (payload.event !== 'action-reference') return;

    const { reference, label } = payload.data as import("@base/types/actionReference").ActionReferenceDragPayload["data"];
    const assignmentType = reference.kind === 'saved_action' ? 'obs' : reference.kind;

    // Generic: any assignment type (flow, program, macro, obs, etc.)
    if (reference.id) {
      externalPrefill.value = {
        title: label || '',
        actionType: 'assignment',
        assignmentType,
        assignmentId: reference.id,
        assignmentLabel: label || '',
      };
    } else {
      externalPrefill.value = { title: label || '' };
    }

    editingContextMenuItem.value = null;
    showContextMenuModal.value = true;
  } catch {
    // not a valid payload, ignore
  }
};

type FlatDisplayEntry =
  | { type: 'header'; groupKey: string; label: string; count: number }
  | { type: 'item'; item: ContextMenuItem };

const flatContextMenuList = computed<FlatDisplayEntry[]>(() => {
  const allItems = [...contextMenuItems.value].sort((a, b) => a.order - b.order);
  if (allItems.length === 0) return [];

  const groupOrder: string[] = [];
  for (const item of allItems) {
    const g = item.group || '';
    if (!groupOrder.includes(g)) groupOrder.push(g);
  }

  const hasGroups = groupOrder.length > 1 || (groupOrder.length === 1 && groupOrder[0] !== '');

  const result: FlatDisplayEntry[] = [];
  for (const group of groupOrder) {
    const groupItems = allItems.filter(i => (i.group || '') === group);
    if (hasGroups) {
      result.push({ type: 'header', groupKey: group || '_default', label: group || 'Ungrouped', count: groupItems.length });
    }
    for (const item of groupItems) {
      result.push({ type: 'item', item });
    }
  }
  return result;
});

const hasMultipleGroups = computed(() => {
  const groups = new Set(contextMenuItems.value.map(i => i.group || ''));
  return groups.size > 1 || (groups.size === 1 && !groups.has(''));
});

const onCtxDragStart = (e: DragEvent, item: ContextMenuItem) => {
  ctxDragState.value = { itemId: item.id };
  if (e.dataTransfer) { e.dataTransfer.effectAllowed = 'move'; e.dataTransfer.setData('text/plain', item.id); }
};
const onCtxDragOver = (e: DragEvent, itemId: string) => {
  if (!ctxDragState.value && !groupDragState.value) return;
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  ctxDragOverId.value = itemId;
  ctxDragOverGroupKey.value = null;
};
const onCtxDragLeave = (itemId: string) => { if (ctxDragOverId.value === itemId) ctxDragOverId.value = null; };
const onCtxDragEnd = () => { ctxDragState.value = null; ctxDragOverId.value = null; ctxDragOverGroupKey.value = null; groupDragState.value = null; };

const onCtxDrop = async (e: DragEvent, targetItem: ContextMenuItem) => {
  if (!ctxDragState.value && !groupDragState.value) return;
  e.preventDefault();
  const dragItemId = ctxDragState.value?.itemId;
  const dragGroupKey = groupDragState.value;
  ctxDragState.value = null; ctxDragOverId.value = null; ctxDragOverGroupKey.value = null; groupDragState.value = null;

  if (dragItemId && dragItemId !== targetItem.id) {
    const items = [...contextMenuItems.value].sort((a, b) => a.order - b.order);
    const fromIndex = items.findIndex(i => i.id === dragItemId);
    const toIndex = items.findIndex(i => i.id === targetItem.id);
    if (fromIndex !== -1 && toIndex !== -1) {
      const moved = items[fromIndex];
      if ((moved.group || '') !== (targetItem.group || '')) moved.group = targetItem.group;
      items.splice(fromIndex, 1); items.splice(toIndex, 0, moved);
      items.forEach((item, idx) => { item.order = idx; });
      contextMenuItems.value = items;
      try {
        await invoke("reorder_context_menu_items", { ids: items.map(i => i.id) });
        await invoke("update_context_menu_item", { id: moved.id, item: moved });
      } catch { await loadContextMenuData(); }
    }
    return;
  }
  if (dragGroupKey) {
    const targetGroup = targetItem.group || '';
    if (dragGroupKey !== targetGroup) await reorderGroups(dragGroupKey, targetGroup);
  }
};

const onGroupDragStart = (e: DragEvent, groupKey: string) => {
  groupDragState.value = groupKey;
  if (e.dataTransfer) { e.dataTransfer.effectAllowed = 'move'; e.dataTransfer.setData('text/plain', `group:${groupKey}`); }
};
const onGroupDragOver = (e: DragEvent, groupKey: string) => {
  if (!ctxDragState.value && !groupDragState.value) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  ctxDragOverGroupKey.value = groupKey; ctxDragOverId.value = null;
};
const onGroupDragLeave = (groupKey: string) => { if (ctxDragOverGroupKey.value === groupKey) ctxDragOverGroupKey.value = null; };

const onGroupDrop = async (e: DragEvent, targetGroupKey: string) => {
  if (!ctxDragState.value && !groupDragState.value) return;
  e.preventDefault();
  const dragGroupKey = groupDragState.value;
  const dragItemId = ctxDragState.value?.itemId;
  ctxDragState.value = null; ctxDragOverId.value = null; ctxDragOverGroupKey.value = null; groupDragState.value = null;

  if (dragGroupKey && dragGroupKey !== targetGroupKey) { await reorderGroups(dragGroupKey, targetGroupKey); return; }

  if (dragItemId) {
    const items = [...contextMenuItems.value].sort((a, b) => a.order - b.order);
    const movedItem = items.find(i => i.id === dragItemId);
    const resolvedGroup = targetGroupKey === '_default' ? '' : targetGroupKey;
    if (!movedItem || (movedItem.group || '') === resolvedGroup) return;
    movedItem.group = resolvedGroup;
    const withoutMoved = items.filter(i => i.id !== dragItemId);
    const firstInGroup = withoutMoved.findIndex(i => (i.group || '') === resolvedGroup);
    const insertAt = firstInGroup !== -1 ? firstInGroup : withoutMoved.length;
    withoutMoved.splice(insertAt, 0, movedItem);
    withoutMoved.forEach((item, idx) => { item.order = idx; });
    contextMenuItems.value = withoutMoved;
    try {
      await invoke("reorder_context_menu_items", { ids: withoutMoved.map(i => i.id) });
      await invoke("update_context_menu_item", { id: movedItem.id, item: movedItem });
    } catch { await loadContextMenuData(); }
  }
};

const reorderGroups = async (fromGroupKey: string, toGroupKey: string) => {
  const items = [...contextMenuItems.value].sort((a, b) => a.order - b.order);
  const groupOrder: string[] = [];
  for (const item of items) { const key = (item.group || '') || '_default'; if (!groupOrder.includes(key)) groupOrder.push(key); }
  const fromIdx = groupOrder.indexOf(fromGroupKey);
  const toIdx = groupOrder.indexOf(toGroupKey);
  if (fromIdx === -1 || toIdx === -1) return;
  const [movedGroup] = groupOrder.splice(fromIdx, 1);
  groupOrder.splice(toIdx, 0, movedGroup);
  const newItems: ContextMenuItem[] = [];
  for (const key of groupOrder) {
    const g = key === '_default' ? '' : key;
    newItems.push(...items.filter(i => (i.group || '') === g));
  }
  newItems.forEach((item, idx) => { item.order = idx; });
  contextMenuItems.value = newItems;
  try { await invoke("reorder_context_menu_items", { ids: newItems.map(i => i.id) }); } catch { await loadContextMenuData(); }
};

const ctxItemClass = (item: ContextMenuItem) => {
  const isDragOver = ctxDragOverId.value === item.id && ctxDragState.value?.itemId !== item.id;
  const isDragging = ctxDragState.value?.itemId === item.id;
  return `ctx-item-card${isDragOver ? ' ctx-item-drag-over' : ''}${isDragging ? ' ctx-item-dragging' : ''}`;
};

const groupHeaderClass = (groupKey: string) => {
  const isDragOver = ctxDragOverGroupKey.value === groupKey && groupDragState.value !== groupKey;
  const isDragging = groupDragState.value === groupKey;
  return `ctx-group-header${isDragOver ? ' ctx-group-drag-over' : ''}${isDragging ? ' ctx-group-dragging' : ''}`;
};

const loadContextMenuData = async () => {
  try { contextMenuItems.value = await invoke<ContextMenuItem[]>("get_context_menu_items"); } catch (error) { console.error("Failed to load context menu items:", error); }
  invoke<typeof contextMenuStatus.value>("get_context_menu_status")
    .then((status) => { contextMenuStatus.value = status; })
    .catch((error) => { console.error("Failed to load context menu status:", error); });

};

const handleContextMenuSubmit = async (item: ContextMenuItem) => {
  try {
    if (editingContextMenuItem.value) {
      await invoke("update_context_menu_item", { id: item.id, item });
    } else {
      await invoke("add_context_menu_item", { item });
    }
    await loadContextMenuData();
    mainStore.addNotification({ title: "Context Menu Updated", message: editingContextMenuItem.value ? "Menu item updated" : "New menu item added", type: "success", autoClose: true, duration: 3000 });
    showContextMenuModal.value = false;
    editingContextMenuItem.value = null;
  } catch (error) {
    console.error("Failed to save context menu item:", error);
    mainStore.addNotification({ title: "Error", message: `Failed to save: ${error}`, type: "error", autoClose: true, duration: 5000 });
  }
};

const editMenuItem = (item: ContextMenuItem) => { editingContextMenuItem.value = item; showContextMenuModal.value = true; };

const pendingDeleteId = ref<string | null>(null);

const deleteMenuItem = (id: string) => {
  pendingDeleteId.value = id;
};

const confirmDelete = async () => {
  const id = pendingDeleteId.value;
  pendingDeleteId.value = null;
  if (!id) return;
  try {
    await invoke("remove_context_menu_item", { id });
    await loadContextMenuData();
    mainStore.addNotification({ title: "Context Menu Updated", message: "Menu item removed", type: "success", autoClose: true, duration: 3000 });
  } catch (error) { console.error("Failed to delete context menu item:", error); }
};

const toggleMenuItem = async (id: string, enabled: boolean) => {
  try { await invoke("toggle_context_menu_item", { id, enabled }); await loadContextMenuData(); } catch (error: any) {
    mainStore.addNotification({ title: "Cannot enable", message: `${error}`, type: "error", autoClose: true, duration: 5000 });
    await loadContextMenuData();
  }
};

const toggleContextMenuFeature = async () => {
  try {
    if (contextMenuStatus.value.dll_registered) {
      await invoke("unregister_context_menu_extension");
      mainStore.addNotification({ title: "Context Menu Disabled", message: "VortiDeck removed from Explorer right-click menu", type: "success", autoClose: true, duration: 3000 });
    } else {
      await invoke("register_context_menu_extension");
      mainStore.addNotification({ title: "Context Menu Enabled", message: "VortiDeck added to Explorer right-click menu", type: "success", autoClose: true, duration: 3000 });
    }
    await loadContextMenuData();
  } catch (error) {
    mainStore.addNotification({ title: "Error", message: `${error}`, type: "error", autoClose: true, duration: 5000 });
    await loadContextMenuData();
  }
};

const getActionBadge = (item: ContextMenuItem): string => {
  const action = item.action;
  if (!action) return '';
  if (action.startsWith('open-page-')) return 'Page';
  if (action.startsWith('{')) {
    try {
      const parsed = JSON.parse(action);
      if (parsed.FlowId !== undefined) return 'Flow';
      if (parsed.Program !== undefined) return 'Program';
      if (parsed.MacroId !== undefined) return 'Macro';
      if (parsed.SavedActionId !== undefined) return 'Saved action';
      if (parsed.HttpRequestId !== undefined) return 'HTTP';
      if (parsed.FocusWindowId !== undefined) return 'Focus';
      if (parsed.SystemControl !== undefined) return 'System';
      if (parsed.App !== undefined) return 'App';
    } catch { /* not JSON */ }
  }
  return 'Custom';
};

onMounted(async () => {
  await loadContextMenuData();
  // Re-fetch when an entity is deleted and its context menu items are cleared
  listen("context-menu-updated", () => loadContextMenuData());
});
</script>

<style lang="scss" scoped>
.ctx-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  width: 100%;
  height: 100%;
  overflow-y: auto;
}

.ctx-panel-header {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.ctx-panel-toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.ctx-panel-actions {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex-shrink: 0;
}

.ctx-panel-section-label {
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: hsl(var(--text2));
}

.ctx-panel-section-hint {
  font-size: 0.7rem;
  color: hsl(var(--text2) / 0.78);
  margin-top: 0.15rem;
}

// Feature toggle
.ctx-toggle {
  position: relative;
  display: inline-flex;
  cursor: pointer;
  flex-shrink: 0;
}

.ctx-toggle-track {
  display: block;
  width: 2.2rem;
  height: 1.25rem;
  border-radius: 999px;
  background: hsl(var(--border2) / 0.5);
  border: 1px solid hsl(var(--border2) / 0.8);
  position: relative;
  transition: background 0.2s ease, border-color 0.2s ease;

  &.ctx-toggle-on {
    background: hsl(var(--button));
    border-color: hsl(var(--button));
  }
}

.ctx-toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 0.875rem;
  height: 0.875rem;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
}

.ctx-toggle-on .ctx-toggle-thumb {
  transform: translateX(0.95rem);
}

// Item list
.ctx-panel-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  overflow-y: auto;
  padding-right: 0.125rem;
}

// Empty state
.ctx-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2.5rem 1rem;
  text-align: center;
}

.ctx-empty-icon {
  width: 2.25rem;
  height: 2.25rem;
  color: hsl(var(--text2) / 0.4);
  margin-bottom: 0.5rem;
}

.ctx-empty-title {
  font-size: 0.82rem;
  font-weight: 600;
  color: hsl(var(--text2) / 0.8);
  margin-bottom: 0.15rem;
}

.ctx-empty-hint {
  font-size: 0.72rem;
  color: hsl(var(--text2) / 0.5);
}

// Group header
.ctx-group-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.45rem 0.25rem 0.125rem;
  border-radius: 0.35rem;
  user-select: none;
  transition: background 0.15s ease, outline 0.15s ease, opacity 0.15s ease;

  &.ctx-group-drag-over {
    background: hsl(var(--button) / 0.07);
    outline: 1px solid hsl(var(--button) / 0.25);
  }

  &.ctx-group-dragging {
    opacity: 0.3;
  }
}

.ctx-drag-handle {
  width: 0.7rem;
  height: 0.7rem;
  color: hsl(var(--text2) / 0.7);
  cursor: grab;
  flex-shrink: 0;

  &:active {
    cursor: grabbing;
  }
}

.ctx-group-label {
  font-size: 0.62rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.09em;
}

.ctx-group-default {
  color: hsl(var(--text2) / 0.72);
}

.ctx-group-named {
  color: hsl(var(--button));
}

.ctx-group-count {
  font-size: 0.6rem;
  padding: 0.05rem 0.4rem;
  background: hsl(var(--background3));
  border: 1px solid hsl(var(--border2) / 0.4);
  color: hsl(var(--text2));
  font-weight: 700;
  border-radius: 999px;
}

.ctx-group-line {
  flex: 1;
  height: 1px;
  background: hsl(var(--border2) / 0.3);
}

// Item card
.ctx-item-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.55rem 0.65rem;
  background: hsl(var(--background2));
  border-radius: 0.6rem;
  border: 1px solid hsl(var(--border2) / 0.5);
  transition: border-color 0.15s ease, box-shadow 0.15s ease, opacity 0.15s ease, transform 0.15s ease;
  user-select: none;
  cursor: default;

  &:hover {
    border-color: hsl(var(--border2) / 0.85);
  }

  &.ctx-item-drag-over {
    border-color: hsl(var(--editor-accent) / 0.5);
    box-shadow: 0 4px 12px -6px hsl(var(--editor-accent) / 0.2);
  }

  &.ctx-item-dragging {
    opacity: 0.3;
    transform: scale(0.98);
  }
}

.ctx-item-handle {
  width: 0.9rem;
  height: 0.9rem;
  color: hsl(var(--text2) / 0.55);
  cursor: grab;
  flex-shrink: 0;

  &:active {
    cursor: grabbing;
  }
}

.ctx-item-title {
  font-weight: 500;
  font-size: 0.78rem;
  color: hsl(var(--text));
}

.ctx-item-group-badge {
  font-size: 0.58rem;
  padding: 0.1rem 0.38rem;
  background: hsl(var(--button) / 0.12);
  color: hsl(var(--button));
  border-radius: 0.25rem;
  flex-shrink: 0;
}

.ctx-item-action-badge {
  font-size: 0.56rem;
  padding: 0.08rem 0.35rem;
  background: hsl(var(--border2) / 0.35);
  color: hsl(var(--text2) / 0.9);
  border-radius: 0.25rem;
  flex-shrink: 0;
  font-weight: 600;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.ctx-item-subtitle {
  font-size: 0.68rem;
  color: hsl(var(--text2));
  margin-top: 0.06rem;
}

// Mini toggle
.ctx-mini-toggle {
  position: relative;
  display: inline-flex;
  cursor: pointer;
  flex-shrink: 0;
}

.ctx-mini-track {
  display: block;
  width: 1.75rem;
  height: 0.95rem;
  border-radius: 999px;
  background: hsl(var(--border2) / 0.5);
  border: 1px solid hsl(var(--border2) / 0.7);
  position: relative;
  transition: background 0.2s ease, border-color 0.2s ease;

  &.ctx-mini-on {
    background: hsl(var(--button));
    border-color: hsl(var(--button));
  }
}

.ctx-mini-thumb {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 0.6875rem;
  height: 0.6875rem;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s ease;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
}

.ctx-mini-on .ctx-mini-thumb {
  transform: translateX(0.8rem);
}

// Item action buttons
.ctx-item-btn {
  padding: 0.275rem;
  border-radius: 0.3rem;
  transition: color 0.15s ease, background 0.15s ease;
  color: hsl(var(--text2) / 0.7);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.ctx-item-edit-btn:hover {
  color: hsl(var(--editor-accent));
  background: hsl(var(--editor-accent) / 0.08);
}

.ctx-item-delete-btn:hover {
  color: hsl(0 80% 55%);
  background: hsl(0 80% 55% / 0.08);
}

// External drop overlay
.ctx-drop-overlay {
  position: absolute;
  inset: 0;
  z-index: 20;
  border-radius: 0.75rem;
  border: 2px dashed hsl(var(--editor-accent) / 0.6);
  background: hsl(var(--editor-accent) / 0.06);
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.ctx-drop-zone {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  color: hsl(var(--editor-accent));
  font-size: 0.8rem;
  font-weight: 600;
}

.ctx-drop-fade-enter-active,
.ctx-drop-fade-leave-active {
  transition: opacity 0.15s ease;
}
.ctx-drop-fade-enter-from,
.ctx-drop-fade-leave-to {
  opacity: 0;
}

// Add button
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

.ctx-add-item-btn {
  gap: 0.35rem;
  min-height: 2rem;
  padding: 0.4rem 0.75rem;
  border-radius: 0.6rem;
  white-space: nowrap;
}

@media (max-width: 760px) {
  .ctx-panel-toggle-row {
    align-items: flex-start;
    flex-direction: column;
  }

  .ctx-panel-actions {
    width: 100%;
    justify-content: space-between;
  }
}

// Delete confirmation overlay
.ctx-confirm-overlay {
  position: absolute;
  inset: 0;
  z-index: 25;
  background: hsl(var(--background) / 0.6);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.75rem;
}

.ctx-confirm-card {
  background: hsl(var(--background2));
  border: 1px solid hsl(var(--border2));
  border-radius: 0.75rem;
  padding: 1.25rem 1.5rem;
  text-align: center;
  max-width: 15rem;
}

.ctx-confirm-title {
  font-size: 0.82rem;
  font-weight: 600;
  color: hsl(var(--text));
  margin-bottom: 0.25rem;
}

.ctx-confirm-hint {
  font-size: 0.7rem;
  color: hsl(var(--text2) / 0.65);
  margin-bottom: 0.85rem;
}

.ctx-confirm-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: center;
}

.ctx-cancel-btn-sm {
  padding: 0.3rem 0.75rem;
  font-size: 0.75rem;
  border-radius: 0.5rem;
  border: 1px solid hsl(var(--border2) / 0.65);
  color: hsl(var(--text2));
  transition: color 0.15s ease, border-color 0.15s ease;

  &:hover {
    color: hsl(var(--text));
    border-color: hsl(var(--border2));
  }
}

.ctx-delete-confirm-btn {
  padding: 0.3rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 600;
  border-radius: 0.5rem;
  background: hsl(0 80% 55%);
  color: white;
  transition: filter 0.15s ease;

  &:hover {
    filter: brightness(1.1);
  }
}
</style>
