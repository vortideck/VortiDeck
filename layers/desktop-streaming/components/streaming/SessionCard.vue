<template lang="pug">
div(
  class="group relative p-6 bg-themeBackground2 rounded-2xl border border-themeBorder2 hover:border-blue-400/50 transition-all duration-300 backdrop-blur-sm hover:shadow-xl hover:shadow-blue-500/10 overflow-hidden"
)
  div(class="absolute inset-0 bg-gradient-to-br from-blue-500/0 to-purple-500/0 group-hover:from-blue-500/5 group-hover:to-purple-500/5 transition-all duration-300")
  div(class="relative")
    // Header
    div(class="flex items-start justify-between mb-4")
      div(class="flex-1")
        div(class="flex items-center gap-2.5 mb-2")
          h3(class="font-semibold text-themeText text-lg") {{ session.name }}
          span(
            :class="statusBadgeClass"
            class="px-3 py-1 rounded-lg text-xs font-bold backdrop-blur-sm border"
          ) {{ statusLabel }}

        div(class="text-sm text-themeText2 flex items-center gap-2")
          span(v-if="session.platforms") {{ platformsText }}
          span(v-if="session.started_at") • {{ formatDuration }}

    // Stats
    div(v-if="status === 'active'" class="flex items-center gap-6 text-sm text-themeText2 mb-4 p-4 bg-themeBackground3 rounded-xl backdrop-blur-sm")
      div(class="flex items-center gap-2")
        IconUsers(class="w-4 h-4 text-blue-400")
        span {{ session.viewers || 0 }} viewers
      div(class="flex items-center gap-2")
        MaterialSymbolsChatOutline(class="w-4 h-4 text-green-400")
        span {{ session.message_count || 0 }} messages
      div(class="flex items-center gap-2")
        IconBolt(class="w-4 h-4 text-purple-400")
        span {{ session.command_count || 0 }} commands

    // Actions
    div(class="flex items-center gap-2.5")
      // Draft state: Activate or Delete
      button(
        v-if="status === 'draft'"
        @click="$emit('activate', session)"
        class="flex-1 px-4 py-2.5 text-sm bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 rounded-xl transition-all duration-200 flex items-center justify-center gap-2 font-medium shadow-lg shadow-blue-500/25"
      )
        MaterialSymbolsPlayArrowOutline(class="w-4 h-4")
        span Activate Session

      // Waiting for live: View Chat or Cancel
      button(
        v-if="status === 'waiting_for_live'"
        @click="$emit('view-chat', session)"
        class="flex-1 px-4 py-2.5 text-sm bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 rounded-xl transition-all duration-200 flex items-center justify-center gap-2 font-medium shadow-lg shadow-blue-500/25"
      )
        MaterialSymbolsChatOutline(class="w-4 h-4")
        span View Chat

      // Active state: Pause or View Chat
      button(
        v-if="status === 'active'"
        @click="$emit('pause', session)"
        class="px-4 py-2.5 text-sm bg-gradient-to-r from-yellow-500/20 to-orange-500/20 hover:from-yellow-500/30 hover:to-orange-500/30 text-yellow-300 rounded-xl transition-all duration-200 backdrop-blur-sm border border-yellow-400/30"
      )
        MaterialSymbolsPauseOutline(class="w-4 h-4")

      button(
        v-if="status === 'active'"
        @click="$emit('view-chat', session)"
        class="flex-1 px-4 py-2.5 text-sm bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 rounded-xl transition-all duration-200 font-medium"
      ) View Chat

      // Paused state: Resume
      button(
        v-if="status === 'paused'"
        @click="$emit('resume', session)"
        class="flex-1 px-4 py-2.5 text-sm bg-gradient-to-r from-green-500 to-emerald-500 hover:from-green-600 hover:to-emerald-600 rounded-xl transition-all duration-200 flex items-center justify-center gap-2 font-medium shadow-lg shadow-green-500/25"
      )
        MaterialSymbolsPlayArrowOutline(class="w-4 h-4")
        span Resume

      // Configure button (all states except ended)
      button(
        v-if="status !== 'ended_by_user' && status !== 'ended_auto'"
        @click="$emit('configure', session)"
        class="px-4 py-2.5 text-sm bg-themeBackground3 hover:bg-themeHover rounded-xl transition-all duration-200 text-themeText2 hover:text-themeText backdrop-blur-sm border border-themeBorder2"
      )
        MaterialSymbolsSettingsOutline(class="w-4 h-4")

      // End button (active, paused, waiting states)
      button(
        v-if="status === 'active' || status === 'paused' || status === 'waiting_for_live'"
        @click="$emit('end', session)"
        class="px-4 py-2.5 text-sm bg-gradient-to-r from-red-500/20 to-pink-500/20 hover:from-red-500/30 hover:to-pink-500/30 text-red-300 rounded-xl transition-all duration-200 backdrop-blur-sm border border-red-400/30"
      ) End

      // Delete button (draft only)
      button(
        v-if="status === 'draft'"
        @click="$emit('delete', session)"
        class="px-4 py-2.5 text-sm bg-gradient-to-r from-red-500/20 to-pink-500/20 hover:from-red-500/30 hover:to-pink-500/30 text-red-300 rounded-xl transition-all duration-200 backdrop-blur-sm border border-red-400/30"
      )
        MaterialSymbolsDeleteOutline(class="w-4 h-4")
