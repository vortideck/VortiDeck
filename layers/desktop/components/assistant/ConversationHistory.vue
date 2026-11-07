<template lang="pug">
  div(class="conversation-history-bar bg-themeBackground2 border-b border-themeBorder2 flex items-center")
    // Control buttons - sticky (don't scroll)
    div(class="sticky left-0 z-10 bg-themeBackground2 shrink-0 flex items-center gap-1 px-2 py-2 border-r border-themeBorder2")
      // New Chat button
      button(
        @click="createNew"
        class="px-2.5 py-1 bg-blue-600 hover:bg-blue-500 active:scale-95 text-white rounded-md text-xs font-semibold flex items-center gap-1.5 transition-all shadow-sm cursor-pointer"
        title="Create New Chat (Ctrl+N)"
      )
        svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 4v16m8-8H4")
        span New

      // Settings button
      button(
        @click="openSettings"
        class="p-1.5 bg-themeBackground3 hover:bg-themeBackground text-themeText2 hover:text-themeText rounded-md text-xs flex items-center justify-center border border-themeBorder2 transition-colors cursor-pointer"
        title="Chat Settings"
      )
        svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z")

    // Scrollable container with Swapy
    div(
      ref="scrollContainer"
      id="conversation-swapy-container"
      class="flex items-center gap-1.5 px-2 py-1.5 overflow-x-scroll flex-1"
      @wheel.prevent="handleWheel"
    )
      // Conversations tabs with Swapy slots
      div(
        v-for="(conv, index) in recentConversations"
        :key="conv.id"
        :data-swapy-slot="index"
        class="shrink-0"
      )
        div(
          :data-swapy-item="conv.id"
          @click="selectConversation(conv.id)"
          :class="['px-2.5 py-1 rounded-md text-xs cursor-pointer transition-all flex items-center gap-1.5 border group font-medium', conv.id === currentId ? 'bg-blue-600 text-white border-blue-500 shadow-sm' : 'bg-themeBackground3/60 hover:bg-themeBackground3 text-themeText2 hover:text-themeText border-themeBorder2']"
          :title="getConversationTooltip(conv)"
        )
          span(class="truncate max-w-[130px]") {{ conv.title }}
          span(class="text-[10px] opacity-70") {{ conv.message_count }}
          button(
            @click.stop="(e) => { e.preventDefault(); popoutConversation(conv.id); }"
            class="opacity-0 group-hover:opacity-100 hover:bg-white/20 rounded p-0.5 transition-all cursor-pointer"
            :class="conv.id === currentId ? 'text-white' : 'text-themeText hover:text-blue-400'"
            title="Pop Out into Separate Window"
          )
            svg(class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14")
          button(
            @click.stop="(e) => { e.preventDefault(); deleteConversation(conv.id); }"
            class="opacity-0 group-hover:opacity-100 hover:bg-red-500/20 rounded p-0.5 transition-all cursor-pointer"
            :class="conv.id === currentId ? 'text-white' : 'text-red-400'"
            title="Delete"
          )
            svg(class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

      // Show all button if there are more conversations
      button(
        v-if="hasMore"
        @click="showAll"
        class="shrink-0 px-2 py-1.5 text-xs text-themeText2 hover:text-themeText transition-colors"
      )
        span +{{ conversations.length - maxVisible }} more
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";
import { createSwapy } from "swapy";

interface ConversationMetadata {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  model: string;
  provider: string;
  message_count: number;
  total_tokens: number;
}

const conversations = ref<ConversationMetadata[]>([]);
const currentId = defineModel<string | null>("currentId");
const maxVisible = ref(10);
const scrollContainer = ref<HTMLElement>();
let swapy: any = null;

const emit = defineEmits<{
  select: [id: string];
  create: [];
  showAllClick: [];
  openSettings: [];
}>();

const recentConversations = computed(() => {
  return conversations.value.slice(0, maxVisible.value);
});

const hasMore = computed(() => {
  return conversations.value.length > maxVisible.value;
});

async function loadConversations() {
  try {
    conversations.value = await invoke<ConversationMetadata[]>("list_conversations");
  } catch (err) {
    console.error("Failed to load conversations:", err);
  }
}

function selectConversation(id: string) {
  currentId.value = id;
  emit("select", id);
}

function createNew() {
  currentId.value = null;
  emit("create");
}

async function deleteConversation(id: string) {
  // Tauri's dialog is async, must await it
  const confirmed = await ask("Delete this conversation? This cannot be undone.", {
    title: "Confirm Delete",
    type: "warning",
  });

  if (!confirmed) {
    return;
  }

  try {
    await invoke("delete_conversation", { conversationId: id });

    // Remove from local state
    conversations.value = conversations.value.filter((c) => c.id !== id);

    // If deleted conversation was active, clear it
    // But DON'T auto-create a new one - let user do that manually
    if (currentId.value === id) {
      currentId.value = null;
      // Don't emit('create') - user can click "New Chat" button if they want
    }
  } catch (err) {
    console.error("Failed to delete conversation:", err);
    // If deletion failed (e.g., file doesn't exist), still remove from UI
    if (err.toString().includes("not found") || err.toString().includes("No such file")) {
      console.log("File not found, removing from UI anyway");
      conversations.value = conversations.value.filter((c) => c.id !== id);
      if (currentId.value === id) {
        currentId.value = null;
      }
    }
  }
}

async function popoutConversation(convId: string) {
  try {
    const windowLabel = `assistant-${convId}`;
    await invoke("open_assistant_window", {
      windowLabel,
      conversationId: convId,
    });
  } catch (error) {
    console.error("Failed to pop out conversation:", error);
  }
}

function showAll() {
  emit("showAllClick");
}

function openSettings() {
  emit("openSettings");
}

function handleWheel(event: WheelEvent) {
  if (scrollContainer.value) {
    // Scroll horizontally when user scrolls vertically
    scrollContainer.value.scrollLeft += event.deltaY;
  }
}

function getConversationTooltip(conv: ConversationMetadata): string {
  const created = new Date(conv.created_at).toLocaleString();
  const updated = new Date(conv.updated_at).toLocaleString();
  return `${conv.title}\n\nMessages: ${conv.message_count}\nModel: ${conv.model}\nProvider: ${conv.provider}\n\nCreated: ${created}\nUpdated: ${updated}`;
}

onMounted(async () => {
  await loadConversations();

  // Initialize Swapy for drag-and-drop reordering
  const swapyContainer = document.querySelector("#conversation-swapy-container");
  if (swapyContainer) {
    swapy = createSwapy(swapyContainer, {
      animation: "dynamic",
      swapMode: "drop",
    });

    // Handle swap event to save new order
    swapy.onSwap(async (event: any) => {
      console.log("Conversation tabs swapped:", event);

      // Extract the new order from the swap event
      const newOrder: string[] = [];
      Object.keys(event.data.array).forEach((slotId) => {
        const itemId = event.data.array[slotId];
        if (itemId) newOrder.push(itemId);
      });

      // Reorder conversations array
      conversations.value = newOrder
        .map((id) => conversations.value.find((c) => c.id === id))
        .filter((c) => c !== undefined) as ConversationMetadata[];

      // TODO: Save order to localStorage or backend
      console.log("New conversation order:", newOrder);
    });
  }
});

onUnmounted(() => {
  if (swapy) {
    swapy.destroy();
  }
});

defineExpose({ loadConversations });
</script>

<style scoped>
/* Minimalistic horizontal scrollbar */
.conversation-history-bar {
  scrollbar-width: thin; /* Firefox */
  scrollbar-color: rgba(255, 255, 255, 0.15) transparent; /* Firefox */
}

.conversation-history-bar::-webkit-scrollbar {
  height: 2px; /* Thin and minimal */
}

.conversation-history-bar::-webkit-scrollbar-track {
  background: transparent; /* Invisible track */
}

.conversation-history-bar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
}

.conversation-history-bar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.25);
}

/* Remove scrollbar arrow buttons (left/right arrows) */
.conversation-history-bar::-webkit-scrollbar-button {
  display: none !important;
  width: 0 !important;
  height: 0 !important;
}
</style>
