<template lang="pug">
div(class="h-full flex flex-col bg-themeBackground overflow-hidden")

  // ===== SESSION-PERSONALIZED VIEW =====
  div(v-if="activeSession" class="h-full flex flex-col")

    // Session Header
    div(class="p-4 bg-gradient-to-r from-blue-500/10 to-purple-500/10 border-b border-blue-500/30")
      div(class="flex items-center justify-between")
        div(class="flex items-center gap-4")
          // Live indicator (only show when status is 'active')
          div(v-if="activeSession.status === 'active'" class="flex items-center gap-2")
            span(class="relative flex h-3 w-3")
              span(class="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-500 opacity-75")
              span(class="relative inline-flex rounded-full h-3 w-3 bg-red-500")
            span(class="text-red-400 font-semibold text-sm uppercase tracking-wide") LIVE

          // Session info
          div
            h2(class="text-lg font-bold text-themeText") {{ activeSession.name }}
            div(class="flex items-center gap-3 text-sm text-themeText2")
              span {{ getSessionPlatformsDisplay(activeSession) }}
              span(v-if="activeSession.status === 'active' && activeSession.duration")
                | • Duration {{ formatSessionDuration(activeSession.duration) }}
              span(v-else-if="activeSession.status !== 'active' && activeSession.endedAt")
                | • Ended {{ formatTimeAgo(activeSession.endedAt) }}
              span • {{ filteredMessages.length }} messages

        // Actions
        div(class="flex items-center gap-2")
          // Stream Selector
          select(
            v-model="selectedStreamId"
            @change="switchStream"
            class="px-3 py-1.5 text-sm text-themeText bg-themeBackground border border-themeBorder2 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          )
            option(value="all") All Streams
            option(
              v-for="stream in connectedStreams"
              :key="stream.id"
              :value="stream.id"
            ) {{ stream.platform }} - {{ stream.accountName }}

          button(
            @click="clearChat"
            class="px-3 py-1.5 text-sm text-themeText bg-themeBackground3 hover:bg-red-500/20 border border-themeBorder2 rounded transition-colors flex items-center gap-1.5"
          )
            MaterialSymbolsDeleteOutline(class="w-4 h-4")
            span Clear

          NuxtLink(
            to="/streaming"
            class="px-3 py-1.5 text-sm text-themeText bg-themeBackground3 hover:bg-themeBackground rounded transition-colors"
          ) ← Control Center
          button(
            @click="configureSession"
            class="px-3 py-1.5 text-sm bg-blue-500/20 hover:bg-blue-500/30 text-blue-400 rounded transition-colors"
          ) ⚙️ Configure
          button(
            @click="endSession"
            class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded transition-colors"
          ) ⏹️ End Session

    // Compact Controls Bar (Platforms + Commands)
    div(class="p-3 bg-themeBackground2 border-b border-themeBorder2")
      div(class="flex items-center justify-between gap-4 text-sm")

        // Left: Platform Filters
        div(class="flex items-center gap-2")
          span(class="text-themeText2 font-medium whitespace-nowrap") Platforms:
          div(class="flex items-center gap-2")
            button(
              v-for="platform in sessionPlatforms"
              :key="platform.id"
              @click="toggleSessionPlatform(platform.id)"
              :class="[  platformToggles[platform.id] === false ? 'bg-themeBackground opacity-60 text-themeText2' : 'bg-themeBackground3 border-blue-500 text-themeText',  'px-2.5 py-1 rounded border border-themeBorder2 transition-all text-xs font-medium flex items-center gap-1'  ]"
            )
              component(
                v-if="getPlatformIcon(platform.id)"
                :is="getPlatformIcon(platform.id)"
                class="w-3.5 h-3.5"
              )
              span {{ platform.name }}
              span(v-if="platform.messageCount > 0" class="ml-0.5 px-1 bg-blue-500 rounded text-xs text-white") {{ platform.messageCount }}

        // Right: Active Commands Summary
        div(v-if="activeSession.commands && activeSession.commands.length" class="flex items-center gap-2")
          span(class="text-themeText2 font-medium whitespace-nowrap") Commands:
          div(class="flex items-center gap-1.5")
            div(
              v-for="cmd in activeSession.commands.slice(0, 4)"
              :key="cmd.trigger"
              class="px-2 py-1 bg-blue-500/10 text-blue-400 rounded text-xs font-mono"
              :title="cmd.deckActionId"
            ) {{ "!" + cmd.trigger }}
            span(v-if="activeSession.commands.length > 4" class="text-themeText2 text-xs")
              | +{{ activeSession.commands.length - 4 }}

    // Main Chat Area
    div(class="flex-1 flex overflow-hidden")

      // Chat Messages (70%)
      div(class="flex-[7] flex flex-col border-r border-themeBorder2")

        // Messages Container
        div(class="flex-1 overflow-y-auto p-4 space-y-3" ref="chatContainer")

          // Message Item
          div(
            v-for="msg in filteredMessages"
            :key="msg.id"
            class="group flex items-start gap-3 p-3 hover:bg-themeBackground2 rounded-lg transition-colors"
          )
            // Platform Indicator
            div(
              :style="{ backgroundColor: msg.platform_color }"
              class="w-1 h-full rounded-full"
            )

            // Avatar
            div(
              class="w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold flex-shrink-0"
              :style="{ backgroundColor: getUserColor(msg.user_id), color: '#fff' }"
            )
              | {{ msg.user_display_name[0].toUpperCase() }}

            // Message Content
            div(class="flex-1 min-w-0")
              // User info
              div(class="flex items-center gap-2 flex-wrap mb-1")
                span(
                  v-for="badge in msg.badges"
                  :key="badge"
                  class="text-xs"
                  :title="badge"
                ) {{ badgeEmoji(badge) }}
                span(
                  class="font-semibold text-sm"
                  :style="{ color: getUserColor(msg.user_id) }"
                ) {{ msg.user_display_name }}
                span(
                  class="text-xs px-1.5 py-0.5 rounded"
                  :style="{ backgroundColor: msg.platform_color + '20', color: msg.platform_color }"
                ) {{ msg.platform }}
                span(class="text-xs text-themeText2") {{ formatTime(msg.timestamp) }}

              // Message text
              div(class="text-themeText break-words") {{ msg.message }}

              // Command execution feedback
              div(
                v-if="msg.commandExecuted"
                class="mt-2 flex items-center gap-2 px-3 py-2 bg-green-500/10 border border-green-500/20 rounded text-sm"
              )
                span(class="text-green-400") ✅ Executed:
                span(class="text-themeText font-medium") {{ msg.commandExecuted.name }}

            // Message Actions (shown on hover)
            div(class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity")
              button(
                @click="replyTo(msg)"
                class="p-1.5 hover:bg-themeBackground3 rounded transition-colors"
                title="Reply"
              )
                MaterialSymbolsReplyRounded(class="w-4 h-4")

              button(
                v-if="isStreamer"
                @click="deleteMessage(msg)"
                class="p-1.5 hover:bg-red-500/20 rounded transition-colors"
                title="Delete"
              )
                MaterialSymbolsDeleteOutline(class="w-4 h-4 text-red-400")

        // Message Input
        div(class="p-4 border-t border-themeBorder2 bg-themeBackground2")
          div(class="flex gap-2")
            input(
              v-model="messageInput"
              @keyup.enter="sendMessage"
              placeholder="Send to all session platforms..."
              class="flex-1 px-4 py-2.5 text-themeText bg-themeBackground border border-themeBorder2 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 transition-all placeholder-themeText3"
            )
            button(
              @click="sendMessage"
              :disabled="!messageInput.trim()"
              class="px-6 py-2.5 bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors font-medium text-white"
            ) Send

      // Session Stats Sidebar (30%)
      div(class="flex-[3] flex flex-col p-4 space-y-4 overflow-y-auto bg-themeBackground2")

        // Session Stats
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Session Stats
          div(class="space-y-2 text-sm")
            div(class="flex justify-between")
              span(class="text-themeText2") Status
              span(:class="activeSession.status === 'active' ? 'text-green-400' : 'text-themeText2'" class="font-semibold capitalize") {{ activeSession.status }}
            div(v-if="activeSession.duration" class="flex justify-between")
              span(class="text-themeText2") Duration
              span(class="text-themeText font-semibold") {{ formatSessionDuration(activeSession.duration) }}
            div(class="flex justify-between")
              span(class="text-themeText2") Messages
              span(class="text-green-400 font-semibold") {{ messageCount }}
            div(class="flex justify-between")
              span(class="text-themeText2") Commands Used
              span(class="text-blue-400 font-semibold") {{ commandExecutionCount }}
            div(class="flex justify-between")
              span(class="text-themeText2") Active Viewers
              span(class="text-purple-400 font-semibold") {{ viewerCount }}

        // Command Execution Log
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          div(class="flex items-center justify-between mb-3")
            h3(class="font-semibold text-themeText") Command Log
            button(
              @click="clearCommandLog"
              class="text-xs text-blue-400 hover:text-blue-300"
            ) Clear

          div(v-if="commandLog.length === 0" class="text-sm text-themeText2 text-center py-4")
            | No commands executed yet

          div(v-else class="space-y-2 max-h-64 overflow-y-auto")
            div(
              v-for="log in commandLog.slice(0, 10)"
              :key="log.id"
              class="flex items-center justify-between text-sm p-2 bg-themeBackground2 rounded"
            )
              div(class="flex items-center gap-2 flex-1 min-w-0")
                span(class="font-mono text-blue-400 text-xs") {{ "!" + log.command }}
                span(class="text-themeText2 text-xs truncate") by @{{ log.user }}
              span(class="text-themeText2 text-xs whitespace-nowrap") {{ log.time }}

        // Top Chatters
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Top Chatters
          div(class="space-y-2")
            div(
              v-for="(chatter, index) in topChatters.slice(0, 5)"
              :key="chatter.userId"
              class="flex items-center justify-between text-sm"
            )
              div(class="flex items-center gap-2")
                span(class="text-themeText2") {{ index + 1 }}.
                span(class="text-themeText") {{ chatter.name }}
              span(class="text-blue-400 font-semibold") {{ chatter.messageCount }}

        // Connection Status
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          div(class="flex items-center justify-between mb-2")
            h3(class="font-semibold text-themeText") Connection Status
            div(:class="[isConnected ? 'bg-green-500' : 'bg-red-500', 'w-2 h-2 rounded-full animate-pulse']")
          div(class="text-sm text-themeText2")
            | {{ isConnected ? 'Connected to cloud' : 'Connecting...' }}

        // Connected Streams
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Connected Streams

          div(v-if="connectedStreams.length === 0" class="text-sm text-themeText2 text-center py-4")
            | No streams connected

          div(v-else class="space-y-2")
            div(
              v-for="stream in connectedStreams"
              :key="stream.id"
              :class="[selectedStreamId === stream.id ? 'ring-2 ring-blue-500' : '', 'flex items-center justify-between p-3 bg-themeBackground2 rounded-lg hover:bg-themeBackground3 transition-all cursor-pointer']"
              @click="selectedStreamId = stream.id"
            )
              div(class="flex-1")
                div(class="flex items-center gap-2 mb-1")
                  div(:class="stream.isLive ? 'bg-red-500 animate-pulse' : 'bg-gray-500'" class="w-2 h-2 rounded-full")
                  span(class="font-medium text-sm text-themeText") {{ stream.platform }}
                div(class="text-xs text-themeText2") {{ stream.accountName }}
              span(class="text-sm text-themeText2") {{ stream.viewers.toLocaleString() }}

          // Total stats
          div(v-if="connectedStreams.length > 0" class="mt-4 pt-4 border-t border-themeBorder2")
            div(class="flex justify-between text-sm mb-2")
              span(class="text-themeText2") {{ selectedStreamId === 'all' ? 'Total Viewers' : 'Viewers' }}
              span(class="font-semibold text-green-400") {{ totalViewers.toLocaleString() }}
            div(class="flex justify-between text-sm")
              span(class="text-themeText2") Uptime
              span(class="font-semibold text-themeText") {{ uptime }}
            div(v-if="selectedStreamId === 'all'" class="flex justify-between text-sm mt-2")
              span(class="text-themeText2") Active Streams
              span(class="font-semibold text-blue-400") {{ connectedStreams.filter(s => s.isLive).length }}/{{ connectedStreams.length }}

        // Command Stats with Bar Charts
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          div(class="flex items-center justify-between mb-3")
            h3(class="font-semibold text-themeText") Command Stats
            button(@click="resetCommandStats" class="text-xs text-blue-400 hover:text-blue-300") Reset

          div(v-if="commandStats.length === 0" class="text-sm text-themeText2 text-center py-4")
            | No commands used yet

          div(v-else class="space-y-2")
            div(
              v-for="cmd in commandStats"
              :key="cmd.command"
              class="flex justify-between items-center text-sm"
            )
              span(class="font-mono text-blue-400") {{ cmd.command }}
              div(class="flex items-center gap-2")
                span(class="text-themeText2 text-xs") {{ cmd.count }}
                div(class="w-16 h-1 bg-gray-700 rounded-full overflow-hidden")
                  div(
                    class="h-full bg-blue-500"
                    :style="{ width: (cmd.count / maxCommandCount * 100) + '%' }"
                  )

        // Quick Actions
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Quick Actions
          div(class="space-y-2")
            NuxtLink(
              to="/deck"
              class="w-full px-3 py-2 bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 text-white rounded-lg text-center transition-colors flex items-center justify-center gap-2 text-sm font-medium"
            )
              MaterialSymbolsGridViewOutline(class="w-4 h-4")
              | Manage Triggers

            button(
              @click="openLogs"
              class="w-full px-3 py-2 text-themeText bg-themeBackground3 hover:bg-themeBackground rounded-lg transition-colors flex items-center justify-center gap-2 text-sm font-medium"
            )
              //- MaterialSymbolsHistoryOutline(class="w-4 h-4")
              | View Logs

            button(
              @click="toggleModMode"
              :class="[modMode ? 'bg-yellow-600 hover:bg-yellow-700 text-white' : 'bg-themeBackground3 hover:bg-themeBackground text-themeText', 'w-full px-3 py-2 rounded-lg transition-colors flex items-center justify-center gap-2 text-sm font-medium']"
            )
              MaterialSymbolsShieldOutline(class="w-4 h-4")
              | {{ modMode ? 'Mod Mode ON' : 'Mod Mode OFF' }}

            button(
              @click="openOverlays"
              class="w-full px-3 py-2 bg-purple-500/20 hover:bg-purple-500/30 text-purple-400 rounded transition-colors text-sm font-medium"
            ) 🎬 Open Overlays
            button(
              @click="openVirtualOBS"
              class="w-full px-3 py-2 bg-orange-500/20 hover:bg-orange-500/30 text-orange-400 rounded transition-colors text-sm font-medium"
            ) ☁️ Open Virtual OBS
            button(
              @click="exportChatLog"
              class="w-full px-3 py-2 bg-green-500/20 hover:bg-green-500/30 text-green-400 rounded transition-colors text-sm font-medium"
            ) 💾 Export Chat Log

        // Platform Connections
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Platform Connections
          div(class="space-y-2")
            button(
              v-for="platform in platforms"
              :key="platform.id"
              @click="connectPlatform(platform.id)"
              class="w-full px-3 py-2 text-themeText bg-themeBackground3 hover:bg-themeBackground rounded-lg transition-colors text-sm flex items-center justify-between"
            )
              div(class="flex items-center gap-2")
                component(:is="platform.icon" class="w-4 h-4")
                span {{ platform.name }}
              span(class="text-xs" :class="platform.enabled ? 'text-green-400' : 'text-themeText2'")
                | {{ platform.enabled ? 'Connected' : 'Connect' }}

  // ===== GLOBAL HUB VIEW (No Active Session) =====
  div(v-else class="h-full flex flex-col")

    // Header
    div(class="p-4 bg-themeBackground2 border-b border-themeBorder2")
      div(class="flex items-center justify-between")
        h1(class="text-xl font-bold text-themeText") 💬 Global Chat Hub
        NuxtLink(
          to="/streaming"
          class="px-3 py-1.5 text-themeText text-sm bg-themeBackground3 hover:bg-themeBackground rounded transition-colors"
        ) ← Back

    // No Session Banner
    div(class="p-6 bg-yellow-500/10 border-b border-yellow-500/30")
      div(class="max-w-2xl mx-auto text-center")
        div(class="text-4xl mb-3") ⚠️
        h2(class="text-lg font-semibold text-themeText mb-2") No Active Session
        p(class="text-themeText2 mb-4")
          | You're viewing global chat from all platforms. Create or activate a session to enable commands, personalized stats, and session management.
        div(class="flex items-center justify-center gap-3")
          NuxtLink(
            to="/streaming"
            class="px-4 py-2 bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 text-white rounded-lg font-medium transition-colors"
          ) + Create Session
          button(
            @click="viewAllSessions"
            class="px-4 py-2 text-themeText bg-themeBackground3 hover:bg-themeBackground rounded-lg transition-colors"
          ) View All Sessions

    // Global Chat View
    div(class="flex-1 flex overflow-hidden")

      // Messages (70%)
      div(class="flex-[7] flex flex-col border-r border-themeBorder2")

        // Platform Filters
        div(class="p-3 bg-themeBackground2 border-b border-themeBorder2")
          div(class="flex items-center gap-2 text-sm")
            span(class="text-themeText2") Filter:
            button(
              v-for="platform in platforms"
              :key="platform.id"
              @click="togglePlatform(platform.id)"
              :class="[ platform.enabled ? 'bg-themeBackground3 border-blue-500 text-themeText' : 'bg-themeBackground opacity-60 text-themeText2', 'px-3 py-1.5 rounded border border-themeBorder2 transition-all' ]"
            )
              | {{ platform.name }}

        // Messages
        div(class="flex-1 overflow-y-auto p-4 space-y-2")
          div(
            v-if="filteredMessages.length === 0"
            class="flex flex-col items-center justify-center h-full text-center"
          )
            div(class="text-6xl mb-4 opacity-50") 💬
            p(class="text-lg font-semibold text-themeText mb-2") No messages yet
            p(class="text-sm text-themeText2") Connect your streaming platforms to see chat

          // Message items (same as above)
          div(
            v-for="msg in filteredMessages"
            :key="msg.id"
            class="flex items-start gap-3 p-3 hover:bg-themeBackground2 rounded-lg transition-colors"
          )
            div(:style="{ backgroundColor: msg.platform_color }" class="w-1 h-full rounded-full")
            div(
              class="w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold"
              :style="{ backgroundColor: getUserColor(msg.user_id), color: '#fff' }"
            ) {{ msg.user_display_name[0].toUpperCase() }}
            div(class="flex-1 min-w-0")
              div(class="flex items-center gap-2 flex-wrap mb-1")
                span(class="font-semibold text-sm" :style="{ color: getUserColor(msg.user_id) }") {{ msg.user_display_name }}
                span(class="text-xs px-1.5 py-0.5 rounded" :style="{ backgroundColor: msg.platform_color + '20', color: msg.platform_color }") {{ msg.platform }}
                span(class="text-xs text-themeText2") {{ formatTime(msg.timestamp) }}
              div(class="text-themeText break-words") {{ msg.message }}

        // Read-only notice
        div(class="p-3 bg-themeBackground2 border-t border-themeBorder2 text-center")
          p(class="text-sm text-themeText2")
            | 🔒 Read-only mode • Create a session to send messages and execute commands

      // Stats Sidebar (30%)
      div(class="flex-[3] flex flex-col p-4 space-y-4 overflow-y-auto bg-themeBackground2")

        // Connection Status
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Connection Status
          div(class="space-y-2 text-sm")
            div(class="flex justify-between items-center")
              span(class="text-themeText2") Backend
              div(class="flex items-center gap-2")
                span(:class="isConnected ? 'bg-green-500' : 'bg-red-500'" class="w-2 h-2 rounded-full animate-pulse")
                span(:class="isConnected ? 'text-green-400' : 'text-red-400'") {{ isConnected ? 'Connected' : 'Disconnected' }}

        // Platform Stats
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Platforms
          div(class="space-y-2")
            div(
              v-for="platform in platforms"
              :key="platform.id"
              class="flex justify-between items-center text-sm"
            )
              span(class="text-themeText") {{ platform.name }}
              span(class="text-themeText2") {{ platform.messageCount }} msgs

        // Recent Sessions
        div(class="bg-themeBackground rounded-lg p-4 border border-themeBorder2")
          h3(class="font-semibold text-themeText mb-3") Recent Sessions
          div(class="space-y-2")
            div(
              v-for="session in recentSessions.slice(0, 3)"
              :key="session.id"
              class="p-2 bg-themeBackground2 rounded cursor-pointer hover:bg-themeBackground3 transition-colors"
              @click="activateSession(session)"
            )
              div(class="text-sm font-medium text-themeText") {{ session.name }}
              div(class="text-xs text-themeText2") {{ session.platforms.join(', ') }}

  // Session Configuration Modal
  StreamingSessionConfigModal(
    v-model="showConfigModal"
    :session="selectedSession"
    @save="handleSaveSession"
  )
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { useRouter, useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";

// Import composables

// Import icons
import MaterialSymbolsDeleteOutline from "~icons/material-symbols/delete-outline";
import MaterialSymbolsReplyRounded from "~icons/material-symbols/reply-rounded";
import MaterialSymbolsShieldOutline from "~icons/material-symbols/shield-outline";
// import MaterialSymbolsHistoryOutline from "~icons/material-symbols/history-outline";
import MaterialSymbolsGridViewOutline from "~icons/material-symbols/grid-view-outline";
import LogosTwitch from "~icons/logos/twitch";
import LogosYoutube from "~icons/logos/youtube-icon";
import SimpleIconsKick from "~icons/simple-icons/kick";

// Define layout
definePageMeta({
  layout: "overlay",
  title: "Streaming Chat",
  description: "Session-personalized chat interface with unified messaging",
});

const router = useRouter();
const route = useRoute();

// Use Rust-backed streaming connection composable (persistent across page reloads)
const streamingConnection = useStreamingRust();

// Use session manager to get desktop session data
const sessionManager = useSessionManager();

// Note: Session activation on stream_online should be handled in Rust, not frontend

type ChatMessage = UnifiedChatMessage;

// State
const messages = ref<ChatMessage[]>([]);
const messageInput = ref("");
const chatContainer = ref<HTMLElement | null>(null);
const isStreamer = ref(true);
const modMode = ref(false);
const startTime = ref(Date.now());
const selectedStreamId = ref("all");
const selectedSessionId = ref<string | null>(null);

// Get active session from URL query param or first active session
const activeSession = computed(() => {
  // If session ID from URL query param
  if (selectedSessionId.value) {
    // Try to find in active sessions
    const active = sessionManager.activeSessions.value.find(
      (s) => s.id === selectedSessionId.value
    );
    if (active) return active;

    // Try to find in draft sessions
    const draft = sessionManager.draftSessions.value.find(
      (s) => s.id === selectedSessionId.value
    );
    if (draft) return draft;

    // Try to find in recent sessions
    const recent = sessionManager.recentSessions.value.find(
      (s) => s.id === selectedSessionId.value
    );
    if (recent) return recent;
  }

  // Fallback to first active session
  const active = sessionManager.activeSessions.value;
  return active.length > 0 ? active[0] : null;
});

// Session platforms - dynamically derived from active session
const sessionPlatforms = computed(() => {
  if (!activeSession.value || !activeSession.value.platforms) return [];

  // Count messages per platform
  const platformCounts: Record<string, number> = {};
  messages.value.forEach((msg) => {
    platformCounts[msg.platform] = (platformCounts[msg.platform] || 0) + 1;
  });

  // Build platform list from session - deduplicate by platform name
  const uniquePlatforms = new Map<string, any>();

  activeSession.value.platforms.forEach((p) => {
    const platformId = typeof p === "string" ? p : p.platform;

    // Only add if not already in map (deduplicates)
    if (!uniquePlatforms.has(platformId)) {
      uniquePlatforms.set(platformId, {
        id: platformId,
        name: platformId.charAt(0).toUpperCase() + platformId.slice(1),
        enabled: true,
        messageCount: platformCounts[platformId] || 0,
      });
    }
  });

  return Array.from(uniquePlatforms.values());
});

// Store platform toggle states separately
const platformToggles = ref<Record<string, boolean>>({});

// Connection status
const isConnected = computed(() => streamingConnection.isConnected.value);

// Filtered messages based on enabled session platforms
const filteredMessages = computed(() => {
  let filtered = messages.value;

  if (!activeSession.value) {
    // Global hub mode - filter by global platforms
    const enabledPlatforms = platforms.value.filter((p) => p.enabled).map((p) => p.id);
    return filtered.filter((msg) => enabledPlatforms.includes(msg.platform));
  }

  // Session-personalized mode
  // Filter by account_ids from session platforms
  if (activeSession.value.platforms && activeSession.value.platforms.length > 0) {
    const sessionAccountIds = activeSession.value.platforms
      .map((p) => (typeof p === "object" && p.accountId ? String(p.accountId) : null))
      .filter((id) => id !== null);

    console.log("🔍 [CHAT FILTER] Session account IDs:", sessionAccountIds);
    console.log("🔍 [CHAT FILTER] Total messages before filter:", filtered.length);

    if (sessionAccountIds.length > 0) {
      filtered = filtered.filter((msg) => {
        const msgAccountId = String(msg.account_id);
        const isMatch = sessionAccountIds.includes(msgAccountId);
        if (!isMatch) {
          console.log(
            "❌ [CHAT FILTER] Message filtered out - account_id:",
            msgAccountId,
            "not in session"
          );
        }
        return isMatch;
      });
      console.log("✅ [CHAT FILTER] Messages after account filter:", filtered.length);
    } else {
      // Fallback: filter by platform if account_ids not available
      console.log(
        "⚠️ [CHAT FILTER] No account IDs found, falling back to platform filter"
      );
      const sessionPlatformIds = new Set<string>(activeSession.value.platforms.map((p) =>
        typeof p === "string" ? p : p.platform
      ));
      console.log("🔍 [CHAT FILTER] Session platforms:", sessionPlatformIds);
      filtered = filtered.filter((msg) => sessionPlatformIds.has(msg.platform));
      console.log("✅ [CHAT FILTER] Messages after platform filter:", filtered.length);
    }
  }

  // Apply platform toggles
  const enabledPlatforms = sessionPlatforms.value
    .filter((p) => platformToggles.value[p.id] !== false)
    .map((p) => p.id);

  filtered = filtered.filter((msg) => enabledPlatforms.includes(msg.platform));

  return filtered;
});

const messageCount = computed(() => filteredMessages.value.length);

// Command stats (derived from messages)
const commandStats = computed(() => {
  const stats: Record<string, number> = {};
  messages.value.forEach((msg) => {
    if (msg.message.startsWith("!")) {
      const command = msg.message.split(" ")[0];
      if (command) stats[command] = (stats[command] || 0) + 1;
    }
  });
  return Object.entries(stats)
    .map(([command, count]) => ({ command, count }))
    .sort((a, b) => b.count - a.count);
});

const commandExecutionCount = computed(() =>
  commandStats.value.reduce((sum, cmd) => sum + cmd.count, 0)
);

const commandLog = computed(() => {
  return messages.value
    .filter((msg) => msg.message.startsWith("!"))
    .slice(-10)
    .reverse()
    .map((msg) => ({
      id: msg.id,
      command: (msg.message.split(" ")[0] ?? "").substring(1),
      user: msg.user_display_name,
      time: formatTime(msg.timestamp),
    }));
});

// Top chatters (derived from messages)
const topChatters = computed(() => {
  const chatterCounts: Record<
    string,
    { userId: string; name: string; messageCount: number }
  > = {};

  filteredMessages.value.forEach((msg) => {
    const chatter = chatterCounts[msg.user_id];
    if (chatter) {
      chatter.messageCount += 1;
    } else {
      chatterCounts[msg.user_id] = {
        userId: msg.user_id,
        name: msg.user_display_name,
        messageCount: 1,
      };
    }
  });

  return Object.values(chatterCounts)
    .sort((a, b) => b.messageCount - a.messageCount)
    .slice(0, 5);
});

const viewerCount = ref(0); // TODO: Get from backend

// Import connected platforms to get account display names
const { accounts: connectedAccounts, getAccountDisplayName } = useConnectedPlatforms();

// Connected streams from active session platforms
const connectedStreams = computed(() => {
  if (!activeSession.value || !activeSession.value.platforms) return [];

  return activeSession.value.platforms.map((platform) => {
    const p = typeof platform === "string" ? { platform, accountId: platform } : platform;

    // Find the actual account data to get display name
    const account = connectedAccounts.value.find((a) => String(a.id) === String(p.accountId));
    const displayName = account
      ? getAccountDisplayName(account)
      : `Account ${p.accountId}`;

    return {
      id: String(p.accountId),
      platform: p.platform.charAt(0).toUpperCase() + p.platform.slice(1), // Capitalize platform name
      accountName: displayName,
      isLive: p.streamId !== undefined,
      viewers: p.viewerCount || 0,
    };
  });
});

// Total viewers across all streams
const totalViewers = computed(() => {
  if (selectedStreamId.value !== "all") {
    const stream = connectedStreams.value.find((s) => s.id === selectedStreamId.value);
    return stream?.viewers || 0;
  }
  return connectedStreams.value.reduce((sum, s) => sum + s.viewers, 0);
});

// Uptime calculator
const uptime = computed(() => {
  if (!activeSession.value?.currentRunStartedAt) {
    const elapsed = Date.now() - startTime.value;
    const hours = Math.floor(elapsed / 3600000);
    const minutes = Math.floor((elapsed % 3600000) / 60000);
    return `${hours}h ${minutes}m`;
  }
  return formatDuration(activeSession.value.currentRunStartedAt);
});

// Max command count for bar charts
const maxCommandCount = computed(() => {
  return Math.max(...commandStats.value.map((c) => c.count), 1);
});

const platforms = ref([
  { id: "twitch", name: "Twitch", enabled: true, icon: LogosTwitch },
  { id: "youtube", name: "YouTube", enabled: true, icon: LogosYoutube },
  { id: "kick", name: "Kick", enabled: true, icon: SimpleIconsKick },
]);

const recentSessions = computed(() => sessionManager.recentSessions.value.slice(0, 3));

// Modal state
const showConfigModal = ref(false);
const selectedSession = computed(() => activeSession.value);

// Functions
function formatDuration(startedAt: string) {
  const now = Date.now();
  const started = new Date(startedAt).getTime();
  const diff = now - started;

  const hours = Math.floor(diff / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

  return `${hours}h ${minutes}m`;
}

function formatTime(timestamp: string) {
  const date = new Date(timestamp);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function formatSessionDuration(durationMs: number) {
  const hours = Math.floor(durationMs / (1000 * 60 * 60));
  const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

function formatTimeAgo(timestamp: string) {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const diff = now - then;

  const minutes = Math.floor(diff / (1000 * 60));
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days > 0) return `${days}d ago`;
  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return "just now";
}

function getUserColor(userId: string): string {
  let hash = 0;
  for (let i = 0; i < userId.length; i++) {
    hash = userId.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = hash % 360;
  return `hsl(${hue}, 70%, 50%)`;
}

function badgeEmoji(badge: string): string {
  const badges: Record<string, string> = {
    moderator: "🗡️",
    subscriber: "⭐",
    vip: "💎",
    broadcaster: "📡",
  };
  return badges[badge] || "👤";
}

function getPlatformIcon(platformId: string) {
  const iconMap: Record<string, any> = {
    twitch: LogosTwitch,
    youtube: LogosYoutube,
    kick: SimpleIconsKick,
  };
  return iconMap[platformId.toLowerCase()];
}

// Functions
function addMessage(msg: ChatMessage) {
  messages.value.push(msg);

  // Auto-scroll
  nextTick(() => scrollToBottom());

  // Keep only last 1000 messages
  if (messages.value.length > 1000) {
    messages.value.shift();
  }
}

async function sendMessage() {
  if (!messageInput.value.trim()) return;

  if (!activeSession.value) {
    alert("No active session");
    return;
  }

  // Get enabled platforms from session
  const enabledPlatformIds = sessionPlatforms.value
    .filter((p) => platformToggles.value[p.id] !== false)
    .map((p) => p.id);

  if (enabledPlatformIds.length === 0) {
    alert("No platforms enabled");
    return;
  }

  // Extract account_ids from active session platforms
  const accountIds: (string | number)[] = [];
  if (activeSession.value.platforms) {
    activeSession.value.platforms.forEach((platform) => {
      const platformObj =
        typeof platform === "string" ? { platform, accountId: platform } : platform;
      if (enabledPlatformIds.includes(platformObj.platform)) {
        accountIds.push(platformObj.accountId || platformObj.platform);
      }
    });
  }

  if (accountIds.length === 0) {
    alert("No account IDs found for enabled platforms");
    return;
  }

  try {
    await streamingConnection.sendChatMessage(accountIds, messageInput.value);
    messageInput.value = "";
  } catch (error) {
    console.error("Failed to send message:", error);
    alert("Failed to send message: " + error);
  }
}

function scrollToBottom() {
  if (chatContainer.value) {
    chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
  }
}

function configureSession() {
  if (!activeSession.value) return;
  showConfigModal.value = true;
}

async function handleSaveSession(sessionData: any) {
  try {
    console.log("Saving session configuration:", sessionData);
    await sessionManager.updateSession(sessionData.id, sessionData);
    showConfigModal.value = false;
  } catch (error) {
    console.error("Failed to save session:", error);
    alert("Failed to save session. Please try again.");
  }
}

async function endSession() {
  if (!activeSession.value) return;
  if (confirm("End this session?")) {
    try {
      await sessionManager.endSession(activeSession.value.id);
    } catch (error) {
      console.error("Failed to end session:", error);
    }
  }
}

function clearCommandLog() {
  // commandLog is computed, can't clear it
  console.log("Command log cleared");
}

function openOverlays() {
  console.log("Opening overlays");
}

function openVirtualOBS() {
  console.log("Opening virtual OBS");
}

function exportChatLog() {
  const chatText = messages.value
    .map(
      (msg) =>
        `[${formatTime(msg.timestamp)}] ${msg.user_display_name} (${msg.platform}): ${
          msg.message
        }`
    )
    .join("\n");

  const blob = new Blob([chatText], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `chat-log-${new Date().toISOString()}.txt`;
  a.click();
  URL.revokeObjectURL(url);
}

function togglePlatform(platformId: string) {
  const platform = platforms.value.find((p) => p.id === platformId);
  if (platform) platform.enabled = !platform.enabled;
}

function toggleSessionPlatform(platformId: string) {
  platformToggles.value[platformId] = platformToggles.value[platformId] === false;
}

function viewAllSessions() {
  router.push("/streaming");
}

function activateSession(session: any) {
  console.log("Activating session:", session);
  router.push(`/streaming/chat?session=${session.id}`);
}

function clearChat() {
  if (confirm("Clear all chat messages?")) {
    messages.value = [];
  }
}

function replyTo(msg: ChatMessage) {
  messageInput.value = `@${msg.user_display_name} `;
}

function deleteMessage(msg: ChatMessage) {
  if (confirm(`Delete message from ${msg.user_display_name}?`)) {
    const index = messages.value.findIndex((m) => m.id === msg.id);
    if (index > -1) {
      messages.value.splice(index, 1);
    }
    // TODO: Call backend to delete on platform
  }
}

function toggleModMode() {
  modMode.value = !modMode.value;
  console.log("Mod mode:", modMode.value);
}

function connectPlatform(platformId: string) {
  console.log("Connecting to:", platformId);
  alert(`Connect to ${platformId} - OAuth flow coming soon!`);
}

function switchStream() {
  console.log("Switched to stream:", selectedStreamId.value);
}

function openLogs() {
  console.log("Opening logs...");
}

function resetCommandStats() {
  if (confirm("Reset command statistics?")) {
    // Command stats are computed, just clear messages to reset
    console.log("Command stats reset");
  }
}

function getSessionPlatformsDisplay(session: any): string {
  if (!session?.platforms || session.platforms.length === 0) {
    return "No platforms";
  }

  const firstPlatform = session.platforms[0];

  // Handle new format (PlatformStream objects)
  if (
    typeof firstPlatform === "object" &&
    firstPlatform !== null &&
    "platform" in firstPlatform
  ) {
    return session.platforms
      .map((p: any) => {
        const platformStr =
          typeof p.platform === "string" ? p.platform : String(p.platform || "unknown");
        return platformStr.charAt(0).toUpperCase() + platformStr.slice(1);
      })
      .join(" + ");
  }

  // Handle old format (string array)
  return session.platforms
    .map((p: any) => {
      if (typeof p === "string") {
        return p.charAt(0).toUpperCase() + p.slice(1);
      }
      // Fallback for unexpected types
      const platformStr = String(p || "unknown");
      return platformStr.charAt(0).toUpperCase() + platformStr.slice(1);
    })
    .join(" + ");
}

// Lifecycle
// Set up event listeners BEFORE async operations (to avoid Vue lifecycle warning)
let unsubscribeMessages: (() => void) | null = null;
let unsubscribeSessionUpdates: (() => void) | null = null;

// Listen for chat messages
unsubscribeMessages = streamingConnection.onMessage((message) => {
  addMessage(message);
});

// Listen for session updates from Rust (when sessions are auto-activated, etc.)
unsubscribeSessionUpdates = streamingConnection.onSessionUpdate(async () => {
  console.log("🔄 [CHAT PAGE] Session updated in Rust, reloading sessions...");
  await sessionManager.loadSessions();
  console.log("✅ [CHAT PAGE] Sessions reloaded");
});

// Cleanup on unmount
onUnmounted(() => {
  console.log("🎥 Chat v2 unmounted");
  if (unsubscribeMessages) unsubscribeMessages();
  if (unsubscribeSessionUpdates) unsubscribeSessionUpdates();
  // Do NOT disconnect - WebSocket stays connected globally for persistent streaming
});

onMounted(async () => {
  console.log("🎥 Chat v2 mounted");

  // Load desktop sessions
  await sessionManager.loadSessions();

  // Read session ID from URL query param
  const sessionParam = route.query.session as string;
  if (sessionParam) {
    selectedSessionId.value = sessionParam;
    console.log("📍 Selected session from URL:", sessionParam);
  }

  // Check if already connected (auto-connect on startup handles this)
  // No need to connect again - just load historical messages
  console.log(
    "📡 Streaming connection status:",
    streamingConnection.isConnected.value ? "Connected" : "Not connected"
  );

  // Load historical messages from Rust memory
  if (selectedSession.value?.platforms) {
    const accountIds = selectedSession.value.platforms.map((p) => Number(p.accountId));
    console.log("🔍 Loading historical messages for accounts:", accountIds);

    try {
      const historicalMessages = await invoke<ChatMessage[]>(
        "streaming_get_messages_for_accounts",
        {
          accountIds,
          limit: 100, // Load last 100 messages
        }
      );

      console.log(`📚 Loaded ${historicalMessages.length} historical messages from Rust`);
      messages.value = historicalMessages;

      // Auto-scroll to bottom to show newest messages
      nextTick(() => scrollToBottom());
    } catch (err) {
      console.error("❌ Failed to load historical messages:", err);
    }
  }
});
</script>
