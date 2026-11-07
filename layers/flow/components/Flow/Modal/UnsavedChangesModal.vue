<template lang="pug">
div(
  v-if="show"
  class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-[10000]"
  @click.self="$emit('cancel')"
)
  div(
    class="bg-themeBackground2 rounded-2xl p-6 max-w-md w-full mx-4 border border-themeBorder shadow-2xl"
    @click.stop=""
  )
    div(class="flex items-center gap-3 mb-4")
      div(class="w-10 h-10 bg-amber-500/15 border border-amber-500/30 rounded-xl flex items-center justify-center")
        span(class="text-amber-400 text-lg") ⚠️
      div
        h3(class="text-base font-bold text-themeText") Unsaved Changes
        p(class="text-xs text-themeText3") You have unsaved changes in your current flow

    div(class="mb-6")
      p(class="text-themeText2 text-xs leading-relaxed") {{ message }}

    div(class="flex justify-end gap-2.5")
      button(
        class="px-4 py-2 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder rounded-xl text-xs font-semibold text-themeText transition-colors cursor-pointer shadow-2xs"
        @click="$emit('cancel')"
      ) Cancel
      button(
        class="px-4 py-2 bg-red-500 hover:bg-red-600 rounded-xl text-xs font-semibold text-white transition-colors cursor-pointer shadow-xs"
        @click="$emit('confirm')"
      ) {{ confirmButtonText }}
</template>

<script setup lang="ts">
// Props
interface Props {
  show: boolean;
  message?: string;
  confirmButtonText?: string;
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  message: "Loading a new flow will discard your current progress. Are you sure you want to continue?",
  confirmButtonText: "Continue",
});

// Emits
const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<style lang="scss"></style> 