</template>

<script setup lang="ts">
import { computed } from 'vue';
import MaterialSymbolsPlayArrowOutline from '~icons/material-symbols/play-arrow-outline';
import MaterialSymbolsPauseOutline from '~icons/material-symbols/pause-outline';
import MaterialSymbolsChatOutline from '~icons/material-symbols/chat-outline';
import MaterialSymbolsSettingsOutline from '~icons/material-symbols/settings-outline';
import MaterialSymbolsDeleteOutline from '~icons/material-symbols/delete-outline';
import IconUsers from '~icons/heroicons/user-group';
import IconBolt from '~icons/heroicons/bolt';

const props = defineProps<{
  session: any;
  status: 'draft' | 'waiting_for_live' | 'active' | 'paused' | 'ended_by_user' | 'ended_auto';
}>();

defineEmits(['activate', 'start', 'pause', 'resume', 'configure', 'end', 'delete', 'view-chat']);

const statusBadgeClass = computed(() => {
  const classes = {
    draft: 'bg-gradient-to-r from-gray-500/20 to-slate-500/20 text-gray-300 border-gray-400/30',
    waiting_for_live: 'bg-gradient-to-r from-orange-500/20 to-amber-500/20 text-orange-300 border-orange-400/30',
    active: 'bg-gradient-to-r from-red-500/30 to-pink-500/30 text-red-300 border-red-400/30 flex items-center gap-1.5',
    paused: 'bg-gradient-to-r from-yellow-500/20 to-orange-500/20 text-yellow-300 border-yellow-400/30',
    ended_by_user: 'bg-gradient-to-r from-green-500/20 to-emerald-500/20 text-green-300 border-green-400/30',
    ended_auto: 'bg-gradient-to-r from-gray-500/20 to-slate-500/20 text-gray-300 border-gray-400/30',
  };
  return classes[props.status] || classes.draft;
});

const statusLabel = computed(() => {
  const labels = {
    draft: 'DRAFT',
    waiting_for_live: 'WAITING',
    active: 'LIVE',
    paused: 'PAUSED',
    ended_by_user: 'ENDED',
    ended_auto: 'AUTO-ENDED',
  };
  return labels[props.status] || props.status.toUpperCase();
});

const platformsText = computed(() => {
  if (Array.isArray(props.session.platforms)) {
    return props.session.platforms.join(' + ');
  }
  return props.session.platforms || '';
});

const formatDuration = computed(() => {
  if (!props.session.started_at) return '';

  const start = new Date(props.session.started_at);
  const now = new Date();
  const diff = now.getTime() - start.getTime();

  const hours = Math.floor(diff / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
});
</script>
