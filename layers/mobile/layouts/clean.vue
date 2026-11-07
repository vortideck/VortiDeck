<template lang="pug">
div(id="clean" class="h-[100vh] max-h-[100vh] w-full overflow-hidden bg-themeBackground" style="touch-action: none;" )
    <slot class="" />
    ModalReconnecting(:isOpen="shouldShowReconnectingModal && !isRelayMode")
    ModalReconnectingRelay(:isOpen="shouldShowReconnectingModal && isRelayMode")

</template>

<script setup lang="ts">
import { useConnectionStore } from "@/stores/Connection";
import { computed } from "vue";

const connectionStore = useConnectionStore();
const route = useRoute();

const isRelayMode = computed(() => connectionStore.getConnectionMode === "relay");

// Only show reconnecting modal on deck page
const shouldShowReconnectingModal = computed(() => {
  const isDeckPage = route.path === "/deck";
  const isNotPaired = connectionStore.getStatus !== "paired";
  return isDeckPage && isNotPaired;
});
</script>

<style lang="scss"></style>
