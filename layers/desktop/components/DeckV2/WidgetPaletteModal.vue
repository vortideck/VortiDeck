<template lang="pug">
div(
  v-if="isOpen"
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4 animate-fadeIn"
  @click.self="emit('close')"
)
  div(
    class="bg-slate-900 border border-white/15 rounded-2xl w-full max-w-2xl overflow-hidden shadow-2xl flex flex-col max-h-[85vh]"
  )
    //- Modal Header
    div(class="px-6 py-4 border-b border-white/10 flex items-center justify-between bg-slate-800/60")
      div
        h2(class="text-lg font-bold text-white flex items-center gap-2")
          span 🧩
          | Add Plugin Widget to Deck
        p(class="text-xs text-white/50") Choose from built-in components and community stream deck widgets
      button(
        type="button"
        class="w-8 h-8 rounded-lg bg-white/5 hover:bg-white/15 text-white/70 hover:text-white flex items-center justify-center transition-colors"
        @click="emit('close')"
      ) ✕

    //- Category filter tabs
    div(class="px-6 py-2.5 border-b border-white/10 flex items-center gap-2 overflow-x-auto bg-slate-950/40")
      button(
        v-for="cat in categories"
        :key="cat"
        type="button"
        class="px-3 py-1.5 rounded-lg text-xs font-semibold whitespace-nowrap transition-all"
        :class="selectedCategory === cat ? 'bg-blue-600 text-white shadow-sm' : 'text-white/60 hover:text-white hover:bg-white/5'"
        @click="selectedCategory = cat"
      ) {{ cat }}

    //- Widgets Grid
    div(class="p-6 overflow-y-auto grid grid-cols-1 md:grid-cols-2 gap-3.5")
      div(
        v-for="manifest in filteredWidgets"
        :key="manifest.type"
        class="widget-card p-3.5 rounded-xl bg-slate-800/70 hover:bg-slate-800 border border-white/10 hover:border-blue-500/50 transition-all cursor-pointer flex flex-col justify-between group"
        @click="selectWidget(manifest)"
      )
        div(class="flex items-start gap-3")
          div(class="w-10 h-10 rounded-lg bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-xl flex-shrink-0 group-hover:scale-105 transition-transform")
            | {{ manifest.icon }}
          div(class="flex-1 min-w-0")
            div(class="flex items-center justify-between")
              h3(class="text-sm font-bold text-white truncate") {{ manifest.name }}
              span(class="px-1.5 py-0.5 rounded bg-white/10 font-mono text-[10px] text-white/70")
                | {{ manifest.defaultSpan.cols }}×{{ manifest.defaultSpan.rows }}
            p(class="text-xs text-white/50 mt-1 leading-relaxed line-clamp-2")
              | {{ manifest.description }}

        div(class="mt-3 pt-2.5 border-t border-white/5 flex items-center justify-between text-[11px]")
          span(class="text-blue-400 font-medium") {{ manifest.category }}
          span(class="text-white/40 group-hover:text-blue-400 font-bold transition-colors") + Add to Grid →
</template>

<script setup lang="ts">
import type { WidgetManifest } from './types';

const props = defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'add', manifest: WidgetManifest): void;
}>();

const categories = ['All', 'Actions', 'Audio & Media', 'Live Streaming', 'Monitoring', 'Utilities'];
const selectedCategory = ref('All');

