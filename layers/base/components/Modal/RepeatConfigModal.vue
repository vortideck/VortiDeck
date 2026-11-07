<template lang="pug">
  Transition(name="fade" appear)
    div(v-if="isOpen")
      div(class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" @click="$emit('close')")
        div(class="bg-themeBackground p-6 rounded-lg shadow-xl w-96" @click.stop)
          p(class="text-lg font-bold mb-4 text-center") ⚙️ Repeat Configuration
          p(class="text-sm text-gray-600 mb-6 text-center") Configure timing for button G{{buttonPosition}}
          
          // Hold Threshold Configuration
          div(class="mb-4")
            label(class="block text-sm font-medium text-gray-700 mb-2") Hold Duration (ms)
            input(
              type="number"
              v-model.number="localConfig.hold_threshold_ms"
              min="100"
              max="2000"
              step="50"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="300"
            )
            p(class="text-xs text-gray-500 mt-1") Time before repeat starts (100-2000ms)
          
          // Repeat Interval Configuration  
          div(class="mb-6")
            label(class="block text-sm font-medium text-gray-700 mb-2") Repeat Interval (ms)
            input(
              type="number"
              v-model.number="localConfig.repeat_interval_ms"
              min="50"
              max="1000"
              step="25"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="100"
            )
            p(class="text-xs text-gray-500 mt-1") Time between repeats (50-1000ms)
          
          // Modal Actions
          div(class="flex justify-end space-x-3")
            button(
              class="px-4 py-2 rounded-lg text-gray-600 hover:bg-gray-200 transition-colors"
              @click="$emit('close')"
            ) Cancel
            button(
              class="bg-blue-500 text-white px-4 py-2 rounded-lg hover:bg-blue-600 transition-colors"
              @click="handleApply"
            ) Apply Configuration
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

interface RepeatConfig {
  hold_threshold_ms: number
  repeat_interval_ms: number
}

interface Props {
  isOpen: boolean
  buttonPosition?: number
  config: RepeatConfig
}

const props = defineProps<Props>()
const emit = defineEmits<{
  close: []
  apply: [config: RepeatConfig]
}>()

// Local config that can be edited
const localConfig = ref<RepeatConfig>({
  hold_threshold_ms: 300,
  repeat_interval_ms: 100
})

// Watch for config changes from parent
watch(() => props.config, (newConfig) => {
  if (newConfig) {
    localConfig.value = { ...newConfig }
  }
}, { immediate: true })

// Handle apply button click
const handleApply = () => {
  emit('apply', localConfig.value)
}
</script>

<style scoped>
/* Fade transition for modal */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>