<template lang="pug">
div(
  class="notes-widget flex flex-col justify-between h-full w-full p-2.5 rounded-xl transition-all duration-200 select-none relative overflow-hidden bg-slate-900 border border-amber-500/20 shadow-md"
)
  //- Header
  div(class="flex items-center justify-between pb-1 border-b border-white/10")
    div(class="flex items-center gap-1.5")
      span(class="text-xs") 📝
      span(class="text-xs font-bold text-amber-400") Stream Agenda
    span(class="text-[10px] text-white/40 font-mono")
      | {{ completedCount }}/{{ items.length }} done

  //- Checklist / Notes Body
  div(class="flex-1 overflow-y-auto py-1.5 space-y-1 scrollbar-thin")
    div(
      v-for="(item, idx) in items"
      :key="idx"
      class="flex items-center gap-2 p-1 rounded bg-slate-800/60 hover:bg-slate-800 cursor-pointer transition-colors"
      @click="toggleItem(idx)"
    )
      input(
        type="checkbox"
        :checked="item.done"
        class="w-3.5 h-3.5 rounded border-white/20 text-amber-500 focus:ring-amber-400"
      )
      span(
        class="text-[11px] leading-tight transition-all"
        :class="item.done ? 'line-through text-white/40' : 'text-white/90 font-medium'"
      ) {{ item.text }}
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const items = ref([
  { text: '🎙️ Welcome & intro topic', done: true },
  { text: '📣 Sponsor shoutout (VortiDeck)', done: false },
  { text: '🎮 Competitive ranked matches', done: false },
  { text: '🎁 Viewer giveaway draw', done: false },
]);

const completedCount = computed(() => items.value.filter((i) => i.done).length);

const toggleItem = (idx: number) => {
  if (!props.isInteractive) return;
  items.value[idx].done = !items.value[idx].done;
};
</script>
