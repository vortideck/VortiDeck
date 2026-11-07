<template lang="pug">
div(
  class="relative inline-block flex items-center"
  @mouseenter="showDropdown"
  @mouseleave="hideDropdown"
)
  //- Trigger element slot (e.g., info icon)
  slot(name="trigger")
    Fa6SolidCircleInfo(class="w-4 h-4 text-themeText2 cursor-pointer hover:text-themeText transition-colors")

  //- Dropdown card
  Transition(name="fade-slide")
    div(
      v-if="isVisible"
      class="absolute right-0 mt-2 p-4 w-72 rounded-xl border border-themeBorder bg-themeBg shadow-xl z-[999] pointer-events-auto"
      :class="positionClass"
      style="top: 100%;"
    )
      slot(name="content")
        div(class="flex flex-col gap-y-2 text-xs text-themeText2 leading-relaxed")
          p(class="font-semibold text-themeText text-sm mb-1") {{ title }}
          p {{ content }}
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import Fa6SolidCircleInfo from "~icons/fa6-solid/circle-info?width=16px&height=16px";

const props = defineProps({
  title: {
    type: String,
    default: "Information",
  },
  content: {
    type: String,
    default: "",
  },
  align: {
    type: String,
    default: "right", // "left" or "right"
  },
});

const isVisible = ref(false);

const positionClass = computed(() => {
  return props.align === "left" ? "left-0 origin-top-left" : "right-0 origin-top-right";
});

let timeoutId: any = null;

const showDropdown = () => {
  if (timeoutId) clearTimeout(timeoutId);
  isVisible.value = true;
};

const hideDropdown = () => {
  // Add a slight delay before hiding to prevent flickering when moving between trigger and dropdown
  timeoutId = setTimeout(() => {
    isVisible.value = false;
  }, 150);
};
</script>

<style scoped lang="scss">
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.2s cubic-bezier(0.16, 1, 0.3, 1), transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.96);
}

.border-themeBorder {
  border-color: hsl(var(--border2) / 0.8);
}
.bg-themeBg {
  background: hsl(var(--background2) / 0.98);
  backdrop-filter: blur(12px);
}
</style>
