<template lang="pug">
LegacyButton(
  :data="button"
  :iconUrl="iconUrl"
  :isAlt="isAlt"
  :isShift="isShift"
  :theme="theme"
  :isReactive="interactive"
  :image-fit="imageFit"
  fill
  @mousedown="emitIntent('start', $event)"
  @mouseup="emitIntent('end', $event)"
  @mouseleave="emitIntent('cancel', $event)"
  @touchstart="emitIntent('start', $event)"
  @touchend="emitIntent('end', $event)"
  @touchcancel="emitIntent('cancel', $event)"
  @click="emitIntent('commit', $event)"
)
</template>

<script setup lang="ts">
import LegacyButton from "@base/components/Deck/Dynamic/Button.vue";
import type { LegacyDeckButton } from "@base/utils/legacyDeckWidget";
import type { WidgetInputSource, WidgetIntent, WidgetIntentPhase } from "@base/types/deckWidget";

const props = withDefaults(defineProps<{
  button: LegacyDeckButton;
  instanceId: string;
  interactive?: boolean;
  iconUrl?: string;
  isAlt?: boolean;
  isShift?: boolean;
  theme?: Record<string, string>;
  imageFit?: "cover" | "contain";
}>(), {
  interactive: false,
  iconUrl: undefined,
  isAlt: false,
  isShift: false,
  theme: () => ({}),
  imageFit: "cover",
});

const emit = defineEmits<{
  (event: "intent", intent: WidgetIntent): void;
}>();

const sequence = ref(0);

const sourceFromEvent = (event?: Event): WidgetInputSource => {
  if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent) return "touch";
  if (typeof PointerEvent !== "undefined" && event instanceof PointerEvent) {
    if (event.pointerType === "pen") return "pen";
    if (event.pointerType === "touch") return "touch";
  }
  return "mouse";
};

const emitIntent = (phase: WidgetIntentPhase, event?: Event) => {
  if (!props.interactive) return;
  emit("intent", {
    instanceId: props.instanceId,
    portId: "primary",
    legacyButtonId: typeof props.button.id === "number" ? props.button.id : undefined,
    kind: "trigger",
    phase,
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource: sourceFromEvent(event),
  });
};
</script>
