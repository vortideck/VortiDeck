<template lang="pug">
div(class="space-y-4")
  div
    h3(class="text-sm font-medium text-neutral-700 mb-3") Element Properties
    div(class="text-center py-4")
      svg(class="w-8 h-8 mx-auto text-neutral-300 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
      p(class="text-xs text-neutral-500") {{ element.type }} element properties coming soon
</template>

<script setup lang="ts">
interface Element {
  id: string
  type: string
  name: string
  style?: Record<string, any>
}

const props = defineProps<{
  element: Element
}>()

const emit = defineEmits<{
  'update-property': [event: { property: string, value: any }]
  'update-style': [event: { property: string, value: string }]
}>()
</script>
