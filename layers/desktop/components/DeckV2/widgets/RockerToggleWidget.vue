<template lang="pug">
div(
  class="rocker-widget flex flex-col items-center justify-between h-full w-full p-2.5 select-none relative overflow-hidden bg-transparent cursor-pointer min-h-0 min-w-0"
  @click="handleToggle"
)
  //- Label
  div(class="flex items-center gap-1.5 text-[10px] font-bold text-slate-300 truncate max-w-[90%] flex-shrink-0")
    MdiMicrophone(v-if="isOn" class="w-3.5 h-3.5 text-emerald-400")
    MdiMicrophoneOff(v-else class="w-3.5 h-3.5 text-red-400")
    span(class="truncate") {{ item.title || item.config.label || 'Mic' }}

  //- Physical Switch Component (Clean inset rail)
  div(
    class="rocker-switch relative my-auto w-12 h-6 rounded-full transition-all duration-200 p-0.5 flex items-center shadow-inner flex-shrink-0"
    :class="isOn ? 'bg-emerald-500/25' : 'bg-[#0d1017]'"
  )
    //- Thumb
    div(
      class="rocker-thumb w-5 h-5 rounded-full shadow-md transition-all duration-200 flex items-center justify-center text-[10px]"
      :class="isOn ? 'translate-x-6 bg-emerald-400 text-slate-950 shadow-emerald-500/40' : 'translate-x-0 bg-slate-400 text-slate-900'"
    )
      span(class="w-1.5 h-1.5 rounded-full bg-current")

  //- Status Text
  div(class="flex items-center gap-1 text-[9px] font-bold tracking-wider flex-shrink-0" :class="isOn ? 'text-emerald-400' : 'text-slate-500'")
    span(
      class="w-1.5 h-1.5 rounded-full"
      :class="isOn ? 'bg-emerald-400 animate-pulse' : 'bg-slate-600'"
    )
    span {{ isOn ? (item.config.onLabel || 'LIVE') : (item.config.offLabel || 'MUTED') }}
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';
import MdiMicrophone from '~icons/mdi/microphone';
import MdiMicrophoneOff from '~icons/mdi/microphone-off';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggle', state: boolean): void;
}>();

const isOn = ref(props.item.config.initialState ?? true);

const handleToggle = () => {
  if (!props.isInteractive) return;
  isOn.value = !isOn.value;
  emit('toggle', isOn.value);
};
</script>
