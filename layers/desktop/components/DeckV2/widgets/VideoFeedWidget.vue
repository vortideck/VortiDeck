<template lang="pug">
div(
  class="video-widget flex flex-col justify-between h-full w-full p-2.5 select-none relative overflow-hidden bg-transparent min-h-0 min-w-0"
)
  //- Top Bar: Telemetry & Rec Status
  div(class="flex items-center justify-between z-10 flex-shrink-0")
    div(class="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-red-600 text-[9px] font-bold text-white tracking-wider uppercase")
      span(class="w-1.5 h-1.5 rounded-full bg-white animate-ping")
      span REC
    div(class="flex items-center gap-2 text-[9px] text-slate-400 font-mono")
      span 1080p60
      span 6.2 Mbps

  //- Viewport Screen (Inset camera monitor)
  div(class="video-canvas relative flex-1 my-1.5 rounded-xl overflow-hidden flex items-center justify-center bg-[#0d1017] min-h-0")
    //- Corner Framing Guides
    div(class="absolute top-1.5 left-1.5 text-[8px] font-mono text-slate-600 select-none") ⌜
    div(class="absolute top-1.5 right-1.5 text-[8px] font-mono text-slate-600 select-none") ⌝
    div(class="absolute bottom-1.5 left-1.5 text-[8px] font-mono text-slate-600 select-none") ⌞
    div(class="absolute bottom-1.5 right-1.5 text-[8px] font-mono text-slate-600 select-none") ⌟

    //- Center Program View
    div(class="flex flex-col items-center justify-center text-center p-2")
      MdiVideo(class="w-6 h-6 text-slate-400 mb-1")
      span(class="text-[10px] font-bold text-slate-200 uppercase tracking-wider") {{ activeScene }}
      span(class="text-[9px] text-slate-500 font-mono") Program Output

    //- Stereo Peak Level Meters
    div(class="absolute right-2 bottom-2 top-2 flex gap-0.5 items-end opacity-70")
      div(class="w-1 bg-gradient-to-t from-emerald-500 via-amber-400 to-red-500 rounded-sm h-[70%]")
      div(class="w-1 bg-gradient-to-t from-emerald-500 via-amber-400 to-red-500 rounded-sm h-[62%]")

  //- Bottom Scene Switcher Bar
  div(class="flex items-center justify-between z-10 flex-shrink-0 pt-0.5")
    span(class="text-[9px] font-bold text-slate-400 uppercase tracking-wider") Scene
    div(class="flex items-center gap-1.5")
      button(
        v-for="scene in ['Game', 'BRB', 'Chat']"
        :key="scene"
        type="button"
        class="px-2.5 py-0.5 rounded-lg text-[9px] font-bold transition-all"
        :class="activeScene === scene ? 'bg-blue-600 text-white shadow-sm' : 'bg-white/[0.06] text-slate-300 hover:bg-white/[0.12]'"
        @click="switchScene(scene)"
      ) {{ scene }}
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';
import MdiVideo from '~icons/mdi/video';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const activeScene = ref('Game');

const switchScene = (scene: string) => {
  if (!props.isInteractive) return;
  activeScene.value = scene;
};
</script>
