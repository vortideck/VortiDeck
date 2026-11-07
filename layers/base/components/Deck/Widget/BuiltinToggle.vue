<template lang="pug">
button(
  type="button"
  class="flex h-full w-full flex-col items-center justify-center gap-2 rounded-xl border border-white/10 bg-black/70 p-3 text-white"
  :class="enabled ? 'ring-2 ring-emerald-400' : ''"
  :disabled="!interactive"
  :aria-pressed="enabled"
  @click="toggle"
)
  span(class="text-xs font-semibold") {{ label }}
  span(class="relative h-6 w-11 rounded-full transition" :class="enabled ? 'bg-emerald-500' : 'bg-slate-600'")
    span(class="absolute top-1 h-4 w-4 rounded-full bg-white transition-all" :class="enabled ? 'left-6' : 'left-1'")
</template>

<script setup lang="ts">
import type { DeckWidgetInstance, WidgetIntent, WidgetRuntimeState } from "@base/types/deckWidget";
const props = defineProps<{ instance: DeckWidgetInstance; runtimeState: WidgetRuntimeState; interactive: boolean }>();
const emit = defineEmits<{ (event: "intent", intent: WidgetIntent): void }>();
const sequence = ref(0);
const label = computed(() => String(props.instance.config.label ?? "Toggle"));
const enabled = computed(() => Boolean(
  props.runtimeState.values.enabled ?? props.instance.config.enabled,
));
const toggle = () => {
  if (!props.interactive) return;
  const next = !enabled.value;
  emit("intent", {
    instanceId: props.instance.id,
    portId: "toggle",
    kind: "trigger",
    phase: "commit",
    value: next,
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource: "mouse",
  });
};
</script>
