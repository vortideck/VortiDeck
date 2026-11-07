<template lang="pug">
div(
  class="action-widget flex flex-col items-center justify-center h-full w-full select-none relative overflow-hidden bg-transparent p-2 min-h-0 min-w-0 transition-all duration-100"
  :class="[isPressed ? 'scale-95 brightness-125' : 'hover:scale-[1.02]']"
  @mousedown="handlePress"
  @mouseup="handleRelease"
  @mouseleave="handleRelease"
  @touchstart.prevent="handlePress"
  @touchend.prevent="handleRelease"
)
  //- Status Badge
  div(
    v-if="item.config.badge"
    class="absolute top-2 right-2 px-1.5 py-0.5 rounded bg-red-600 text-[8px] font-bold text-white uppercase tracking-wider leading-none shadow-sm"
  ) {{ item.config.badge }}

  //- Center Action Icon
  div(class="text-slate-300 mb-1.5 flex items-center justify-center flex-shrink-0")
    MdiRecordCircle(v-if="item.config.iconName === 'record'" class="w-7 h-7 text-red-500")
    MdiMicrophoneOff(v-else-if="item.config.iconName === 'mic-off'" class="w-7 h-7 text-amber-400")
    MdiVideoOff(v-else-if="item.config.iconName === 'video-off'" class="w-7 h-7 text-purple-400")
    MdiBroadcast(v-else-if="item.config.iconName === 'stream'" class="w-7 h-7 text-cyan-400")
    MdiLightningBolt(v-else class="w-7 h-7 text-blue-400")

  //- Action Label
  span(class="text-[11px] font-bold text-slate-200 truncate max-w-[90%] text-center leading-tight")
    | {{ item.title || item.config.label || 'Action' }}
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';
import MdiRecordCircle from '~icons/mdi/record-circle';
import MdiMicrophoneOff from '~icons/mdi/microphone-off';
import MdiVideoOff from '~icons/mdi/video-off';
import MdiBroadcast from '~icons/mdi/broadcast';
import MdiLightningBolt from '~icons/mdi/lightning-bolt';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const emit = defineEmits<{
  (e: 'action', id: string): void;
}>();

const isPressed = ref(false);
const activeState = ref(props.item.config.initiallyActive || false);

const handlePress = () => {
  if (!props.isInteractive) return;
  isPressed.value = true;
};

const handleRelease = () => {
  if (!props.isInteractive || !isPressed.value) return;
  isPressed.value = false;
  activeState.value = !activeState.value;
  emit('action', props.item.id);
};
</script>
