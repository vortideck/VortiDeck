<template lang="pug">
  .flex.flex-col.items-center.user-select-none.rotary-container.rounded-full.aspect-square

    
    .relative.rotary-knob(
      
      ref="knob"
      :style="`padding: ${deckStore.getActiveDeck?.settings?.buttons_padding}px; transform: rotate(${pointerRotation}deg); ${theme.knobCss}; ${isReady ? theme.readyCss : ''}`"
      @mousedown.left="startDrag"
      @touchstart.prevent="startDrag"
      @mouseenter="isHovered = true"
      @mouseleave="isHovered = false"
      @contextmenu="openOptions($event)"
      :class="['rounded-full flex text-center items-center justify-center', isDragging ? 'cursor-grabbing' : 'cursor-grab', {'overflow-hidden': !isAlt && !isShift}]"
    )
      nuxt-img( draggable="false" v-if="iconUrl && !isAlt && !isShift" :src="iconUrl"  class="z-1 object-cover w-full h-full" :style="`${theme?.iconCss}; transform: rotate(-${pointerRotation}deg);`"  placeholder="/transparent.png")

      .absolute.w-full.h-full.flex.items-center.justify-center(v-if="(isAlt || !iconUrl) && !isShift" :style="`transform: rotate(-${pointerRotation}deg);`")
        span(class="text-white text-xs") {{data.label}}
      .absolute.w-full.h-full.flex.items-center.justify-center(v-if="isShift" :style="`transform: rotate(-${pointerRotation}deg);`")
        span(class="text-white text-xs") {{data.position}}
      .knob-pointer(v-show="!isAlt && !isShift" :class="['absolute top-[10%] left-1/2 -translate-x-1/2']" :style="theme.pointerCss")
    p.value-display(v-if="displayValue" class="mt-6 text-cyan-400 font-orbitron text-lg tracking-wide text-shadow-glow") {{ computedDisplayValue }} 
</template>

<script setup lang="ts">
import { vibrate } from "@tauri-apps/plugin-haptics";
import { useDeckStore } from "../../../stores/Deck";
const props = defineProps({
  iconUrl: {
    type: String,
    required: false,
  },
  behavior: {
    type: String as () => "free" | "blocked" | "continuous",
    required: false,
    default: "free",
  },
  displayValue: {
    type: Boolean,
    required: false,
    default: false,
  },
  theme: {
    type: Object,
    required: true,
  },
  data: {
    type: Object,
    required: false,
  },
  isAlt: {
    type: Boolean,
    required: false,
  },
  isShift: {
    type: Boolean,
    required: false,
  },
  isReactive: {
    type: Boolean,
    required: false,
    default: true,
  },
  holdDuration: {
    type: Number,
    required: false,
    default: 70, // Duration in milliseconds to hold before value changes
  },
});

// const testRotation = useRotaryValue(props.data.id);
const deckStore = useDeckStore();
const pointerRotation = ref(10); // Initialize at 10 degrees (value 0)
const isHovered = ref(false);
const isDragging = ref(false);
const isReady = ref(false); // Tracks if the knob is ready for value change
const buttonPosition = ref({ top: 0, left: 0, width: 0, height: 0 });
const knob = ref<HTMLElement | null>(null);
const lastAngle = ref<number | null>(null);
const lastValue = ref<number>(0); // Initialize at 0
const holdTimer = ref<number | null>(null);
const holdStartAngle = ref<number | null>(null);

// 🚀 PERFORMANCE: Throttle value change emissions to prevent message flooding
const lastEmitTime = ref<number>(0);
const THROTTLE_MS = 50; // Emit max once per 50ms (20 updates/sec) - smooth for relay connections

const emit = defineEmits(["openButtonOptions", "valueChange"]);

// Define utility functions first
const computeValue = (angle: number) => {
  if (angle < 10 || angle > 350) return null;
  return Math.round(((angle - 10) / 340) * 100);
};

onMounted(() => {
  try {
    if (JSON.parse(props.data?.background) && !deckStore.getSystemVolume?.value) {
      pointerRotation.value = 10 + JSON.parse(props.data?.background).value * 3.4;
      lastValue.value = JSON.parse(props.data?.background).value;
    }
  } catch (error) {
    console.log(error);
  }
});

watch(
  () => deckStore.getSystemVolume,
  (newValue) => {
    if (isDragging.value) return;
    if (props.data?.action?.includes("SystemControl")) {
      if (newValue.buttonId === props.data.id) {
        return; // Skip update if this button triggered the change
      }
      
      // Only update if the new value is significantly different from current UI state
      const currentUIValue = computeValue(pointerRotation.value);
      if (currentUIValue !== null && Math.abs(currentUIValue - newValue.value) <= 1) {
        return; // Skip update if values are very close (within 1 unit)
      }
      
      console.log("🔄 System volume changed:", newValue.value, "current UI:", currentUIValue);
      
      // Use requestAnimationFrame for smoother updates
      requestAnimationFrame(() => {
        // Convert volume (0-100) to rotation angle (10-350)
        const rotation = 10 + newValue.value * 3.4;
        pointerRotation.value = rotation;
        lastValue.value = newValue.value;
      });
    }
  },
  { immediate: true, flush: 'sync' } // Sync flush for immediate processing
);

const openOptions = (event: MouseEvent) => {
  if (!props.data) return;
  event.preventDefault();

  if (knob.value) {
    // Get the button's bounding rectangle
    const rect = knob.value.getBoundingClientRect();

    // Store the button's position and size
    buttonPosition.value = {
      top: rect.top,
      left: rect.left,
      width: rect.width,
      height: rect.height,
    };
  }

  // Emit the event with the boundaries
  emit("openButtonOptions", buttonPosition.value, props.data.id);
};

