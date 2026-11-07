<template lang="pug">
div(
    class="flex gap-2 select-none w-full h-full" 
    :class="orientation === 'vertical' ? 'flex-col' : 'flex-row'"
)
    p(class="text-sm shrink-0" v-if="label") {{ label }}
    div(
        class="relative bg-[rgb(0,0,0,0.4)]  rounded-full p-1 cursor-pointer flex grow" 
        :class="orientation === 'vertical' ? 'flex-col w-10' : 'flex-row h-12'"
        @click="handleSwitchClick" 
        @mousedown="handleDragStart"
        @mousemove="handleDrag"
        @mouseup="handleDragEnd"
        @mouseleave="handleDragEnd"
        @touchstart="handleDragStart"
        @touchmove="handleDrag" 
        @touchend="handleDragEnd"
        ref="switchTrack"
    )
        div(
            class="absolute bg-blue-500 rounded-full transition-all duration-100 ease-out" 
            :class="orientation === 'vertical' ? 'w-8 h-[calc(100%/3-4px)]' : 'h-10 w-[calc(100%/3-4px)]'"
            :style="switchPositionStyle" 
            ref="switchThumb"
        )
        div(
            class="relative z-10 flex items-center text-sm w-full h-full" 
            :class="orientation === 'vertical' ? 'flex-col' : 'flex-row'"
        )
            div(class="flex-1 flex items-center justify-center" :class="{'text-white': modelValue === 0, 'text-gray-600': modelValue !== 0}") {{ options[0] }}
            div(class="flex-1 flex items-center justify-center" :class="{'text-white': modelValue === 1, 'text-gray-600': modelValue !== 1}") {{ options[1] }}
            div(class="flex-1 flex items-center justify-center" :class="{'text-white': modelValue === 2, 'text-gray-600': modelValue !== 2}") {{ options[2] }}
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { vibrate } from "@tauri-apps/plugin-haptics";

interface Props {
  modelValue: number;
  orientation?: "horizontal" | "vertical";
  label?: string;
  options?: string[];
}

const switchPositionStyle = computed(() => {
  if (!switchTrack.value) return {};

  const isVertical = props.orientation === "vertical";

  const trackSize = isVertical
    ? switchTrack.value.offsetHeight - 8
    : switchTrack.value.offsetWidth - 8;

  const segmentSize = trackSize / 3;

  const position = props.modelValue * segmentSize;

  return isVertical
    ? { transform: `translateY(${position}px)` }
    : { transform: `translateX(${position}px)` };
});

const props = withDefaults(defineProps<Props>(), {
  orientation: "horizontal",
  label: "",
  options: () => ["Left", "Center", "Right"],
});

const emit = defineEmits<{
  (e: "update:modelValue", value: number): void;
}>();

const switchTrack = ref<HTMLElement | null>(null);
const switchThumb = ref<HTMLElement | null>(null);
let isDragging = ref(false);
let startX = ref(0);

const switchPosition = computed(() => {
  if (!switchTrack.value) return 0;
  const trackWidth = switchTrack.value.offsetWidth - 8; // Account for padding
  const segmentWidth = trackWidth / 3;

  return props.modelValue * segmentWidth;
});

const handleSwitchClick = async (e: MouseEvent) => {
  if (!switchTrack.value) return;
  const rect = switchTrack.value.getBoundingClientRect();
  const isVertical = props.orientation === "vertical";

  // Get position based on orientation
  const pos = isVertical ? e.clientY - rect.top : e.clientX - rect.left;

  // Get total size based on orientation
  const size = isVertical ? rect.height : rect.width;
  const third = size / 3;

  if (pos < third) {
    emit("update:modelValue", 0);
  } else if (pos < third * 2) {
    emit("update:modelValue", 1);
  } else {
    emit("update:modelValue", 2);
  }
  await vibrate(2);
};

const handleDragStart = (e: MouseEvent | TouchEvent) => {
  isDragging.value = true;
  const isVertical = props.orientation === "vertical";

  // Store initial position based on orientation
  startX.value =
    "touches" in e
      ? isVertical
        ? e.touches[0].clientY
        : e.touches[0].clientX
      : isVertical
      ? e.clientY
      : e.clientX;
};

const handleDrag = (e: MouseEvent | TouchEvent) => {
  if (!isDragging.value || !switchTrack.value) return;

  const rect = switchTrack.value.getBoundingClientRect();
  const isVertical = props.orientation === "vertical";

  // Get current position based on orientation
  const currentPos =
    "touches" in e
      ? isVertical
        ? e.touches[0].clientY
        : e.touches[0].clientX
      : isVertical
      ? e.clientY
      : e.clientX;

  // Calculate relative position
  const relativePos = isVertical ? currentPos - rect.top : currentPos - rect.left;

  // Get size based on orientation
  const size = isVertical ? rect.height : rect.width;
  const third = size / 3;

  if (relativePos < third) {
    emit("update:modelValue", 0);
  } else if (relativePos < third * 2) {
    emit("update:modelValue", 1);
  } else {
    emit("update:modelValue", 2);
  }
};

const handleDragEnd = () => {
  isDragging.value = false;
};
</script>

<style lang="scss" scoped></style>
