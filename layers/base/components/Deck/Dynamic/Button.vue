<template lang="pug">
div(@click="haptics" @mousedown="$emit('mousedown', $event)" @mouseup="$emit('mouseup', $event)" @mouseleave="$emit('mouseleave', $event)" @touchstart="$emit('touchstart', $event)" @touchend="$emit('touchend', $event)" :class="[theme.buttonCss? '': 'border-[1px] rounded-xl border-[#1a1a1a35] bg-[#1a1a1af7] shadow-md ', fill ? 'h-full w-full' : 'h-min w-full aspect-square', {'button': isReactive}]" class="relative overflow-hidden" :style="`padding: ${deckStore.getActiveDeck.settings.buttons_padding}px; ${theme?.buttonCss}`" @contextmenu="openOptions($event)" ref="buttonRef")
  nuxt-img( draggable="false" v-if="iconUrlComputed && !isAlt && !isShift" :src="iconUrlComputed" @error="handleImageError" class="w-full h-full" :style="`${theme?.iconCss}; object-fit: ${imageFit};`" placeholder="/transparent.png")
  p(v-if="isAlt || isShift" class="flex text-center items-center justify-center text-white w-full h-full text-xs") {{data.position}}
  //- p(v-else class="text-white") {{data.action}}

</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { impactFeedback } from "@tauri-apps/plugin-haptics";
import { useDeckStore } from "@base/stores/Deck";

const deckStore = useDeckStore();
const haptics = async () => {
  if (props.isReactive) {
    await impactFeedback("soft");
  }
};

const props = defineProps({
  data: {
    type: Object,
    required: false,
  },
  iconUrl: {
    type: String,
    required: false,
  },
  isAlt: {
    type: Boolean,
    required: false,
  },
  isShift: {
    type: Boolean,
    required: false,
  },
  theme: {
    type: Object,
    required: false,
  },
  isReactive: {
    type: Boolean,
    required: false,
  },
  fill: {
    type: Boolean,
    required: false,
    default: false,
  },
  imageFit: {
    type: String as () => "cover" | "contain",
    required: false,
    default: "cover",
  },
});

const emit = defineEmits(["openButtonOptions", "mousedown", "mouseup", "mouseleave", "touchstart", "touchend"]);

const buttonPosition = ref({ top: 0, left: 0, width: 0, height: 0 });
const buttonRef = ref<HTMLElement | null>(null);

const hasError = ref(false);

watch(() => props.iconUrl, () => {
  hasError.value = false;
});

const iconUrlComputed = computed(() => {
  if (hasError.value) return undefined;
  return props.iconUrl;
});

const openOptions = (event: MouseEvent) => {
  event.preventDefault();
  if (buttonRef.value) {
    const rect = buttonRef.value.getBoundingClientRect();
    buttonPosition.value = {
      top: rect.top,
      left: rect.left,
      width: rect.width,
      height: rect.height,
    };
  }
  emit("openButtonOptions", buttonPosition.value, props.data?.id);
};

const handleImageError = () => {
  console.warn("Image failed to load:", props.iconUrl);
  hasError.value = true;
};
</script>

<style lang="scss" scoped>
.button {
  transition: all 0.2s ease-in-out;
}

.button:active {
  transform: scale(1.1);
  transition: 0.1s;
}
</style>
