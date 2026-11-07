<template lang="pug">
div(
  v-if="isLocked"
  class="absolute inset-0 z-40 flex items-center justify-center"
  :class="[`lock-overlay-${position}`, 'bg-black bg-opacity-70', isInteractive ? 'pointer-events-auto cursor-pointer' : 'pointer-events-none']"
  :style="overlayStyle"
  @touchstart="handleTouchStart"
  @touchend="handleTouchEnd"
  @touchcancel="handleTouchEnd"
  @touchmove="handleTouchMove"
  @click="handleClick"
)
  div(class="flex flex-col items-center" :class="overlayClasses" style="mix-blend: normal;")
    svg(
      xmlns="http://www.w3.org/2000/svg" 
      :class="iconClasses" 
      class="text-white opacity-90" 
      fill="none" 
      viewBox="0 0 24 24" 
      stroke="currentColor" 
      stroke-width="2.5"
    )
      path(
        stroke-linecap="round" 
        stroke-linejoin="round" 
        d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
      )
    p(v-if="shouldShowText" :class="textClasses" class="text-white font-medium opacity-90 mt-1") Upgrade
</template>

<script setup lang="ts">
import { computed } from "vue";

interface Props {
  position: number;
  isLocked: boolean;
  gridSize?: string; // e.g. "3,4" for responsive sizing
  isMobile?: boolean; // Different behavior for mobile vs desktop
  borderRadius?: string; // Match button theme rounding
  isInteractive?: boolean; // Locks execution, but must not intercept editor gestures
}

interface Emits {
  (event: "locked-touch-start", payload: { position: number; event: TouchEvent }): void;
  (event: "locked-touch-end", payload: { position: number; event: TouchEvent }): void;
  (event: "locked-touch-move", payload: { position: number; event: TouchEvent }): void;
  (event: "locked-click", position: number): void;
}

const props = withDefaults(defineProps<Props>(), {
  gridSize: "3,4",
  isMobile: false,
  borderRadius: "0.75rem",
  isInteractive: true,
});

const emit = defineEmits<Emits>();

const gridDimensions = computed(() => {
  const [cols = 3, rows = 4] = props.gridSize.split(",").map(Number);
  return {
    cols: Number.isFinite(cols) ? cols : 3,
    rows: Number.isFinite(rows) ? rows : 4,
  };
});

// Responsive sizing based on grid size (mainly for desktop)
const overlayClasses = computed(() => {
  if (props.isMobile) {
    return "scale-90"; // Slightly smaller on mobile
  }

  if (!props.gridSize) return "";

  const { cols, rows } = gridDimensions.value;
  const totalButtons = cols * rows;

  if (totalButtons <= 9) return ""; // Default size for small grids
  if (totalButtons <= 16) return "scale-90";
  if (totalButtons <= 25) return "scale-75";
  return "scale-[0.6]"; // Very small for large grids
});

const iconClasses = computed(() => {
  if (props.isMobile) {
    return "w-6 h-6";
  }

  if (!props.gridSize) return "w-8 h-8";

  const { cols, rows } = gridDimensions.value;
  const totalButtons = cols * rows;

  if (totalButtons <= 9) return "w-8 h-8";
  if (totalButtons <= 16) return "w-6 h-6";
  if (totalButtons <= 25) return "w-5 h-5";
  return "w-4 h-4";
});

const textClasses = computed(() => {
  if (props.isMobile) {
    return "text-xs";
  }

  if (!props.gridSize) return "text-sm";

  const { cols, rows } = gridDimensions.value;
  const totalButtons = cols * rows;

  if (totalButtons <= 9) return "text-sm";
  if (totalButtons <= 16) return "text-xs";
  if (totalButtons <= 25) return "text-[10px]";
  return "text-[8px]";
});

// Hide text if grid is dense to avoid overflow
const shouldShowText = computed(() => {
  if (!props.gridSize) return true;
  const { cols, rows } = gridDimensions.value;

  // Hide based on grid dimensions, not total count
  // This works better across different screen sizes
  let showText;
  if (props.isMobile) {
    // Mobile: Hide if more than 4 columns OR more than 5 rows
    showText = cols <= 4 && rows <= 5;
  } else {
    // Desktop: Hide if more than 5 columns OR more than 6 rows
    showText = cols <= 5 && rows <= 6;
  }

  return showText;
});

// Match button shape by applying provided radius and clip content
const overlayStyle = computed(() => ({
  borderRadius: props.borderRadius,
  overflow: "hidden",
}));

// Touch event handlers - emit to parent without stopping propagation
// This allows parent to handle swipe gestures while we handle the lock behavior
const handleTouchStart = (event: TouchEvent) => {
  // Don't stop propagation - let parent handle swipe gestures
  emit("locked-touch-start", { position: props.position, event });
};

const handleTouchEnd = (event: TouchEvent) => {
  // Don't stop propagation
  emit("locked-touch-end", { position: props.position, event });
};

const handleTouchMove = (event: TouchEvent) => {
  // Don't stop propagation - needed for swipe detection
  emit("locked-touch-move", { position: props.position, event });
};

const handleClick = () => {
  emit("locked-click", props.position);
};
</script>

<style scoped>
/* Flash animation for locked button clicks */
@keyframes button-blocked-flash {
  0% {
    background-color: rgba(0, 0, 0, 0.5);
    transform: scale(1);
  }
  50% {
    background-color: rgba(239, 68, 68, 0.8);
    transform: scale(1.05);
  }
  100% {
    background-color: rgba(0, 0, 0, 0.5);
    transform: scale(1);
  }
}

.flash-animation {
  animation: button-blocked-flash 0.4s ease-in-out;
}

</style>
