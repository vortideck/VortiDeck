<template lang="pug">
div(class="flex h-full w-full flex-col items-center justify-center overflow-hidden rounded-xl border border-white/10 bg-black/70 p-3 text-white")
  span(class="w-full truncate text-center text-[10px] uppercase tracking-wider text-white/60") {{ label }}
  strong(class="max-w-full truncate text-2xl tabular-nums") {{ displayValue }}
</template>

<script setup lang="ts">
import type { DeckWidgetInstance, WidgetRuntimeState } from "@base/types/deckWidget";
const props = defineProps<{ instance: DeckWidgetInstance; runtimeState: WidgetRuntimeState }>();
const label = computed(() => String(props.instance.config.label ?? "Telemetry"));
const displayValue = computed(() => {
  const key = String(props.instance.config.valueKey ?? "value");
  const value = props.runtimeState.values[key] ?? "—";
  return `${String(value)}${String(props.instance.config.unit ?? "")}`;
});
</script>
