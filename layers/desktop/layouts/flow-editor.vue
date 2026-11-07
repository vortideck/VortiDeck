<template lang="pug">
div(class="flow-editor-layout flex flex-col w-full h-screen bg-themeBackground text-themeText select-none overflow-hidden")
  WindowsTitlebar(class="z-[10001]")
  div(class="flex-1 w-full h-[calc(100vh-30px)] mt-[30px] overflow-hidden flex flex-col")
    slot
</template>

<script setup lang="ts">
import { useMainStore } from "@/stores/Main";
const mainStore = useMainStore();
</script>

<style lang="scss">
.titlebar {
  z-index: 10000;
}
</style>
