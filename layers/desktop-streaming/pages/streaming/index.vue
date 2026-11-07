<template lang="pug">
div(class="h-full flex flex-col bg-themeBackground overflow-hidden")

  // Top Action Header
  div(class="px-8 py-5 border-b border-themeBorder flex items-center justify-between bg-themeBackground/80 backdrop-blur shrink-0")
    div(class="flex items-center gap-3.5")
      div(class="w-10 h-10 rounded-lg bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400 shrink-0")
        MaterialSymbolsVideoLibraryOutline(class="w-5 h-5")
      div
        div(class="flex items-center gap-2.5")
          h1(class="text-xl font-bold text-themeText tracking-tight") Broadcast Studio
          span(v-if="filteredActiveSessions.length > 0" class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-red-500/10 text-red-400 border border-red-500/20")
            span(class="w-1.5 h-1.5 rounded-full bg-red-500 animate-pulse")
            | {{ filteredActiveSessions.length }} LIVE
        p(class="text-xs text-themeText2 mt-0.5") Central dashboard for sessions, broadcast overlays, and unified chat

    button(
      @click="openCreateSessionModal"
      class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold flex items-center gap-2 transition-colors shadow-sm cursor-pointer"
    )
      MaterialSymbolsAddCircleOutline(class="w-4 h-4")
      span New Session

  // Main Scrollable Workspace
  div(class="flex-1 overflow-y-auto px-8 py-6 space-y-6")

    // Studio Modules Hub
    div(class="grid grid-cols-1 sm:grid-cols-3 lg:grid-cols-3 gap-4 lg:gap-5")
      // Live Chat Card
      div(class="p-4 bg-themeBackground2 rounded-2xl border border-themeBorder shadow-sm flex flex-col gap-y-3.5")
        div(class="flex w-full justify-start items-center gap-x-3")
          MaterialSymbolsChatOutline(class="w-5 h-5 text-emerald-500")
          p(class="text-sm font-semibold text-themeText") Live Chat
          div(class="ml-auto px-2.5 py-0.5 rounded-full text-xs font-medium bg-themeBackground3 text-themeText2 border border-themeBorder/60")
            span {{ totalMessages }} msgs

        NuxtLink(
          to="/streaming/chat"
          class="mt-auto px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold flex items-center justify-center gap-1.5 transition-all shadow-sm"
        )
          span Open Chat Console

      // Overlay Canvas Card
      div(class="p-4 bg-themeBackground2 rounded-2xl border border-themeBorder shadow-sm flex flex-col gap-y-3.5")
        div(class="flex w-full justify-start items-center gap-x-3")
          MaterialSymbolsLayersOutline(class="w-5 h-5 text-themePrimary")
          p(class="text-sm font-semibold text-themeText") Overlay Canvas
          div(class="ml-auto px-2.5 py-0.5 rounded-full text-xs font-medium bg-themePrimary/10 text-themePrimary border border-themePrimary/20")
            span Ready

        button(
          @click="openOverlaysWindow"
          class="mt-auto px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold flex items-center justify-center gap-1.5 transition-all shadow-sm cursor-pointer"
        )
          span Launch Overlay Editor

      // Virtual OBS Card
      div(class="p-4 bg-themeBackground2 rounded-2xl border border-themeBorder shadow-sm flex flex-col gap-y-3.5")
        div(class="flex w-full justify-start items-center gap-x-3")
          MaterialSymbolsCloudOutline(class="w-5 h-5 text-blue-400")
          p(class="text-sm font-semibold text-themeText") Virtual OBS
          div(class="ml-auto px-2.5 py-0.5 rounded-full text-xs font-medium bg-themeBackground3 text-themeText2 border border-themeBorder/60")
            span Standby

        button(
          type="button"
          disabled
          class="mt-auto px-4 py-2 bg-themeBackground3 text-themeText3 rounded-lg text-xs font-semibold flex items-center justify-center gap-1.5 border border-themeBorder/60 cursor-not-allowed opacity-75"
        )
          span Relay Standby

    // Search & Filter Toolbar
    div(class="flex flex-col sm:flex-row items-center justify-between gap-3 pt-2")
      // Search Bar
      div(class="relative w-full sm:w-80")
        MaterialSymbolsSearchRounded(class="w-4 h-4 text-themeText3 absolute left-3 top-1/2 -translate-y-1/2")
        input(
          v-model="searchQuery"
          type="text"
          placeholder="Filter sessions..."
          class="w-full pl-9 pr-8 py-2 bg-themeBackground2 border border-themeBorder rounded-xl text-xs text-themeText placeholder-themeText3 focus:outline-none focus:border-blue-500 shadow-2xs transition-colors"
        )
        button(
          v-if="searchQuery"
          @click="searchQuery = ''"
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-themeText3 hover:text-themeText"
        )
          MaterialSymbolsCloseRounded(class="w-3.5 h-3.5")

      // Segmented Filter Tabs
      div(class="flex items-center p-1 bg-themeBackground2 border border-themeBorder rounded-xl gap-1 w-full sm:w-auto overflow-x-auto shadow-2xs")
        button(
          @click="statusFilter = 'all'"
          :class="statusFilter === 'all' ? 'bg-themeBackground3 text-themeText font-semibold shadow-xs border border-themeBorder' : 'text-themeText2 hover:text-themeText'"
          class="px-3 py-1.5 rounded-lg text-xs transition-colors whitespace-nowrap cursor-pointer"
        )
          | All ({{ activeSessions.length + inactiveSessions.length + draftSessions.length }})

        button(
          @click="statusFilter = 'active'"
          :class="statusFilter === 'active' ? 'bg-themeBackground3 text-themeText font-semibold shadow-xs border border-themeBorder' : 'text-themeText2 hover:text-themeText'"
          class="px-3 py-1.5 rounded-lg text-xs transition-colors whitespace-nowrap flex items-center gap-1.5 cursor-pointer"
        )
          span(v-if="activeSessions.length > 0" class="w-1.5 h-1.5 rounded-full bg-red-500")
          | Active ({{ activeSessions.length }})

        button(
          @click="statusFilter = 'inactive'"
          :class="statusFilter === 'inactive' ? 'bg-themeBackground3 text-themeText font-semibold shadow-xs border border-themeBorder' : 'text-themeText2 hover:text-themeText'"
          class="px-3 py-1.5 rounded-lg text-xs transition-colors whitespace-nowrap cursor-pointer"
        )
          | Library ({{ inactiveSessions.length }})

        button(
          @click="statusFilter = 'draft'"
          :class="statusFilter === 'draft' ? 'bg-themeBackground3 text-themeText font-semibold shadow-xs border border-themeBorder' : 'text-themeText2 hover:text-themeText'"
          class="px-3 py-1.5 rounded-lg text-xs transition-colors whitespace-nowrap cursor-pointer"
        )
          | Drafts ({{ draftSessions.length }})

    // Active Live Sessions
    div(v-if="filteredActiveSessions.length > 0" class="space-y-2.5")
      div(class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-red-400")
        span(class="w-2 h-2 rounded-full bg-red-500 animate-pulse")
        | Live Sessions ({{ filteredActiveSessions.length }})

      div(class="bg-themeBackground2 border border-red-500/30 rounded-2xl divide-y divide-themeBorder/40 overflow-hidden shadow-sm")
        div(
          v-for="session in filteredActiveSessions"
          :key="session.id"
          @click="viewChat(session)"
          class="group flex items-center justify-between px-4 py-3 hover:bg-themeBackground3/50 transition-colors cursor-pointer gap-4"
        )
          div(class="flex items-center gap-3 min-w-0 flex-1")
            span(class="px-2 py-0.5 bg-red-500/10 text-red-400 rounded-md text-[10px] font-bold border border-red-500/20 shrink-0") LIVE
            span(class="font-semibold text-themeText text-sm truncate") {{ session.name }}
            span(class="bg-themeBackground3/70 px-2 py-0.5 rounded-md border border-themeBorder/60 text-xs text-themeText2 font-medium shrink-0") {{ getSessionPlatformsDisplay(session) }}

          div(class="flex items-center gap-4 shrink-0")
            span(class="font-mono text-xs text-themeText2") {{ session.duration || '00:00:00' }}
            div(class="flex items-center gap-2")
              button(
                @click.stop="viewChat(session)"
                class="px-3 py-1.5 text-xs bg-themeBackground2 hover:bg-themeBackground3 text-themeText rounded-lg border border-themeBorder font-medium transition-colors cursor-pointer"
              ) Chat Console
              button(
                @click.stop="configureSession(session)"
                class="px-2.5 py-1.5 text-xs text-themeText2 hover:text-themeText bg-themeBackground2 hover:bg-themeBackground3 rounded-lg border border-themeBorder transition-colors cursor-pointer"
              ) Configure
              button(
                @click.stop="endSession(session)"
                class="px-2.5 py-1.5 text-xs text-red-400 hover:text-red-300 bg-red-500/10 hover:bg-red-500/20 rounded-lg border border-red-500/20 transition-colors cursor-pointer"
              ) End

    // Saved Sessions Library
    div(v-if="filteredInactiveSessions.length > 0" class="space-y-2.5")
      div(class="flex items-center justify-between")
        span(class="text-xs font-semibold uppercase tracking-wider text-themeText2")
          | Session Library ({{ filteredInactiveSessions.length }})

      div(class="bg-themeBackground2 border border-themeBorder rounded-2xl divide-y divide-themeBorder/40 overflow-hidden shadow-sm")
        div(
          v-for="session in filteredInactiveSessions"
          :key="session.id"
          @click="viewHistory(session)"
          class="group flex items-center justify-between px-4 py-3 hover:bg-themeBackground3/50 transition-colors cursor-pointer gap-4"
        )
          div(class="flex items-center gap-3 min-w-0 flex-1")
            div(class="w-8 h-8 rounded-lg bg-themeBackground3/80 border border-themeBorder flex items-center justify-center text-themeText3 shrink-0 group-hover:text-themeText group-hover:border-themeBorder2 transition-colors shadow-2xs")
              MaterialSymbolsVideoLibraryOutline(class="w-4 h-4")
            span(class="font-semibold text-themeText text-sm truncate") {{ session.name }}
            span(class="bg-themeBackground3/70 px-2 py-0.5 rounded-md border border-themeBorder/60 text-xs text-themeText2 font-medium shrink-0")
              | {{ getSessionPlatformsDisplay(session) }}
            span(v-if="session.total_runs" class="text-xs text-themeText3 shrink-0 hidden md:inline")
              | • {{ session.total_runs }} runs
            span(v-if="session.last_used_at" class="text-xs text-themeText3 shrink-0 hidden lg:inline")
              | • {{ formatTimeAgo(session.last_used_at) }}

          div(class="flex items-center gap-2 shrink-0")
            button(
              @click.stop="editSession(session)"
              class="px-3 py-1.5 text-xs text-themeText2 hover:text-themeText bg-themeBackground2 hover:bg-themeBackground3 rounded-lg border border-themeBorder transition-colors whitespace-nowrap cursor-pointer"
            ) Edit
            button(
              @click.stop="deleteSessionNow(session)"
              class="p-1.5 text-themeText3 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-colors cursor-pointer"
              title="Delete session"
            )
              MaterialSymbolsDeleteOutline(class="w-4 h-4")
            button(
              @click.stop="activateSessionNow(session)"
              class="px-3.5 py-1.5 text-xs bg-themeButton hover:bg-themeButtonHover text-white rounded-lg font-medium transition-colors whitespace-nowrap shadow-xs cursor-pointer"
            ) Go Live

    // Draft Sessions
    div(v-if="filteredDraftSessions.length > 0" class="space-y-2.5")
      div(class="flex items-center justify-between")
        span(class="text-xs font-semibold uppercase tracking-wider text-themeText2")
          | Drafts ({{ filteredDraftSessions.length }})

      div(class="bg-themeBackground2 border border-themeBorder rounded-2xl divide-y divide-themeBorder/40 overflow-hidden shadow-sm")
        div(
          v-for="session in filteredDraftSessions"
          :key="session.id"
          @click="editSession(session)"
          class="group flex items-center justify-between px-4 py-3 hover:bg-themeBackground3/50 transition-colors cursor-pointer gap-4"
        )
          div(class="flex items-center gap-3 min-w-0 flex-1")
            span(class="px-2 py-0.5 bg-themeBackground3 text-themeText3 rounded-md text-[10px] font-mono border border-themeBorder shrink-0") Draft
            span(class="font-semibold text-themeText text-sm truncate") {{ session.name }}
            span(class="bg-themeBackground3/70 px-2 py-0.5 rounded-md border border-themeBorder/60 text-xs text-themeText2 font-medium shrink-0")
              | {{ getSessionPlatformsDisplay(session) }}
            span(v-if="session.commands?.length" class="text-xs text-themeText3 shrink-0 hidden md:inline")
              | • {{ session.commands.length }} commands

          div(class="flex items-center gap-2 shrink-0")
            button(
              @click.stop="editSession(session)"
              class="px-3 py-1.5 text-xs text-themeText2 hover:text-themeText bg-themeBackground2 hover:bg-themeBackground3 rounded-lg border border-themeBorder transition-colors whitespace-nowrap cursor-pointer"
            ) Edit
            button(
              @click.stop="deleteSessionNow(session)"
              class="p-1.5 text-themeText3 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-colors cursor-pointer"
              title="Delete draft"
            )
              MaterialSymbolsDeleteOutline(class="w-4 h-4")
            button(
              @click.stop="activateSessionNow(session)"
              class="px-3.5 py-1.5 text-xs bg-themeButton hover:bg-themeButtonHover text-white rounded-lg font-medium transition-colors whitespace-nowrap shadow-xs cursor-pointer"
            ) Go Live

    // No Results State
    div(
      v-if="(searchQuery || statusFilter !== 'all') && filteredActiveSessions.length === 0 && filteredDraftSessions.length === 0 && filteredInactiveSessions.length === 0"
      class="flex flex-col items-center justify-center py-16 text-center"
    )
      div(class="w-12 h-12 rounded-xl bg-themeBackground2 border border-themeBorder2 flex items-center justify-center text-themeText3 mb-3")
        MaterialSymbolsSearchRounded(class="w-6 h-6")
      h3(class="text-sm font-semibold text-themeText mb-1") No matching sessions
      p(class="text-xs text-themeText2 mb-4") Adjust search query or status filters
      button(
        @click="clearFilters"
        class="px-3 py-1.5 bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 rounded-lg text-xs text-themeText transition-colors"
      ) Clear Filters

    // Empty Studio State
    div(
      v-else-if="activeSessions.length === 0 && draftSessions.length === 0 && inactiveSessions.length === 0"
      class="flex flex-col items-center justify-center py-20 text-center"
    )
      div(class="w-14 h-14 rounded-2xl bg-themeBackground2 border border-themeBorder2 flex items-center justify-center text-themeText3 mb-4")
        MaterialSymbolsVideoLibraryOutline(class="w-7 h-7")
      h3(class="text-base font-semibold text-themeText mb-1") No broadcast sessions created
      p(class="text-xs text-themeText2 max-w-sm mb-5")
        | Set up a broadcast session to organize your stream platform channels, overlay presets, and chat integrations.

      button(
        @click="openCreateSessionModal"
        class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-semibold flex items-center gap-2 transition-colors shadow-sm"
      )
        MaterialSymbolsAddCircleOutline(class="w-4 h-4")
        span Create First Session

  // Session Configuration Modal
  StreamingSessionConfigModal(
    v-model="showCreateSessionModal"
    :session="selectedSession"
    @save="handleSaveSession"
    @update:modelValue="(value) => { if (!value) selectedSession = null; }"
  )
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import MaterialSymbolsVideoLibraryOutline from "~icons/material-symbols/video-library-outline";
import MaterialSymbolsAddCircleOutline from "~icons/material-symbols/add-circle-outline";
import MaterialSymbolsChatOutline from "~icons/material-symbols/chat-outline";
import MaterialSymbolsLayersOutline from "~icons/material-symbols/layers-outline";
import MaterialSymbolsCloudOutline from "~icons/material-symbols/cloud-outline";
import MaterialSymbolsDeleteOutline from "~icons/material-symbols/delete-outline";
import MaterialSymbolsSearchRounded from '~icons/material-symbols/search-rounded';
import MaterialSymbolsCloseRounded from '~icons/material-symbols/close-rounded';

import { useSessionManager, type SessionStatus } from "../../composables/streaming/useSessionManager";

// Define layout
definePageMeta({
  layout: "streaming",
  title: "Streaming Control Center",
  description: "Manage sessions, chat, overlays, and virtual instances",
});

const router = useRouter();

// Session management
const sessionManager = useSessionManager();
const {
  loadSessions,
  activeSessions: activeSessionsList,
  draftSessions: draftSessionsList,
  inactiveSessions: inactiveSessionsList,
  recentSessions,
  endSession: endSessionAction,
  createSession,
  updateSession,
  deleteSession,
  activateSession,
} = sessionManager;

// Use the loaded sessions
const activeSessions = activeSessionsList;
const draftSessions = draftSessionsList;
const inactiveSessions = inactiveSessionsList;

// Modal state
const showCreateSessionModal = ref(false);
const selectedSession = ref(null);

// Search and filter state
const searchQuery = ref("");
const statusFilter = ref("all");

const totalMessages = computed(() => 0);
const overlayCount = computed(() => 0);
const virtualInstanceCount = computed(() => 0);

// Filter sessions based on search query and status filter
function filterSessions(sessions: any[]) {
  let filtered = sessions;

  // Apply search filter
  if (searchQuery.value.trim()) {
    const query = searchQuery.value.toLowerCase().trim();
    filtered = filtered.filter((session) => {
      if (session.name?.toLowerCase().includes(query)) return true;

      if (session.platforms) {
        const platformsText = getSessionPlatformsDisplay(session).toLowerCase();
        if (platformsText.includes(query)) return true;
      }

      if (session.tags && Array.isArray(session.tags)) {
        return session.tags.some((tag: string) => tag.toLowerCase().includes(query));
      }

      return false;
    });
  }

  return filtered;
}

// Filtered session lists
const filteredActiveSessions = computed(() => {
  if (statusFilter.value === "all" || statusFilter.value === "active") {
    return filterSessions(activeSessions.value);
  }
  return [];
});

const filteredInactiveSessions = computed(() => {
  if (statusFilter.value === "all" || statusFilter.value === "inactive") {
    return filterSessions(inactiveSessions.value);
  }
  return [];
});

const filteredDraftSessions = computed(() => {
  if (statusFilter.value === "all" || statusFilter.value === "draft") {
    return filterSessions(draftSessions.value);
  }
  return [];
});

function clearFilters() {
  searchQuery.value = "";
  statusFilter.value = "all";
}

onMounted(async () => {
  await loadSessions();
});

function openCreateSessionModal() {
  selectedSession.value = null;
  showCreateSessionModal.value = true;
}

function editSession(session: any) {
  selectedSession.value = session;
  showCreateSessionModal.value = true;
}

async function handleSaveSession(sessionData: any) {
  try {
    if (selectedSession.value) {
      await updateSession(selectedSession.value.id, sessionData);
    } else {
      await createSession(sessionData);
    }
    showCreateSessionModal.value = false;
    selectedSession.value = null;
  } catch (error) {
    console.error("Failed to save session:", error);
    alert("Failed to save session. Please try again.");
  }
}

async function activateSessionNow(session: any) {
  try {
    await activateSession(session.id);
    console.log("Session activated:", session.name);
  } catch (error) {
    console.error("Failed to activate session:", error);
    alert(`Failed to activate session: ${error.message}`);
  }
}

async function deleteSessionNow(session: any) {
  if (confirm(`Delete session "${session.name}"?`)) {
    try {
      await deleteSession(session.id);
      console.log("Session deleted:", session.name);
    } catch (error) {
      console.error("Failed to delete session:", error);
      alert("Failed to delete session. Please try again.");
    }
  }
}

function viewChat(session: any) {
  router.push(`/streaming/chat?session=${session.id}`);
}

function configureSession(session: any) {
  editSession(session);
}

async function openOverlaysWindow() {
  try {
    await invoke("open_overlay_window");
  } catch (error) {
    console.error("Failed to open overlay window:", error);
  }
}

async function endSession(session: any) {
  if (confirm(`End session "${session.name}"?`)) {
    try {
      await endSessionAction(session.id, false);
      console.log("Session ended:", session);
    } catch (err) {
      console.error("Failed to end session:", err);
      alert("Failed to end session. Please try again.");
    }
  }
}

function viewHistory(session: any) {
  selectedSession.value = session;
  showCreateSessionModal.value = true;
}

function getSessionPlatformsDisplay(session: any): string {
  if (!session?.platforms || session.platforms.length === 0) {
    return 'No platforms';
  }

  if (typeof session.platforms[0] === 'object' && session.platforms[0] !== null) {
    const platformNames = session.platforms.map((p: any) => {
      if (!p || typeof p.platform !== 'string') {
        return 'Unknown';
      }
      const name = p.platform.charAt(0).toUpperCase() + p.platform.slice(1);
      return p.account_id ? `${name} (${p.account_id})` : name;
    });
    return platformNames.join(' + ');
  }

  return session.platforms.map((p: any) => {
    if (typeof p === 'string') {
      return p.charAt(0).toUpperCase() + p.slice(1);
    }
    return 'Unknown';
  }).join(' + ');
}

function formatTimeAgo(timestamp: string): string {
  const now = new Date().getTime();
  const then = new Date(timestamp).getTime();
  const diff = now - then;

  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ago`;
  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return 'just now';
}
</script>
