<template lang="pug">
label(class="flex h-full w-full flex-col items-center justify-center gap-2 rounded-xl bg-black/70 p-3 text-white")
  span(class="truncate text-xs font-semibold") {{ label }}
  input(
    class="min-h-0 w-full flex-1 accent-blue-500"
    type="range"
    orient="vertical"
    :min="minimum"
    :max="maximum"
    :step="step"
    :value="value"
    :disabled="!interactive"
    aria-orientation="vertical"
    @input="emitValue('preview', $event)"
    @change="emitValue('commit', $event)"
  )
  output(class="text-[11px] tabular-nums") {{ value }}
</template>

<script setup lang="ts">
import type { DeckWidgetInstance, WidgetIntent, WidgetRuntimeState } from "@base/types/deckWidget";

const props = defineProps<{
  instance: DeckWidgetInstance;
  runtimeState: WidgetRuntimeState;
  interactive: boolean;
}>();
const emit = defineEmits<{ (event: "intent", intent: WidgetIntent): void }>();
const sequence = ref(0);
const label = computed(() => String(props.instance.config.label ?? "Fader"));
const minimum = computed(() => Number(props.instance.config.min ?? 0));
const maximum = computed(() => Number(props.instance.config.max ?? 100));
const step = computed(() => Number(props.instance.config.step ?? 1));
const value = computed(() => Number(props.runtimeState.values.value ?? props.instance.config.value ?? minimum.value));

const emitValue = (phase: "preview" | "commit", event: Event) => {
  if (!props.interactive || !(event.target instanceof HTMLInputElement)) return;
  emit("intent", {
    instanceId: props.instance.id,
    portId: "value",
    kind: "value",
    phase,
    value: event.target.valueAsNumber,
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource: "mouse",
  });
};
</script>

<style scoped>
input[type="range"] { writing-mode: vertical-lr; direction: rtl; }
</style>
