<template lang="pug">
    div(class="min-h-screen")
      // Background
      div(class="fixed inset-0 bg-[url('/grid.svg')] bg-center [mask-image:linear-gradient(180deg,white,rgba(255,255,255,0))]")
      div(class="fixed inset-0 bg-gradient-to-br from-blue-500/10 via-purple-500/10 to-blue-500/10 animate-gradient")
    
      // Content
      Transition(name="fade" v-if="!routing")
        div(v-if="isOpen" @click="$emit('close')" tabindex="-1" class="backdrop-blur-[1.5px] fixed top-0 flex items-center bg-[rgba(0,0,0,0.75)] justify-center left-0 right-0 z-50 w-full h-full")
      
      Transition(name="jump" v-if="!routing") 
        div(v-if="isOpen" class="fixed top-0 bottom-0 pointer-events-auto flex items-center justify-center left-0 right-0 z-50 w-[90%] max-w-[30rem] mx-auto p-4")
          div(class="relative bg-gray-900/80 backdrop-blur rounded-lg shadow-xl w-full border border-blue-500/20")
            // Header
            div(class="flex items-center justify-between p-4 border-b border-blue-500/20")
              h3(class="text-xl font-semibold text-blue-400") Connection Lost
             
            // Body
            div(class="p-6 text-center")
              div(class="animate-spin rounded-full h-10 w-10 border-[3px] border-blue-900/30 border-t-blue-500 mx-auto mb-3")
              h4(class="text-base font-medium text-blue-300 mb-2") Reconnecting...
              p(class="text-sm text-blue-400") Lost connection to the server. Attempting to reconnect automatically.
    
    </template>

<script setup lang="ts">
import { ref } from "vue";
import { useConnectionStore } from "@/stores/Connection";

const router = useRouter();
const routing = ref(false);

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits(["close"]);
const connectionStore = useConnectionStore();
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
</style>
