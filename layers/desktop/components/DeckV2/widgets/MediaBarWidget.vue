<template lang="pug">
div(
  class="media-widget flex items-center justify-between h-full w-full p-2.5 select-none relative overflow-hidden bg-transparent gap-2.5 min-h-0 min-w-0"
)
  //- Left: Album Cover Art Icon
  div(class="relative w-10 h-10 rounded-xl overflow-hidden flex-shrink-0 bg-[#0d1017] shadow-inner flex items-center justify-center")
    MdiSpotify(class="w-6 h-6 text-emerald-400")

  //- Center: Track info & scrubber
  div(class="flex-1 min-w-0 flex flex-col justify-center")
    div(class="flex items-center justify-between gap-1")
      span(class="text-[11px] font-bold text-slate-200 truncate") {{ trackTitle }}
      span(class="text-[9px] text-slate-400 font-mono flex-shrink-0") {{ formatTime(progressSeconds) }}
    span(class="text-[10px] text-slate-400 truncate mb-1") {{ artistName }}

    //- Interactive Scrub Bar
    div(
      class="w-full h-1.5 bg-[#0d1017] rounded-full overflow-hidden cursor-pointer relative"
      @click="handleScrub"
    )
      div(
        class="h-full bg-emerald-400 rounded-full transition-all duration-300"
        :style="{ width: `${progressPercent}%` }"
      )

  //- Right: Playback Controls
  div(class="flex items-center gap-1 flex-shrink-0")
    button(
      type="button"
      class="w-7 h-7 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] active:scale-95 text-slate-300 hover:text-white flex items-center justify-center transition-all"
      @click.stop="prevTrack"
      title="Previous"
    )
      MdiSkipPrevious(class="w-4 h-4")

    button(
      type="button"
      class="w-8 h-8 rounded-lg bg-emerald-500 hover:bg-emerald-400 active:scale-95 text-slate-950 flex items-center justify-center shadow-md transition-all font-bold"
      @click.stop="togglePlay"
      :title="isPlaying ? 'Pause' : 'Play'"
    )
      MdiPause(v-if="isPlaying" class="w-4 h-4")
      MdiPlay(v-else class="w-4 h-4")

    button(
      type="button"
      class="w-7 h-7 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] active:scale-95 text-slate-300 hover:text-white flex items-center justify-center transition-all"
      @click.stop="nextTrack"
      title="Next"
    )
      MdiSkipNext(class="w-4 h-4")
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';
import MdiSpotify from '~icons/mdi/spotify';
import MdiPlay from '~icons/mdi/play';
import MdiPause from '~icons/mdi/pause';
import MdiSkipNext from '~icons/mdi/skip-next';
import MdiSkipPrevious from '~icons/mdi/skip-previous';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const isPlaying = ref(true);
const progressSeconds = ref(94);
const trackIndex = ref(0);

const playlist = [
  { title: 'Synthwave Odyssey', artist: 'Neon Horizon' },
  { title: 'Midnight Lo-Fi Stream', artist: 'Study Beats' },
  { title: 'Cyberpunk Drive', artist: 'Retrowave' },
];

const trackTitle = computed(() => props.item.config.title || playlist[trackIndex.value].title);
const artistName = computed(() => props.item.config.artist || playlist[trackIndex.value].artist);
const progressPercent = computed(() => (progressSeconds.value / 225) * 100);

let timer: any = null;

onMounted(() => {
  timer = setInterval(() => {
    if (isPlaying.value) {
      progressSeconds.value = (progressSeconds.value + 1) % 225;
    }
  }, 1000);
});

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});

const formatTime = (sec: number) => {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s < 10 ? '0' : ''}${s}`;
};

const togglePlay = () => {
  if (!props.isInteractive) return;
  isPlaying.value = !isPlaying.value;
};

const nextTrack = () => {
  if (!props.isInteractive) return;
  trackIndex.value = (trackIndex.value + 1) % playlist.length;
  progressSeconds.value = 0;
};

const prevTrack = () => {
  if (!props.isInteractive) return;
  trackIndex.value = (trackIndex.value - 1 + playlist.length) % playlist.length;
  progressSeconds.value = 0;
};

const handleScrub = (event: MouseEvent) => {
  if (!props.isInteractive) return;
  const target = event.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  const clickX = event.clientX - rect.left;
  const ratio = Math.max(0, Math.min(1, clickX / rect.width));
  progressSeconds.value = Math.floor(ratio * 225);
};
</script>