const availableWidgets: WidgetManifest[] = [
  {
    type: 'action_button',
    name: 'Hotkey / Macro Action',
    category: 'Actions',
    description: 'Trigger keyboard shortcuts, soundboard effects, or scene transitions.',
    icon: '⚡',
    defaultSpan: { cols: 1, rows: 1 },
    minSpan: { cols: 1, rows: 1 },
    maxSpan: { cols: 2, rows: 2 },
    defaultConfig: { label: 'New Action', iconEmoji: '⚡', bgColor: '#1e293b' },
  },
  {
    type: 'vertical_fader',
    name: 'Vertical Audio Fader',
    category: 'Audio & Media',
    description: 'Touch/drag vertical slider for master volume, Discord, or mic gain.',
    icon: '🎚️',
    defaultSpan: { cols: 1, rows: 2 },
    minSpan: { cols: 1, rows: 2 },
    maxSpan: { cols: 2, rows: 3 },
    defaultConfig: { label: 'Master Vol', defaultLevel: 75 },
  },
  {
    type: 'media_bar',
    name: 'Spotify / Media Player',
    category: 'Audio & Media',
    description: 'Track scrub bar, album cover, and playback controls (Play, Skip, Back).',
    icon: '🎵',
    defaultSpan: { cols: 2, rows: 1 },
    minSpan: { cols: 2, rows: 1 },
    maxSpan: { cols: 3, rows: 1 },
    defaultConfig: { title: 'Cyberpunk Synthwave', artist: 'Neon Horizon' },
  },
  {
    type: 'rotary_dial',
    name: 'Rotary Dial Knob',
    category: 'Audio & Media',
    description: 'Rotatable knob with LED progress arc for audio levels, zoom, or lighting.',
    icon: '🎛️',
    defaultSpan: { cols: 1, rows: 1 },
    minSpan: { cols: 1, rows: 1 },
    maxSpan: { cols: 2, rows: 2 },
    defaultConfig: { label: 'Headphones', defaultValue: 60, ringColor: '#22d3ee' },
  },
  {
    type: 'live_chat',
    name: 'Live Stream Chat Feed',
    category: 'Live Streaming',
    description: 'Real-time scrolling chat stream with badges, emotes, and quick reactions.',
    icon: '💬',
    defaultSpan: { cols: 2, rows: 2 },
    minSpan: { cols: 2, rows: 2 },
    maxSpan: { cols: 4, rows: 3 },
    defaultConfig: { platform: 'Twitch', channel: 'LiveStream' },
  },
  {
    type: 'video_feed',
    name: 'OBS Scene / Webcam Viewport',
    category: 'Live Streaming',
    description: 'Live video feed monitor showing active OBS program output and scene switcher.',
    icon: '📹',
    defaultSpan: { cols: 2, rows: 2 },
    minSpan: { cols: 2, rows: 2 },
    maxSpan: { cols: 4, rows: 3 },
    defaultConfig: { source: 'OBS Studio' },
  },
  {
    type: 'system_gauge',
    name: 'System Hardware Monitor',
    category: 'Monitoring',
    description: 'Live animated CPU, GPU temperature, and RAM graphs.',
    icon: '📊',
    defaultSpan: { cols: 2, rows: 1 },
    minSpan: { cols: 2, rows: 1 },
    maxSpan: { cols: 3, rows: 2 },
    defaultConfig: { metrics: ['cpu', 'gpu', 'ram'] },
  },
  {
    type: 'rocker_toggle',
    name: 'Rocker Switch / Toggle',
    category: 'Actions',
    description: 'Tactile physical toggle switch with On/Off states for mic mute, VPN, or lights.',
    icon: '🔘',
    defaultSpan: { cols: 1, rows: 1 },
    minSpan: { cols: 1, rows: 1 },
    maxSpan: { cols: 2, rows: 1 },
    defaultConfig: { label: 'Mic Mute', onLabel: 'ACTIVE', offLabel: 'MUTED', initialState: true },
  },
  {
    type: 'stream_notes',
    name: 'Teleprompter & Stream Notes',
    category: 'Utilities',
    description: 'Interactive checklist and talking points for smooth stream flow.',
    icon: '📝',
    defaultSpan: { cols: 2, rows: 1 },
    minSpan: { cols: 2, rows: 1 },
    maxSpan: { cols: 3, rows: 2 },
    defaultConfig: { title: 'Stream Agenda' },
  },
];

const filteredWidgets = computed(() => {
  if (selectedCategory.value === 'All') return availableWidgets;
  return availableWidgets.filter((w) => w.category === selectedCategory.value);
});

const selectWidget = (manifest: WidgetManifest) => {
  emit('add', manifest);
  emit('close');
};
</script>
