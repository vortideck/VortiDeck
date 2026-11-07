<template lang="pug">
div(
  v-if="item"
  class="widget-inspector bg-slate-900/95 border-t border-white/10 p-4 shadow-2xl backdrop-blur-md transition-all duration-300"
)
  div(class="max-w-5xl mx-auto flex flex-col md:flex-row items-start md:items-center justify-between gap-4")
    //- Left info
    div(class="flex items-center gap-3")
      div(class="w-10 h-10 rounded-xl bg-blue-500/20 border border-blue-500/30 flex items-center justify-center text-xl")
        | {{ widgetIcon }}
      div
        div(class="flex items-center gap-2")
          h4(class="text-sm font-bold text-white") {{ item.title || item.config.label || item.type }}
          span(class="px-2 py-0.5 rounded bg-blue-500/20 text-blue-400 font-mono text-[10px] font-bold")
            | {{ item.colSpan }}×{{ item.rowSpan }}
        p(class="text-xs text-white/50") Type: {{ item.type.replace('_', ' ') }}

    //- Center: Dimension span controls
    div(class="flex items-center gap-3 bg-slate-800/80 px-3 py-1.5 rounded-xl border border-white/5")
      div(class="flex items-center gap-1.5")
        label(class="text-xs text-white/60 font-semibold") Width (Cols):
        select(
          v-model="item.colSpan"
          class="bg-slate-900 border border-white/10 rounded-lg px-2 py-1 text-xs text-white focus:outline-none focus:border-blue-500"
        )
          option(:value="1") 1
          option(:value="2") 2
          option(:value="3") 3
          option(:value="4") 4

      div(class="flex items-center gap-1.5")
        label(class="text-xs text-white/60 font-semibold") Height (Rows):
        select(
          v-model="item.rowSpan"
          class="bg-slate-900 border border-white/10 rounded-lg px-2 py-1 text-xs text-white focus:outline-none focus:border-blue-500"
        )
          option(:value="1") 1
          option(:value="2") 2
          option(:value="3") 3

    //- Quick Rename input
    div(class="flex items-center gap-2 flex-1 max-w-xs")
      label(class="text-xs text-white/60 font-semibold") Title:
      input(
        type="text"
        v-model="item.title"
        class="w-full bg-slate-800 border border-white/10 rounded-lg px-3 py-1.5 text-xs text-white focus:outline-none focus:border-blue-500 placeholder-white/30"
        placeholder="Custom Widget Name"
      )

    //- Right: Actions (Delete / Close)
    div(class="flex items-center gap-2 ml-auto")
      button(
        type="button"
        class="px-3 py-1.5 rounded-lg bg-red-500/20 hover:bg-red-500/30 text-red-300 text-xs font-semibold transition-colors flex items-center gap-1"
        @click="emit('delete', item.id)"
      )
        span 🗑️
        span Delete

      button(
        type="button"
        class="px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white text-xs font-semibold transition-colors"
        @click="emit('close')"
      ) Done
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from './types';

const props = defineProps<{
  item: DeckWidgetItem | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'delete', id: string): void;
}>();

const widgetIcon = computed(() => {
  if (!props.item) return '🧩';
  switch (props.item.type) {
    case 'action_button':
      return '⚡';
    case 'vertical_fader':
      return '🎚️';
    case 'media_bar':
      return '🎵';
    case 'rotary_dial':
      return '🎛️';
    case 'system_gauge':
      return '📊';
    case 'rocker_toggle':
      return '🔘';
    case 'live_chat':
      return '💬';
    case 'video_feed':
      return '📹';
    case 'stream_notes':
      return '📝';
    default:
      return '🧩';
  }
});
</script>
