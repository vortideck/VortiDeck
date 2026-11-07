<template lang="pug">
LegacyRotary(
  :data="button"
  :iconUrl="iconUrl"
  :isAlt="isAlt"
  :isShift="isShift"
  :theme="theme"
  :isReactive="interactive"
  :displayValue="displayValue"
  :behavior="behavior"
  @valueChange="emitValue('preview', $event)"
  @valueCommit="emitValue('commit', $event)"
)
</template>

<script setup lang="ts">
import LegacyRotary from "@base/components/Deck/Buttons/Rotary.vue";
import type { LegacyDeckButton } from "@base/utils/legacyDeckWidget";
import type { WidgetIntent } from "@base/types/deckWidget";

const props = withDefaults(defineProps<{
  button: LegacyDeckButton;
  instanceId: string;
  interactive?: boolean;
  iconUrl?: string;
  isAlt?: boolean;
  isShift?: boolean;
  theme: Record<string, string>;
  displayValue?: boolean;
  behavior?: "free" | "blocked" | "continuous";
}>(), {
  interactive: false,
  iconUrl: undefined,
  isAlt: false,
  isShift: false,
  displayValue: false,
  behavior: "blocked",
});

const emit = defineEmits<{
  (event: "intent", intent: WidgetIntent): void;
}>();

const sequence = ref(0);

const emitValue = (phase: "preview" | "commit", value: number) => {
  if (!props.interactive) return;
  emit("intent", {
    instanceId: props.instanceId,
    portId: "value",
    legacyButtonId: typeof props.button.id === "number" ? props.button.id : undefined,
    kind: "value",
    phase,
    value,
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource: "mouse",
  });
};
</script>
