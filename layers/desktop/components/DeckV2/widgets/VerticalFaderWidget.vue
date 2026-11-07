<template lang="pug">
div(
  class="fader-widget flex flex-col items-center justify-between h-full w-full p-2.5 select-none relative overflow-hidden bg-transparent min-h-0 min-w-0"
  @wheel.prevent="handleWheel"
)
  //- Top Label & Value
  div(class="flex items-center justify-between w-full text-[10px] font-bold text-slate-300 px-0.5 flex-shrink-0")
    span(class="truncate max-w-[65%]") {{ item.title || item.config.label || 'Master Vol' }}
    span(class="font-mono text-blue-400 font-bold") {{ Math.round(volumeLevel) }}%

  //- Fader Rail Area (Clean inset)
  div(
    ref="trackRef"
    class="fader-track flex-1 w-full my-1.5 relative rounded-xl bg-[#0d1017] overflow-hidden flex items-end justify-center cursor-ns-resize min-h-0 shadow-inner"
    @mousedown="handleTrackMouseDown"
    @touchstart.prevent="handleTrackTouchStart"
  )
    //- Fill Bar
    div(
      class="fader-fill w-full transition-all duration-75"
      :style="{ height: `${volumeLevel}%`, background: fillGradient }"
    )

    //- Studio dB Calibration Marks
    div(class="fader-ticks absolute inset-0 flex flex-col justify-between py-2 px-1.5 pointer-events-none opacity-20 font-mono text-[7px] text-slate-400")
      div(class="flex justify-between items-center w-full border-t border-slate-500")
        span +6
      div(class="flex justify-between items-center w-full border-t border-slate-500")
        span 0
      div(class="flex justify-between items-center w-full border-t border-slate-500")
        span -6
      div(class="flex justify-between items-center w-full border-t border-slate-500")
        span -18
      div(class="flex justify-between items-center w-full border-t border-slate-500")
        span -inf

    //- Fader Knob Thumb
    div(
      class="fader-thumb absolute left-1 right-1 h-3.5 rounded-md bg-gradient-to-b from-slate-200 to-slate-400 shadow-md pointer-events-none flex items-center justify-center"
      :style="{ bottom: `calc(${volumeLevel}% - 7px)` }"
    )
      div(class="w-4 h-0.5 bg-slate-700 rounded-full")

  //- Bottom Mute Button
  button(
    type="button"
    class="flex items-center justify-center w-full py-1 rounded-lg text-[10px] font-bold transition-all flex-shrink-0"
    :class="isMuted ? 'bg-red-500/25 text-red-400' : 'bg-white/[0.06] text-slate-300 hover:bg-white/[0.12]'"
    @click.stop="toggleMute"
  )
    MdiVolumeOff(v-if="isMuted" class="w-3 h-3 mr-1")
    MdiVolumeHigh(v-else class="w-3 h-3 mr-1")
    span {{ isMuted ? 'MUTED' : 'MUTE' }}
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';
import MdiVolumeHigh from '~icons/mdi/volume-high';
import MdiVolumeOff from '~icons/mdi/volume-off';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const emit = defineEmits<{
  (e: 'change', value: number): void;
}>();

const volumeLevel = ref(props.item.config.defaultLevel ?? 75);
const previousLevel = ref(75);
const isMuted = ref(false);
const trackRef = ref<HTMLElement | null>(null);
const isDragging = ref(false);

const fillGradient = computed(() => {
  if (isMuted.value) return 'linear-gradient(to top, #ef4444, #f87171)';
  return props.item.config.accentGradient || 'linear-gradient(to top, #3b82f6, #8b5cf6)';
});

const updateLevelFromEvent = (clientY: number) => {
  if (!trackRef.value) return;
  const rect = trackRef.value.getBoundingClientRect();
  const rawPercent = ((rect.bottom - clientY) / rect.height) * 100;
  const clamped = Math.max(0, Math.min(100, rawPercent));
  volumeLevel.value = clamped;
  if (isMuted.value && clamped > 0) isMuted.value = false;
  emit('change', clamped);
};

const handleTrackMouseDown = (event: MouseEvent) => {
  if (!props.isInteractive) return;
  isDragging.value = true;
  updateLevelFromEvent(event.clientY);

  const onMouseMove = (e: MouseEvent) => {
    if (isDragging.value) updateLevelFromEvent(e.clientY);
  };
  const onMouseUp = () => {
    isDragging.value = false;
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
  };

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);
};

const handleTrackTouchStart = (event: TouchEvent) => {
  if (!props.isInteractive) return;
  isDragging.value = true;
  const touch = event.touches[0];
  if (touch) updateLevelFromEvent(touch.clientY);

  const onTouchMove = (e: TouchEvent) => {
    const t = e.touches[0];
    if (t && isDragging.value) updateLevelFromEvent(t.clientY);
  };
  const onTouchEnd = () => {
    isDragging.value = false;
    window.removeEventListener('touchmove', onTouchMove);
    window.removeEventListener('touchend', onTouchEnd);
  };

  window.addEventListener('touchmove', onTouchMove);
  window.addEventListener('touchend', onTouchEnd);
};

const handleWheel = (e: WheelEvent) => {
  if (!props.isInteractive) return;
  const delta = e.deltaY < 0 ? 5 : -5;
  volumeLevel.value = Math.max(0, Math.min(100, volumeLevel.value + delta));
  if (isMuted.value && volumeLevel.value > 0) isMuted.value = false;
  emit('change', volumeLevel.value);
};

const toggleMute = () => {
  if (!props.isInteractive) return;
  if (isMuted.value) {
    isMuted.value = false;
    volumeLevel.value = previousLevel.value || 50;
  } else {
    previousLevel.value = volumeLevel.value;
    isMuted.value = true;
    volumeLevel.value = 0;
  }
  emit('change', volumeLevel.value);
};
</script>
