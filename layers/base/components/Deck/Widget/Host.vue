<template lang="pug">
div(
  ref="hostRef"
  class="deck-widget-host relative h-full w-full min-h-0 min-w-0"
  :class="hostClasses"
  :style="hostStyle"
  :data-widget-instance-id="instance.id"
  :data-widget-definition-id="definition.id"
  :aria-disabled="!canInteract"
  :aria-busy="resolvedRuntimeState.status === 'loading' || resolvedRuntimeState.status === 'executing'"
  :tabindex="mode === 'readonly' || !canInteract ? -1 : 0"
  @pointerdown.capture="dismissErrorIndicator"
  @click.capture="dismissErrorIndicator"
  @click="handleHostClick"
  @keydown.enter.prevent="handleKeyboardActivate"
  @keydown.space.prevent="handleKeyboardActivate"
  @contextmenu="handleContextMenu"
)
  slot(:context="hostContext")

  div(
    v-if="resolvedRuntimeState.status === 'loading'"
    class="absolute inset-0 z-30 grid place-items-center rounded-[inherit] bg-black/20 pointer-events-none"
  )
    span(class="sr-only") Loading widget

  div(
    v-else-if="showErrorIndicator"
    data-widget-error-indicator
    class="absolute top-1 left-1 z-30 flex items-center gap-1 rounded-full bg-red-600/90 px-1.5 py-0.5 text-[9px] font-medium text-white shadow pointer-events-none"
    :title="resolvedRuntimeState.error?.message || 'Action error'"
  )
    span(class="inline-block w-1.5 h-1.5 rounded-full bg-white animate-pulse")
    span(class="max-w-[80px] truncate") {{ resolvedRuntimeState.error?.code || 'Error' }}

  div(
    v-else-if="resolvedRuntimeState.status === 'disconnected' && mode !== 'edit' && hasProviderObservation"
    class="absolute bottom-1 right-1 z-30 rounded-full bg-amber-500 px-1.5 py-0.5 text-[9px] font-bold text-black pointer-events-none"
  ) Unavailable · stale

  div(
    v-if="resolvedRuntimeState.request && mode !== 'edit'"
    class="absolute top-1 right-1 z-30 rounded-full bg-blue-500 px-1.5 py-0.5 text-[9px] font-bold text-white pointer-events-none"
  ) Pending

  slot(name="overlay" :context="hostContext")
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import type {
  DeckWidgetInstance,
  WidgetDefinition,
  WidgetHostContext,
  WidgetHostMode,
  WidgetIntent,
  WidgetRuntimeState,
} from "@base/types/deckWidget";

const props = withDefaults(defineProps<{
  instance: DeckWidgetInstance;
  definition: WidgetDefinition;
  mode?: WidgetHostMode;
  selected?: boolean;
  locked?: boolean;
  runtimeState?: WidgetRuntimeState;
  theme?: Record<string, unknown>;
  selectionCss?: string;
}>(), {
  mode: "edit",
  selected: false,
  locked: false,
  runtimeState: undefined,
  theme: () => ({}),
  selectionCss: "",
});

const emit = defineEmits<{
  (event: "select", payload: {
    instanceId: string;
    additive: boolean;
  }): void;
  (event: "open-context-menu", payload: {
    instanceId: string;
    bounds: { top: number; left: number; width: number; height: number };
  }): void;
  (event: "intent", intent: WidgetIntent): void;
}>();

const hostRef = ref<HTMLElement | null>(null);
const sequence = ref(0);
const interactionLocked = computed(() => props.locked && props.mode !== "edit");
const hasProviderObservation = computed(() =>
  Object.keys(resolvedRuntimeState.value.providerStates ?? {}).length > 0,
);

const resolvedRuntimeState = computed<WidgetRuntimeState>(() => props.runtimeState ?? ({
  instanceId: props.instance.id,
  revision: 0,
  status: "ready",
  values: {},
}));