// Use the hover styles composable

// Drag logic with hold feature
const startDrag = (event: MouseEvent | TouchEvent) => {
  if (!props.isReactive) return;
  isDragging.value = true;
  if (!knob.value) return;

  const angle = getAngle(event);
  holdStartAngle.value = angle;

  // Start the hold timer
  holdTimer.value = window.setTimeout(() => {
    isReady.value = true;
    vibrate(5); // Vibrate when ready

    // Update pointer position once ready
    pointerRotation.value = normalizeAngle(angle + 90);
    lastAngle.value = angle;
    lastValue.value = computeValue(pointerRotation.value) ?? 0;
  }, props.holdDuration);

  document.addEventListener("mousemove", onDrag);
  document.addEventListener("mouseup", stopDrag);
  document.addEventListener("touchmove", onDrag, { passive: false });
  document.addEventListener("touchend", stopDrag);
};

const onDrag = (event: MouseEvent | TouchEvent) => {
  if (!isDragging.value || !knob.value) return;
  event.preventDefault();

  const currentAngle = getAngle(event);

  // If the user moves too much before the hold duration completes, cancel the timer
  if (!isReady.value && holdStartAngle.value !== null) {
    const angleDiff = Math.abs(shortestAngle(holdStartAngle.value, currentAngle));
    if (angleDiff > 5) {
      // If moved more than 5 degrees
      if (holdTimer.value) {
        clearTimeout(holdTimer.value);
        holdTimer.value = null;
      }
      return;
    }
  }

  // Only update rotation if ready
  if (isReady.value) {
    updateRotation(event);
  }
};

const stopDrag = () => {
  isDragging.value = false;
  isReady.value = false;
  lastAngle.value = null;
  holdStartAngle.value = null;

  if (holdTimer.value) {
    clearTimeout(holdTimer.value);
    holdTimer.value = null;
  }

  document.removeEventListener("mousemove", onDrag);
  document.removeEventListener("mouseup", stopDrag);
  document.removeEventListener("touchmove", onDrag);
  document.removeEventListener("touchend", stopDrag);
};

const getAngle = (event: MouseEvent | TouchEvent) => {
  if (!knob.value) return 0;
  const rect = knob.value.getBoundingClientRect();
  const centerX = rect.left + rect.width / 2;
  const centerY = rect.top + rect.height / 2;
  const touch = "touches" in event
    ? event.touches[0] ?? event.changedTouches[0]
    : undefined;
  const clientX = touch?.clientX ?? ("clientX" in event ? event.clientX : centerX);
  const clientY = touch?.clientY ?? ("clientY" in event ? event.clientY : centerY);
  return Math.atan2(clientY - centerY, clientX - centerX) * (180 / Math.PI);
};

const normalizeAngle = (angle: number) => {
  return (angle + 360) % 360;
};

const updateRotation = async (event: MouseEvent | TouchEvent) => {
  if (!isReady.value || lastAngle.value === null) return;

  const currentAngle = getAngle(event);
  const delta = shortestAngle(lastAngle.value, currentAngle);
  const newRotation = normalizeAngle(pointerRotation.value + delta);
  let currentValue = computeValue(newRotation);
  const prevValue = lastValue.value;

  if (props.behavior === "free") {
    // Free mode - handle null points but don't allow full 360 rotation
    if (currentValue === null) {
      await vibrate(2);
      return; // Don't update rotation when hitting null points
    }
  } else if (props.behavior === "blocked") {
    // Blocked mode - don't allow past endpoints or null points
    if (
      currentValue === null ||
      (prevValue !== null &&
        ((prevValue === 0 && newRotation > 50) ||
          (prevValue === 100 && newRotation < 310)))
    ) {
      return;
    }
  } else if (props.behavior === "continuous") {
    // Continuous mode - allow full 360-degree rotation
    pointerRotation.value = newRotation;
    lastAngle.value = currentAngle;
    if (currentValue === null) currentValue = prevValue;
    if (lastValue.value !== currentValue && currentValue !== null) {
      lastValue.value = currentValue;

      // 🚀 THROTTLED: Only emit if enough time has passed since last emit
      const now = Date.now();
      if (now - lastEmitTime.value >= THROTTLE_MS) {
        emit("valueChange", currentValue);
        lastEmitTime.value = now;
      }
    }
    return;
  }

  // Update for free and blocked modes
  pointerRotation.value = newRotation;
  lastAngle.value = currentAngle;

  // Only emit value change when the value actually changes AND throttle time has passed
  if (lastValue.value !== currentValue && currentValue !== null) {
    lastValue.value = currentValue;

    // 🚀 THROTTLED: Only emit if enough time has passed since last emit
    const now = Date.now();
    if (now - lastEmitTime.value >= THROTTLE_MS) {
      emit("valueChange", currentValue);
      lastEmitTime.value = now;
    }
  }
};

const computedDisplayValue = computed(() => {
  return props.displayValue ? lastValue.value : null;
});

const shortestAngle = (from: number, to: number) => {
  let delta = to - from;
  if (delta > 180) delta -= 360;
  else if (delta < -180) delta += 360;
  return delta;
};

// Clean up any timers when component is unmounted
onBeforeUnmount(() => {
  if (holdTimer.value) {
    clearTimeout(holdTimer.value);
  }
});
</script>

<style scoped lang="scss">
@import url("https://fonts.googleapis.com/css2?family=Orbitron:wght@400;700&display=swap");

.rotary-knob {
  width: 100%;
  height: 100%;
}

.knob-pointer {
  width: 10%;
  height: 25%;
}

.text-shadow-glow {
  text-shadow: 0 0 8px rgba(0, 255, 255, 0.9), 0 0 3px rgba(0, 255, 255, 0.6);
}
</style>
