<template lang="pug">
div(
  class="rotary-widget flex flex-col items-center justify-between h-full w-full p-2.5 select-none relative overflow-hidden bg-transparent min-h-0 min-w-0"
  @wheel.prevent="handleWheel"
)
  //- Top Label & %
  div(class="flex items-center justify-between w-full text-[10px] font-bold text-slate-300 px-0.5 flex-shrink-0")
    span(class="truncate max-w-[65%]") {{ item.title || item.config.label || 'Gain' }}
    span(class="font-mono text-cyan-400 font-bold") {{ Math.round(value) }}%

  //- Rotary Knob Outer Dial with LED Ring
  div(
    ref="knobRef"
    class="knob-container relative my-auto w-12 h-12 rounded-full flex items-center justify-center cursor-grab active:cursor-grabbing flex-shrink-0"
    @mousedown="handleMouseDown"
    @touchstart.prevent="handleTouchStart"
  )
    //- LED Arc Ring SVG
    svg(class="absolute inset-0 w-full h-full transform -rotate-90" viewBox="0 0 48 48")
      circle(
        cx="24"
        cy="24"
        r="20"
        stroke="rgba(255, 255, 255, 0.08)"
        stroke-width="3"
        fill="none"
      )
      circle(
        cx="24"
        cy="24"
        r="20"
        :stroke="ringColor"
        stroke-width="3.5"
        stroke-linecap="round"
        fill="none"
        :stroke-dasharray="125.6"
        :stroke-dashoffset="125.6 - (125.6 * (value / 100))"
      )

    //- Physical Knob Center with clean tactile ring
    div(
      class="knob-face w-9 h-9 rounded-full bg-gradient-to-br from-slate-700 via-slate-800 to-slate-950 shadow-md flex items-center justify-center relative transition-transform duration-75"
      :style="{ transform: `rotate(${rotationDegrees}deg)` }"
    )
      //- Indicator Notch
      div(class="absolute top-1 w-1 h-2 rounded-full shadow-sm" :style="{ backgroundColor: ringColor }")

  //- Bottom Status / Action
  div(class="text-[9px] font-mono text-slate-400 text-center flex-shrink-0")
    span Dial
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const emit = defineEmits<{
  (e: 'change', value: number): void;
}>();

const value = ref(props.item.config.defaultValue ?? 60);
const rotationDegrees = computed(() => (value.value / 100) * 270 - 135);
const ringColor = computed(() => props.item.config.ringColor || '#22d3ee');

const knobRef = ref<HTMLElement | null>(null);
const startY = ref(0);
const startVal = ref(0);

const handleMouseDown = (e: MouseEvent) => {
  if (!props.isInteractive) return;
  startY.value = e.clientY;
  startVal.value = value.value;

  const onMouseMove = (moveEvent: MouseEvent) => {
    const deltaY = startY.value - moveEvent.clientY;
    const nextVal = Math.max(0, Math.min(100, startVal.value + deltaY * 0.8));
    value.value = nextVal;
    emit('change', nextVal);
  };

  const onMouseUp = () => {
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
  };

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);
};

const handleTouchStart = (e: TouchEvent) => {
  if (!props.isInteractive) return;
  const touch = e.touches[0];
  if (!touch) return;
  startY.value = touch.clientY;
  startVal.value = value.value;

  const onTouchMove = (moveEvent: TouchEvent) => {
    const t = moveEvent.touches[0];
    if (!t) return;
    const deltaY = startY.value - t.clientY;
    const nextVal = Math.max(0, Math.min(100, startVal.value + deltaY * 0.8));
    value.value = nextVal;
    emit('change', nextVal);
  };

  const onTouchEnd = () => {
    window.removeEventListener('touchmove', onTouchMove);
    window.removeEventListener('touchend', onTouchEnd);
  };

  window.addEventListener('touchmove', onTouchMove);
  window.addEventListener('touchend', onTouchEnd);
};

const handleWheel = (e: WheelEvent) => {
  if (!props.isInteractive) return;
  const delta = e.deltaY < 0 ? 5 : -5;
  value.value = Math.max(0, Math.min(100, value.value + delta));
  emit('change', value.value);
};
</script>
