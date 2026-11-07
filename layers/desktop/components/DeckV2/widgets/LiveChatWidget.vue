<template lang="pug">
div(
  class="chat-widget flex flex-col h-full w-full p-2.5 select-none relative overflow-hidden bg-transparent min-h-0 min-w-0 justify-between"
)
  //- Header (Clean, no divider border)
  div(class="flex items-center justify-between pb-1 flex-shrink-0")
    div(class="flex items-center gap-1.5 min-w-0")
      MdiTwitch(class="w-3.5 h-3.5 text-purple-400 flex-shrink-0")
      span(class="text-[11px] font-bold text-slate-200 truncate tracking-wide") Twitch Chat
    div(class="flex items-center gap-1 text-[10px] text-slate-400 font-mono flex-shrink-0")
      span(class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse")
      span 1.4k

  //- Message Stream - Clean, smooth text
  div(
    ref="messagesContainer"
    class="flex-1 overflow-y-auto py-1 space-y-1 text-[11px] leading-tight min-h-0 scrollbar-none"
  )
    div(
      v-for="msg in messages"
      :key="msg.id"
      class="chat-row flex items-baseline gap-1 py-0.5 px-1 rounded hover:bg-white/[0.04] transition-colors"
    )
      span(
        class="text-[8px] font-bold px-1 py-0.2 rounded uppercase flex-shrink-0 leading-none"
        :class="msg.badgeClass"
      ) {{ msg.badge }}
      span(class="font-semibold flex-shrink-0 text-[10px]" :style="{ color: msg.color }") {{ msg.user }}:
      span(class="text-slate-300 break-words text-[10px] flex-1") {{ msg.text }}

  //- Quick Moderator / Streamer Reaction Bar (Borderless subtle buttons)
  div(class="flex items-center gap-1.5 pt-1 flex-shrink-0")
    button(
      type="button"
      class="flex-1 py-1 px-2 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] active:scale-95 text-[9px] font-semibold text-slate-200 transition-all"
      @click="sendQuickReaction('HYPE! 🔥')"
    ) Hype!
    button(
      type="button"
      class="flex-1 py-1 px-2 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] active:scale-95 text-[9px] font-semibold text-slate-200 transition-all"
      @click="sendQuickReaction('GG WP')"
    ) GG WP
    button(
      type="button"
      class="py-1 px-2 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] text-[9px] text-slate-400 hover:text-slate-200 transition-all"
      title="Clear Chat"
      @click="clearChat"
    )
      MdiTrashCanOutline(class="w-3 h-3")
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';
import MdiTwitch from '~icons/mdi/twitch';
import MdiTrashCanOutline from '~icons/mdi/trash-can-outline';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const messages = ref([
  { id: 1, badge: 'SUB', badgeClass: 'bg-purple-500/25 text-purple-300', user: 'VortiFan', color: '#c084fc', text: 'hype stream today!' },
  { id: 2, badge: 'VIP', badgeClass: 'bg-amber-500/25 text-amber-300', user: 'PixelKnight', color: '#38bdf8', text: 'Audio levels look crisp' },
  { id: 3, badge: 'MOD', badgeClass: 'bg-emerald-500/25 text-emerald-300', user: 'Sarah_Mod', color: '#34d399', text: 'Welcome in everyone!' },
  { id: 4, badge: 'SUB', badgeClass: 'bg-purple-500/25 text-purple-300', user: 'ApexPro', color: '#f472b6', text: 'Clip that round' },
]);

const samplePool = [
  { badge: 'SUB', badgeClass: 'bg-purple-500/25 text-purple-300', user: 'SynthWave', color: '#fb923c', text: 'Great scene transition' },
  { badge: 'VIEW', badgeClass: 'bg-white/10 text-slate-300', user: 'Gamer_99', color: '#60a5fa', text: 'POG 🚀' },
  { badge: 'VIP', badgeClass: 'bg-amber-500/25 text-amber-300', user: 'NeonFox', color: '#a78bfa', text: 'That deck setup is clean' },
  { badge: 'SUB', badgeClass: 'bg-purple-500/25 text-purple-300', user: 'CyberByte', color: '#2dd4bf', text: 'Audio fader is reactive!' },
];

const messagesContainer = ref<HTMLElement | null>(null);
let chatTimer: any = null;

onMounted(() => {
  chatTimer = setInterval(() => {
    const next = samplePool[Math.floor(Math.random() * samplePool.length)];
    messages.value.push({
      id: Date.now(),
      badge: next.badge,
      badgeClass: next.badgeClass,
      user: next.user,
      color: next.color,
      text: next.text,
    });
    if (messages.value.length > 20) {
      messages.value.shift();
    }
    nextTick(() => {
      if (messagesContainer.value) {
        messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
      }
    });
  }, 4000);
});

onBeforeUnmount(() => {
  if (chatTimer) clearInterval(chatTimer);
});

const sendQuickReaction = (text: string) => {
  if (!props.isInteractive) return;
  messages.value.push({
    id: Date.now(),
    badge: 'HOST',
    badgeClass: 'bg-blue-500/30 text-blue-300',
    user: 'Host',
    color: '#60a5fa',
    text,
  });
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    }
  });
};

const clearChat = () => {
  if (!props.isInteractive) return;
  messages.value = [];
};
</script>

<style scoped>
.scrollbar-none::-webkit-scrollbar {
  display: none;
}
.scrollbar-none {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>
