<template lang="pug">
div(class="grid h-full w-full grid-cols-3 overflow-hidden rounded-xl border border-white/10 bg-black/70 text-white")
  button(v-for="control in controls" :key="control.portId" type="button" class="grid place-items-center hover:bg-white/10 disabled:opacity-50" :disabled="!interactive" :aria-label="control.label" @click="activate(control.portId)")
    span(class="text-lg") {{ control.symbol }}
</template>

<script setup lang="ts">
import type { DeckWidgetInstance, WidgetIntent } from "@base/types/deckWidget";
const props = defineProps<{ instance: DeckWidgetInstance; interactive: boolean }>();
const emit = defineEmits<{ (event: "intent", intent: WidgetIntent): void }>();
const sequence = ref(0);
const controls = [
  { portId: "previous", label: "Previous", symbol: "◀|" },
  { portId: "playPause", label: "Play or pause", symbol: "▶" },
  { portId: "next", label: "Next", symbol: "|▶" },
];
const activate = (portId: string) => {
  if (!props.interactive) return;
  emit("intent", {
    instanceId: props.instance.id,
    portId,
    kind: "trigger",
    phase: "commit",
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource: "mouse",
  });
};
</script>
