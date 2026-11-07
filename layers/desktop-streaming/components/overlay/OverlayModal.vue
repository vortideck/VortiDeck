<template lang="pug">
BaseModal(
  :is-open="show"
  :z-index="zIndex"
  @close="$emit('close')"
)
  div(
    class="vd-modal-card relative bg-themeBackground2 rounded-2xl shadow-2xl w-full flex flex-col border border-themeBorder text-themeText overflow-hidden"
    :class="modalClass"
  )
    //- Header
    div(class="flex items-center justify-between p-6 border-b border-themeBorder shrink-0 select-none")
      div
        h2(class="text-xl font-bold text-themeText") {{ title }}
        p(v-if="subtitle" class="text-sm text-themeText2") {{ subtitle }}
      button(
        type="button"
        @click="$emit('close')"
        class="p-2 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-xl transition-colors cursor-pointer"
        title="Close"
      )
        svg(class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
    
    //- Content
    div(class="flex-1 h-full overflow-hidden flex flex-col min-h-0")
      slot
    
    //- Footer
    div(v-if="$slots.footer" class="p-6 border-t border-themeBorder shrink-0")
      slot(name="footer")
</template>

<script setup lang="ts">
import BaseModal from "../../../desktop/components/Modal/BaseModal.vue";

const props = withDefaults(
  defineProps<{
    show?: boolean;
    title: string;
    subtitle?: string;
    modalClass?: string;
    zIndex?: number | null;
  }>(),
  {
    show: false,
    subtitle: "",
    modalClass: "max-w-6xl max-h-[90vh] h-[80vh]",
    zIndex: 50000,
  }
);

const emit = defineEmits<{
  close: [];
}>();
</script>