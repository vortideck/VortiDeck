<template lang="pug">
div(
  class="deck-widget-canvas grid h-full w-full min-h-0 min-w-0"
  :style="canvasStyle"
  :data-layout-key="layoutKey"
)
  div(
    v-for="instance in visibleInstances"
    :key="instance.id"
    class="min-h-0 min-w-0"
    :style="itemStyle(instance)"
    :data-deck-widget-item="instance.id"
  )
    slot(:instance="instance" :placement="instance.layouts[layoutKey]")
</template>

<script setup lang="ts">
import type { CSSProperties } from "vue";
import type { DeckWidgetInstance } from "@base/types/deckWidget";

const props = withDefaults(defineProps<{
  instances: DeckWidgetInstance[];
  columns: number;
  rows: number;
  layoutKey?: string;
  gap?: number | string;
}>(), {
  layoutKey: "default",
  gap: 0,
});

const visibleInstances = computed(() => props.instances.filter(
  (instance) => instance.enabled && instance.layouts[props.layoutKey],
));

const canvasStyle = computed<CSSProperties>(() => ({
  gridTemplateColumns: `repeat(${Math.max(1, props.columns)}, minmax(0, 1fr))`,
  gridTemplateRows: `repeat(${Math.max(1, props.rows)}, minmax(0, 1fr))`,
  gap: typeof props.gap === "number" ? `${props.gap}px` : props.gap,
}));

const itemStyle = (instance: DeckWidgetInstance): CSSProperties => {
  const placement = instance.layouts[props.layoutKey];
  if (!placement) return { display: "none" };
  return {
    gridColumn: `${placement.x + 1} / span ${placement.cols}`,
    gridRow: `${placement.y + 1} / span ${placement.rows}`,
  };
};
</script>
