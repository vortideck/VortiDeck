<template lang="pug">
Teleport(to="body")
  Transition(name="fade")
    div(v-if="isOpen" class="fixed inset-0 z-[9999]")
      // Titlebar area - visually covered but no pointer events
      div(class="absolute top-0 left-0 right-0 h-[30px] bg-slate-900/55 pointer-events-none")
      
      // Main clickable area - below titlebar
      div(@click="$emit('resume')" class="absolute top-[30px] left-0 right-0 bottom-0 bg-slate-900/45 backdrop-blur-[4px] flex items-center justify-center cursor-pointer")
  
  Transition(name="jump")
    div(v-if="isOpen" class="fixed inset-0 flex items-center justify-center z-[9999] pointer-events-none")
      div(@click.stop="$emit('resume')" class="modal-content vd-modal-card p-8 text-center pointer-events-auto cursor-pointer")
        div(class="w-8 h-8 bg-yellow-500/20 rounded-full flex items-center justify-center mx-auto mb-4")
          svg(class="w-4 h-4 text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z")
        
        h3(class="text-lg font-semibold text-themeText mb-2") Recording Paused
        p(class="text-sm text-themeText2 mb-4") Click anywhere to resume
        
        button(
          @click="$emit('resume')"
          class="vd-modal-primary-btn px-4 py-2 text-sm"
        ) Resume
</template>

<script setup lang="ts">
defineProps<{
  isOpen: boolean;
}>();

defineEmits<{
  resume: [];
}>();
</script>

<style scoped lang="scss">
@use "./modalTheme.scss" as *;

.modal-content {
  min-width: 300px;
}
</style>
