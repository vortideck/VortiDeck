<template>
  <component
    :is="tagName"
    class="portable-widget-node"
    :class="[`portable-widget-node--${node.type}`]"
    :style="nodeStyle"
    :disabled="isNativeControl && !interactive"
    :type="node.type === 'press-zone' || node.type === 'toggle' ? 'button' : undefined"
    :min="isRangeControl ? node.min ?? 0 : undefined"
    :max="isRangeControl ? node.max ?? 100 : undefined"
    :step="isRangeControl ? node.step ?? 1 : undefined"
    :value="isRangeControl ? numericValue : undefined"
    @pointerdown="handlePointerDown"
    @pointerup="handlePointerUp"
    @pointercancel="handlePointerCancel"
    @input="handleRangePreview"
    @change="handleRangeCommit"
    @keydown="activeInputSource = 'keyboard'"
    @click="handleClick"
  >
    <template v-if="node.type === 'text' || node.type === 'badge'">
      {{ displayValue }}
    </template>
    <template v-else-if="node.type === 'icon'">
      {{ node.icon || displayValue }}
    </template>
    <img
      v-else-if="node.type === 'image' && imageSource"
      :src="imageSource"
      alt=""
      class="h-full w-full object-cover"
      draggable="false"
    >
    <template v-else-if="node.type === 'progress' || node.type === 'meter'">
      <span class="portable-widget-node__track">
        <span class="portable-widget-node__fill" :style="progressStyle" />
      </span>
      <span v-if="labelValue !== undefined" class="portable-widget-node__label">{{ labelValue }}</span>
    </template>
    <template v-else-if="node.type === 'toggle'">
      <span class="portable-widget-node__toggle" :data-active="Boolean(displayValue)" />
      <span v-if="labelValue !== undefined">{{ labelValue }}</span>
    </template>
    <PortableWidgetNode
      v-for="(child, index) in node.children || []"
      :key="child.id || `${node.id || node.type}-${index}`"
      :node="child"
      :definition="definition"
      :instance="instance"
      :state="state"
      :theme="theme"
      :interactive="interactive"
      @gesture="$emit('gesture', $event)"
    />
  </component>
</template>

<script setup lang="ts">
import type {
  DeckWidgetInstance,
  PortableWidgetGestureBinding,
  PortableWidgetGestureEvent,
  PortableWidgetNode,
  WidgetDefinition,
  WidgetInputSource,
  WidgetIntentPhase,
} from "@base/types/deckWidget";
import { resolvePortableWidgetValue } from "@base/utils/portableWidgetView";
import { usePortableWidgetAssets } from "@base/composables/usePortableWidgetAssets";

const props = defineProps<{
  node: PortableWidgetNode;
  definition: WidgetDefinition;
  instance: DeckWidgetInstance;
  state: Record<string, unknown>;
  theme?: Record<string, unknown>;
  interactive: boolean;
}>();
const emit = defineEmits<{ (event: "gesture", gesture: PortableWidgetGestureEvent): void }>();
const activeInputSource = ref<WidgetInputSource>("mouse");
const portableAssets = usePortableWidgetAssets();

const isRangeControl = computed(() => props.node.type === "slider" || props.node.type === "rotary");
const isNativeControl = computed(() => (
  props.node.type === "press-zone" || props.node.type === "toggle" || isRangeControl.value
));
const tagName = computed(() => {
  if (props.node.type === "press-zone" || props.node.type === "toggle") return "button";
  if (isRangeControl.value) return "input";
  return "div";
});
const value = computed(() => resolvePortableWidgetValue(
  props.node.value, props.instance.config, props.state,
));
const labelValue = computed(() => resolvePortableWidgetValue(
  props.node.label, props.instance.config, props.state,
));
const displayValue = computed(() => value.value ?? labelValue.value ?? "");
const numericValue = computed(() => {
  const resolved = Number(value.value ?? props.node.min ?? 0);
  return Number.isFinite(resolved) ? resolved : Number(props.node.min ?? 0);
});
const imageSource = computed(() => {
  if (!props.node.source) return undefined;
  if ("asset" in props.node.source) {
    return portableAssets.resolve(props.definition, props.node.source.asset);
  }
  const source = resolvePortableWidgetValue(props.node.source, props.instance.config, props.state);
  if (typeof source !== "string") return undefined;
  return /^(?:blob:|data:image\/(?:png|jpeg|webp);base64,)/i.test(source) ? source : undefined;
});
const token = (name: string | undefined) => {
  if (!name) return undefined;
  const direct = props.theme?.[name];
  return typeof direct === "string" ? direct : `var(--vd-widget-${name})`;
};
const nodeStyle = computed(() => ({
  gap: props.node.gap === undefined ? undefined : `${props.node.gap}px`,
  padding: props.node.padding === undefined ? undefined : `${props.node.padding}px`,
  borderRadius: props.node.radius === undefined ? undefined : `${props.node.radius}px`,
  color: token(props.node.colorToken),
  background: token(props.node.backgroundToken),
  fontSize: props.node.fontSize === undefined ? undefined : `${props.node.fontSize}px`,
  fontWeight: props.node.fontWeight,
  alignItems: props.node.align,
  justifyContent: props.node.justify === "between" ? "space-between" : props.node.justify,
  gridTemplateColumns: props.node.type === "grid"
    ? `repeat(${props.node.columns ?? 1}, minmax(0, 1fr))`
    : undefined,
  WebkitLineClamp: props.node.maxLines,
}));
const progressStyle = computed(() => {
  const min = props.node.min ?? 0;
  const max = props.node.max ?? 100;
  const percent = Math.max(0, Math.min(100, ((numericValue.value - min) / (max - min || 1)) * 100));
  return { width: `${percent}%` };
});

