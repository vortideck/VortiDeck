<template lang="pug">
SandboxedPluginAdapter(
  v-if="registration?.sandboxHtml"
  :instance="instance"
  :definition="definition"
  :html="registration.sandboxHtml"
  :mode="mode"
  :runtime-state="runtimeState"
  @intent="$emit('intent', $event)"
)
component(
  v-else-if="registration?.renderer"
  :is="registration.renderer"
  :instance="instance"
  :definition="definition"
  :runtime-state="runtimeState"
  :interactive="interactive"
  :theme="theme"
  @intent="$emit('intent', $event)"
)
PortableWidgetRenderer(
  v-else-if="definition.portableView"
  :instance="instance"
  :definition="definition"
  :runtime-state="runtimeState"
  :interactive="interactive"
  :theme="theme"
  @intent="$emit('intent', $event)"
)
div(v-else class="grid h-full w-full place-items-center rounded-xl border border-dashed border-amber-400/50 bg-amber-950/40 p-3 text-center text-xs text-amber-100")
  div
    strong(class="block") {{ definition.name }}
    span(class="mt-1 block opacity-75") {{ fallbackMessage }}
</template>

<script setup lang="ts">
import SandboxedPluginAdapter from "@base/components/Deck/Widget/SandboxedPluginAdapter.vue";
import PortableWidgetRenderer from "@base/components/Deck/Widget/PortableWidgetRenderer.vue";
import { useWidgetRegistry } from "@base/composables/useWidgetRegistry";
import type {
  DeckWidgetInstance,
  WidgetDefinition,
  WidgetHostMode,
  WidgetIntent,
  WidgetRuntimeState,
} from "@base/types/deckWidget";

const props = defineProps<{
  instance: DeckWidgetInstance;
  definition: WidgetDefinition;
  runtimeState: WidgetRuntimeState;
  mode: WidgetHostMode;
  interactive: boolean;
  theme?: Record<string, unknown>;
}>();
defineEmits<{ (event: "intent", intent: WidgetIntent): void }>();
const registry = useWidgetRegistry();
const registration = computed(() => registry.resolve(props.definition.id));
const fallbackMessage = computed(() => {
  if (props.definition.mobileSupport === "fallback_only") {
    return "This widget is available on desktop only.";
  }
  if (props.instance.packageId) {
    return "The widget package or approved renderer is unavailable on this device.";
  }
  return "Renderer unavailable.";
});
</script>
