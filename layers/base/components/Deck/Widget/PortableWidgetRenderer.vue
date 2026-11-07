<template>
  <div class="portable-widget-renderer h-full w-full min-h-0 min-w-0 overflow-hidden">
    <PortableWidgetNode
      v-if="validation.valid && definition.portableView"
      :node="definition.portableView.root"
      :definition="definition"
      :instance="instance"
      :state="runtimeState.values"
      :theme="theme"
      :interactive="interactive"
      @gesture="handleGesture"
    />
    <div v-else class="grid h-full w-full place-items-center p-3 text-center text-xs text-red-100">
      Invalid portable widget definition
    </div>
  </div>
</template>

<script setup lang="ts">
import PortableWidgetNode from "@base/components/Deck/Widget/PortableWidgetNode.vue";
import type {
  DeckWidgetInstance,
  PortableWidgetGestureEvent,
  WidgetDefinition,
  WidgetIntent,
  WidgetRuntimeState,
} from "@base/types/deckWidget";
import { validatePortableWidgetView } from "@base/utils/portableWidgetView";

const props = defineProps<{
  instance: DeckWidgetInstance;
  definition: WidgetDefinition;
  runtimeState: WidgetRuntimeState;
  interactive: boolean;
  theme?: Record<string, unknown>;
}>();
const emit = defineEmits<{ (event: "intent", intent: WidgetIntent): void }>();
const sequence = ref(0);
const validation = computed(() => validatePortableWidgetView(
  props.definition.portableView, props.definition,
));

const handleGesture = ({ binding, phase, value, inputSource }: PortableWidgetGestureEvent) => {
  const port = props.definition.inputs.find((candidate) => candidate.id === binding.portId);
  if (!port) return;
  emit("intent", {
    instanceId: props.instance.id,
    portId: port.id,
    legacyButtonId: props.instance.legacyButtonId,
    kind: port.kind,
    phase,
    value,
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource,
  });
};
</script>

<style scoped>
.portable-widget-renderer > :deep(.portable-widget-node) {
  width: 100%;
  height: 100%;
}
</style>
