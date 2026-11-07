<template lang="pug">
  div(class="conversation-sidebar bg-themeBackground2 border-r border-themeBorder2 flex flex-col w-64")
    // Header
    div(class="p-3 border-b border-themeBorder2")
      button(
        @click="createNew"
        class="w-full px-3 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm flex items-center justify-center gap-2 transition-colors"
      )
        svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4")
        span New Chat

    // Search
    div(class="p-2 border-b border-themeBorder2")
      input(
        v-model="searchQuery"
        placeholder="Search..."
        class="w-full px-2 py-1.5 bg-themeBackground text-themeText text-xs rounded border border-themeBorder2 focus:outline-none focus:ring-1 focus:ring-blue-500"
      )

    // Conversation list
    div(class="flex-1 overflow-y-auto")
      div(
        v-for="conv in filteredConversations"
        :key="conv.id"
        :class="['p-3 border-b border-themeBorder2 transition-colors hover:bg-themeBackground3 group', { 'bg-blue-500/10 border-l-2 border-l-blue-500': conv.id === currentId }]"
      )
        // Edit mode
        div(v-if="editingId === conv.id" class="flex items-center gap-2" @click.stop)
          input(
            ref="editInput"
            v-model="editingTitle"
            @keydown.enter="saveTitle(conv.id)"
            @keydown.esc="cancelEdit"
            class="flex-1 px-2 py-1 bg-themeBackground text-themeText text-sm rounded border border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
          )
          button(
            @click="saveTitle(conv.id)"
            class="p-1 hover:bg-green-500/20 rounded text-green-400 transition-colors"
            title="Save"
          )
            svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
          button(
            @click="cancelEdit"
            class="p-1 hover:bg-red-500/20 rounded text-red-400 transition-colors"
            title="Cancel"
          )
            svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        // Normal mode
        div(v-else @click="selectConversation(conv.id)" class="cursor-pointer")
          div(class="flex items-center justify-between")
            div(class="text-sm font-medium text-themeText truncate flex-1") {{ conv.title }}
            div(class="flex items-center gap-1")
              button(
                @click.stop="startEdit(conv)"
                class="p-1 opacity-0 group-hover:opacity-100 hover:bg-blue-500/20 rounded text-blue-400 transition-all"
                title="Edit title"
              )
                svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z")
              button(
                @click.stop="(e) => { e.preventDefault(); deleteConversation(conv.id); }"
                class="p-1 opacity-0 group-hover:opacity-100 hover:bg-red-500/20 rounded text-red-400 transition-all"
                title="Delete conversation"
              )
                svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16")
          div(class="flex items-center justify-between mt-1")
            span(class="text-xs text-themeText2") {{ formatDate(conv.updated_at) }}
            span(class="text-xs text-themeText2") {{ conv.message_count }} msgs

    // Empty state
    div(v-if="conversations.length === 0" class="p-4 text-center text-xs text-themeText2")
      p No conversations yet
      p(class="mt-1") Click "New Chat" to start
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';

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
const searchQuery = ref('');
const currentId = defineModel<string | null>('currentId');

// Edit mode state
const editingId = ref<string | null>(null);
const editingTitle = ref('');
const editInput = ref<HTMLInputElement>();

const emit = defineEmits<{
  select: [id: string];
  create: [];
}>();

const filteredConversations = computed(() => {
  if (!searchQuery.value) return conversations.value;

  const query = searchQuery.value.toLowerCase();
  return conversations.value.filter(c =>
    c.title.toLowerCase().includes(query)
  );
});

async function loadConversations() {
  try {
    conversations.value = await invoke<ConversationMetadata[]>('list_conversations');
  } catch (err) {
    console.error('Failed to load conversations:', err);
  }
}

function selectConversation(id: string) {
  currentId.value = id;
  emit('select', id);
}

function createNew() {
  currentId.value = null;
  emit('create');
}

function startEdit(conv: ConversationMetadata) {
  editingId.value = conv.id;
  editingTitle.value = conv.title;

  nextTick(() => {
    editInput.value?.focus();
    editInput.value?.select();
  });
}

async function saveTitle(id: string) {
  if (!editingTitle.value.trim()) {
    cancelEdit();
    return;
  }

  try {
    await invoke('update_conversation_title', {
      conversationId: id,
      newTitle: editingTitle.value.trim(),
    });

    // Update local state
    const conv = conversations.value.find(c => c.id === id);
    if (conv) {
      conv.title = editingTitle.value.trim();
    }

    cancelEdit();
  } catch (err) {
    console.error('Failed to update title:', err);
  }
}

function cancelEdit() {
  editingId.value = null;
  editingTitle.value = '';
}

async function deleteConversation(id: string) {
  console.log('Delete button clicked for:', id);

  // Tauri's confirm returns a Promise, must await it
  const confirmed = await ask('Delete this conversation? This cannot be undone.', {
    title: 'Confirm Delete',
    type: 'warning'
  });
  console.log('User confirmed?', confirmed);

  if (!confirmed) {
    console.log('User cancelled deletion');
    return;
  }

  console.log('Proceeding with deletion...');

  try {
    await invoke('delete_conversation', { conversationId: id });
    console.log('Successfully deleted from backend');

    // Remove from local state
    conversations.value = conversations.value.filter(c => c.id !== id);

    // If deleted conversation was active, clear it
    // But DON'T auto-create a new one - let user do that manually
    if (currentId.value === id) {
      currentId.value = null;
      // Don't emit('create') - user can click "New Chat" button if they want
    }
  } catch (err) {
    console.error('Failed to delete conversation:', err);
    // If deletion failed (e.g., file doesn't exist), still remove from UI
    if (err.toString().includes('not found') || err.toString().includes('No such file')) {
      console.log('File not found, removing from UI anyway');
      conversations.value = conversations.value.filter(c => c.id !== id);
      if (currentId.value === id) {
        currentId.value = null;
      }
    }
  }
}

function formatDate(dateStr: string) {
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return `${days}d ago`;
  return date.toLocaleDateString();
}

onMounted(() => {
  loadConversations();
});

defineExpose({ loadConversations });
</script>

<style scoped>
/* Custom scrollbar */
::-webkit-scrollbar {
  width: 4px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}
</style>