const pointerSource = (event: PointerEvent): WidgetInputSource => (
  event.pointerType === "touch" || event.pointerType === "pen" ? event.pointerType : "mouse"
);
const bindingFor = (...gestures: PortableWidgetGestureBinding["gesture"][]) => (
  props.node.gestures?.find((binding) => gestures.includes(binding.gesture))
);
const send = (
  binding: PortableWidgetGestureBinding | undefined,
  phase: WidgetIntentPhase,
  inputSource: WidgetInputSource,
  nextValue?: unknown,
) => {
  if (!props.interactive || !binding) return;
  emit("gesture", { binding, phase, value: nextValue, inputSource });
};
const handlePointerDown = (event: PointerEvent) => {
  activeInputSource.value = pointerSource(event);
  if (isRangeControl.value) return;
  const binding = bindingFor("hold", "press");
  if (binding?.delivery === "start_end" || binding?.gesture === "hold") {
    send(binding, "start", pointerSource(event));
  }
};
const handlePointerUp = (event: PointerEvent) => {
  if (isRangeControl.value) return;
  const binding = bindingFor("release", "hold", "press");
  if (binding && (binding.gesture !== "press" || binding.delivery === "start_end")) {
    send(binding, "end", pointerSource(event));
  }
};
const handlePointerCancel = (event: PointerEvent) => {
  send(bindingFor("release", "hold", "press"), "cancel", pointerSource(event));
};
const handleClick = (event: MouseEvent) => {
  if (props.node.type !== "press-zone" && props.node.type !== "toggle") return;
  const binding = bindingFor("press");
  if (binding?.delivery !== "start_end") send(binding, "commit", "mouse", props.node.type === "toggle"
    ? !Boolean(value.value) : undefined);
  event.stopPropagation();
};
const rangeValue = (event: Event) => Number((event.target as HTMLInputElement).value);
const handleRangePreview = (event: Event) => send(
  bindingFor(props.node.type === "rotary" ? "rotate" : "drag"), "preview", activeInputSource.value, rangeValue(event),
);
const handleRangeCommit = (event: Event) => send(
  bindingFor(props.node.type === "rotary" ? "rotate" : "drag"), "commit", activeInputSource.value, rangeValue(event),
);
</script>

<style scoped>
.portable-widget-node { box-sizing: border-box; min-width: 0; min-height: 0; max-width: 100%; max-height: 100%; overflow: hidden; }
.portable-widget-node--stack,
.portable-widget-node--column,
.portable-widget-node--panel,
.portable-widget-node--press-zone { display: flex; flex-direction: column; }
.portable-widget-node--row { display: flex; flex-direction: row; }
.portable-widget-node--grid { display: grid; }
.portable-widget-node--press-zone,
.portable-widget-node--toggle { border: 0; color: inherit; }
.portable-widget-node--press-zone { width: 100%; height: 100%; }
.portable-widget-node--text { overflow: hidden; display: -webkit-box; -webkit-box-orient: vertical; }
.portable-widget-node--badge { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.portable-widget-node--slider,
.portable-widget-node--rotary { width: 100%; accent-color: currentColor; }
.portable-widget-node__track { display: block; width: 100%; height: 0.5rem; overflow: hidden; border-radius: 999px; background: rgb(255 255 255 / 18%); }
.portable-widget-node__fill { display: block; height: 100%; border-radius: inherit; background: currentColor; }
.portable-widget-node__toggle { width: 2.25rem; height: 1.25rem; border-radius: 999px; background: rgb(255 255 255 / 25%); }
.portable-widget-node__toggle[data-active="true"] { background: currentColor; }
</style>
