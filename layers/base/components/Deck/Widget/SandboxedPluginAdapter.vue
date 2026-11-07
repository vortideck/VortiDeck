<template lang="pug">
iframe(
  ref="frameRef"
  class="h-full w-full border-0 bg-transparent"
  :title="definition.name"
  :srcdoc="sandboxDocument"
  sandbox="allow-scripts"
  referrerpolicy="no-referrer"
  @load="sendContext"
)
</template>

<script setup lang="ts">
import type {
  DeckWidgetInstance,
  WidgetDefinition,
  WidgetHostMode,
  WidgetIntent,
  WidgetIntentPhase,
  WidgetRuntimeState,
} from "@base/types/deckWidget";

const props = defineProps<{
  instance: DeckWidgetInstance;
  definition: WidgetDefinition;
  html: string;
  mode: WidgetHostMode;
  runtimeState: WidgetRuntimeState;
}>();

const emit = defineEmits<{
  (event: "intent", intent: WidgetIntent): void;
}>();

const frameRef = ref<HTMLIFrameElement | null>(null);
const sequence = ref(0);

const sandboxDocument = computed(() => `<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data: blob:; media-src data: blob:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'none'; font-src data:">
<style>html,body{width:100%;height:100%;margin:0;overflow:hidden;background:transparent}</style></head>
<body>${props.html}</body></html>`);

const sendContext = () => {
  frameRef.value?.contentWindow?.postMessage({
    type: "vortideck.widget.context",
    version: 1,
    instanceId: props.instance.id,
    mode: props.mode,
    config: props.instance.config,
    state: props.runtimeState,
  }, "*");
};

const validPhase = (phase: unknown): phase is WidgetIntentPhase => (
  phase === "start" || phase === "preview" || phase === "commit"
  || phase === "end" || phase === "cancel"
);

const handleMessage = (event: MessageEvent) => {
  if (event.source !== frameRef.value?.contentWindow) return;
  const payload = event.data as Record<string, unknown> | null;
  if (!payload || payload.type !== "vortideck.widget.intent") return;
  const portId = typeof payload.portId === "string" ? payload.portId : "";
  const port = props.definition.inputs.find((candidate) => candidate.id === portId);
  if (!port || payload.kind !== port.kind || !validPhase(payload.phase)) return;

  emit("intent", {
    instanceId: props.instance.id,
    portId,
    legacyButtonId: props.instance.legacyButtonId,
    kind: port.kind,
    phase: payload.phase,
    value: payload.value,
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource: "remote",
  });
};

onMounted(() => window.addEventListener("message", handleMessage));
onUnmounted(() => window.removeEventListener("message", handleMessage));
watch(() => [props.mode, props.instance.config, props.runtimeState] as const, sendContext, {
  deep: true,
});
</script>