// Dismiss presentation only: the host's revision, error and provider truth must
// remain intact so a failed action is never reported as recovered by a UI timer.
const showErrorIndicator = ref(false);
let errorIndicatorTimer: ReturnType<typeof setTimeout> | undefined;
const dismissErrorIndicator = () => {
  if (errorIndicatorTimer !== undefined) clearTimeout(errorIndicatorTimer);
  errorIndicatorTimer = undefined;
  showErrorIndicator.value = false;
};
watch([
  () => props.instance.id,
  () => resolvedRuntimeState.value.status,
  () => resolvedRuntimeState.value.error?.code,
  () => resolvedRuntimeState.value.error?.message,
], () => {
  dismissErrorIndicator();
  if (resolvedRuntimeState.value.status === "error") {
    showErrorIndicator.value = true;
    errorIndicatorTimer = setTimeout(dismissErrorIndicator, 3000);
  }
}, { immediate: true, flush: "sync" });
onUnmounted(dismissErrorIndicator);
const providerUnavailable = computed(() => {
  const providers = Object.values(resolvedRuntimeState.value.providerStates ?? {});
  return providers.length > 0 && providers.every(
    ({ availability }) => availability === "unavailable",
  );
});
const runtimeInteractionLocked = computed(() =>
  Boolean(resolvedRuntimeState.value.request) || providerUnavailable.value,
);
const canInteract = computed(() =>
  !interactionLocked.value && !runtimeInteractionLocked.value && props.instance.enabled,
);

const hostContext = computed<WidgetHostContext>(() => ({
  mode: props.mode,
  interactive: props.mode === "interactive" && canInteract.value,
  selected: props.selected,
  locked: props.locked,
  runtimeState: resolvedRuntimeState.value,
  theme: props.theme,
}));

const hostClasses = computed(() => ({
  "deck-widget-host--selected": props.selected && props.mode === "edit",
  "deck-widget-host--themed-selection": Boolean(props.selectionCss),
  "deck-widget-host--interactive": props.mode === "interactive",
  "deck-widget-host--locked": interactionLocked.value,
  "deck-widget-host--locked-edit": props.locked && props.mode === "edit",
  "deck-widget-host--disabled": !props.instance.enabled,
  "deck-widget-host--runtime-locked": runtimeInteractionLocked.value && props.mode === "interactive",
}));

const hostStyle = computed(() => (
  props.selected && props.mode === "edit" && props.selectionCss
    ? props.selectionCss
    : undefined
));

const handleHostClick = (event: MouseEvent) => {
  if (props.mode === "edit" && props.instance.enabled) {
    emit("select", {
      instanceId: props.instance.id,
      additive: event.shiftKey || event.ctrlKey || event.metaKey,
    });
  }
};

const handleKeyboardActivate = (event: KeyboardEvent) => {
  dismissErrorIndicator();
  if (!hostContext.value.interactive) return;
  const port = props.definition.inputs.find((input) => input.kind === "trigger");
  if (!port) return;
  emit("intent", {
    instanceId: props.instance.id,
    portId: port.id,
    legacyButtonId: props.instance.legacyButtonId,
    kind: "trigger",
    phase: "commit",
    sequence: ++sequence.value,
    timestamp: Date.now(),
    inputSource: "keyboard",
  });
  event.stopPropagation();
};

const handleContextMenu = (event: MouseEvent) => {
  if (props.mode !== "edit" || !hostRef.value) return;
  event.preventDefault();
  if (!props.instance.enabled) return;
  const rect = hostRef.value.getBoundingClientRect();
  emit("open-context-menu", {
    instanceId: props.instance.id,
    bounds: {
      top: rect.top,
      left: rect.left,
      width: rect.width,
      height: rect.height,
    },
  });
};
</script>

<style scoped>
.deck-widget-host {
  isolation: isolate;
  overflow: visible;
  contain: layout;
  outline: none;
}

.deck-widget-host:focus-visible {
  border-radius: 0.5rem;
  outline: 2px solid rgb(59 130 246);
  outline-offset: 2px;
}

.deck-widget-host--selected:not(.deck-widget-host--themed-selection) {
  border-radius: 0.75rem;
  box-shadow: inset 0 0 0 2px rgb(59 130 246);
}

.deck-widget-host--locked,
.deck-widget-host--disabled,
.deck-widget-host--runtime-locked {
  cursor: not-allowed;
}

.deck-widget-host--runtime-locked {
  pointer-events: none;
}
</style>
