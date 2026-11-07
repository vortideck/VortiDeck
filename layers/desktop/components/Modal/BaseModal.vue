<template lang="pug">
div
  Transition(name="fade")
    div(
      v-if="isOpen"
      tabindex="-1"
      class="vd-modal-overlay"
      :class="overlayClass"
      :style="overlayStyle"
      @mousedown.self="onBackdropMouseDown"
    )

  Transition(name="jump")
    div(v-if="isOpen" class="vd-modal-shell" :class="shellClass" :style="shellStyle")
      slot
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    isOpen: boolean;
    closeOnBackdrop?: boolean;
    overlayClass?: string;
    shellClass?: string;
    zIndex?: number | null;
  }>(),
  {
    closeOnBackdrop: true,
    overlayClass: "",
    shellClass: "",
    zIndex: null,
  }
);

const emit = defineEmits<{
  close: [];
}>();

const onBackdropMouseDown = () => {
  if (props.closeOnBackdrop) {
    emit("close");
  }
};

const overlayStyle = computed(() => {
  if (props.zIndex == null) {
    return undefined;
  }
  return { zIndex: String(props.zIndex) };
});

const shellStyle = computed(() => {
  if (props.zIndex == null) {
    return undefined;
  }
  return { zIndex: String(props.zIndex + 10) };
});
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;
</style>
