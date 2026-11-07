<template lang="pug">
LegacyActionButtonAdapter(
  :button="button"
  :instance-id="instance.id"
  :interactive="interactive"
  :is-alt="deckStore.getIsAlt"
  :is-shift="deckStore.getIsShift"
  :theme="buttonTheme"
  :icon-url="iconUrl"
  :image-fit="imageFit"
  @intent="$emit('intent', $event)"
)
</template>

<script setup lang="ts">
import { computed } from "vue";
import LegacyActionButtonAdapter from "@base/components/Deck/Widget/LegacyActionButtonAdapter.vue";
import { useDeckStore } from "@base/stores/Deck";
import type { DeckWidgetInstance, WidgetIntent } from "@base/types/deckWidget";
import type { LegacyDeckButton } from "@base/utils/legacyDeckWidget";

const props = defineProps<{
  instance: DeckWidgetInstance;
  interactive: boolean;
  theme?: Record<string, unknown>;
}>();
defineEmits<{ (event: "intent", intent: WidgetIntent): void }>();
const deckStore = useDeckStore();

const binding = computed(() => props.instance.bindings?.find(({ portId }) => portId === "primary"));
const persistedButton = computed(() => deckStore.getActiveDeck?.buttons?.find(
  ({ id }) => id === props.instance.legacyButtonId,
));
const iconUrl = computed(() => {
  const value = props.instance.config?.iconUrl;
  if (typeof value === "string" && value) return value;
  const button = persistedButton.value;
  if (button?.icon) {
    if (button.icon.startsWith("http://") || button.icon.startsWith("https://") || button.icon.startsWith("data:")) {
      return button.icon;
    }
  }
  const buttonId = button?.id ?? props.instance.legacyButtonId;
  if (deckStore.iconUrlBase && buttonId != null) {
    return `${deckStore.iconUrlBase}${buttonId}/icon.webp?${deckStore.iconRefreshKey || Date.now()}`;
  }
  return typeof button?.icon === "string" && button.icon ? button.icon : undefined;
});
const imageFit = computed(() => (
  persistedButton.value
    ? (persistedButton.value.icon_fit === "contain" ? "contain" : "cover")
    : (props.instance.config?.imageFit === "contain" ? "contain" : "cover")
));
const buttonTheme = computed(() => (
  (props.theme?.components as Record<string, Record<string, string>> | undefined)?.clickButton ?? {}
));
const button = computed<LegacyDeckButton>(() => ({
  id: props.instance.legacyButtonId ?? props.instance.id,
  deck_id: 0,
  label: String(props.instance.config?.label ?? persistedButton.value?.label ?? ""),
  icon: iconUrl.value ?? null,
  action: typeof binding.value?.action === "string" ? binding.value.action : (persistedButton.value?.action ?? null),
  position: Number(props.instance.config?.position ?? persistedButton.value?.position ?? 1),
  background: null,
  mode: "click/press",
  execution_mode: persistedButton.value?.execution_mode ?? null,
  execution_config: persistedButton.value?.execution_config ?? null,
  icon_updated_at: null,
  icon_fit: imageFit.value,
}));
</script>
