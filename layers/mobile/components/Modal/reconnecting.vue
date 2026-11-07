<template lang="pug">
div
  Transition(name="fade" v-if="!routing")
    div(v-if="isOpen" tabindex="-1" class="backdrop-blur-[1.5px] fixed top-0 flex items-center bg-[rgba(0,0,0,0.55)] justify-center left-0 right-0 z-[999] w-full h-full pointer-events-auto")

  Transition(name="jump" v-if="!routing")
    div(v-if="isOpen" class=" fixed top-0 bottom-0 pointer-events-auto flex items-center justify-center left-0 right-0 z-[999] w-[90%] max-w-[30rem] mx-auto p-4" @click.stop)
      div(class="relative bg-themeBackground2 rounded-lg shadow-lg w-full bg-white pointer-events-auto" @click.stop)
        // Header
        div(class="flex items-center justify-between p-4 border-b")
          h3(class="text-xl font-semibold text-themeText2") Desktop Offline


        // Body
        div(class="p-6 text-center")
          div(class="enhanced-spin rounded-full h-10 w-10 border-[3px] border-gray-800 border-t-blue-600 mx-auto mb-3")
          h4(class="text-base font-medium text-themeText2 mb-2") Waiting for Desktop...
          p(class="text-sm text-themeText3 mb-4") Your desktop went offline. We'll reconnect automatically when it comes back on your network.

          button(@click.stop="goBack" class="mt-4 px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors pointer-events-auto") Back to Home

</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from "vue";
import { useConnectionStore } from "@/stores/Connection";

const router = useRouter();
const routing = ref(false);
const connectionStore = useConnectionStore();

const props = defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits(["close"]);

let reconnectInterval: any = null;

// Watch for desktop coming back online
watch(() => connectionStore.getStatus, (newStatus) => {
  if (newStatus === 'paired' && props.isOpen) {
    console.log("✅ Desktop came back online - connection restored");
    // Connection will be restored by the WebSocket handlers
  }
});

// Periodically check if we should try to reconnect
onMounted(() => {
  reconnectInterval = setInterval(() => {
    if (props.isOpen && connectionStore.getConnectionMode === 'direct') {
      // Check will happen automatically via WebSocket reconnection
      console.log("🔄 Checking for desktop availability...");
    }
  }, 5000); // Check every 5 seconds
});

onUnmounted(() => {
  if (reconnectInterval) {
    clearInterval(reconnectInterval);
  }
});

function goBack() {
  routing.value = true;
  router.push('/');
}
</script>

<style lang="scss">
.fade-enter-active {
  animation: fade-in 0.2s;
}

.fade-leave-active {
  animation: fade-out 0.2s;
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes fade-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}

.jump-enter-active {
  animation: jump-in 0.2s;
}

.jump-leave-active {
  animation: jump-out 0.2s;
}

@keyframes jump-in {
  0% {
    transform: translateY(50px);
    opacity: 0;
  }
  100% {
    transform: translateY(0);
    opacity: 1;
  }
}

@keyframes jump-out {
  0% {
    transform: translateY(0);
    opacity: 1;
  }
  100% {
    transform: translateY(50px);
    opacity: 0;
  }
}

.enhanced-spin {
  animation: enhanced-spin 1s linear infinite;
}

@keyframes enhanced-spin {
  0% {
    transform: rotate(0deg) scale(1);
  }
  50% {
    transform: rotate(180deg) scale(1.05);
  }
  100% {
    transform: rotate(360deg) scale(1);
  }
}
</style>
