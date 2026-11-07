<template lang="pug">
div(class="flex flex-col gap-y-1")
    p(class="text-xs text-gray-400") {{ label }}
    input(type="range" class="w-32 accent-blue-500" :min="min" :max="max" :step="step" :value="modelValue" @input="$emit('update', parseFloat($event.target.value))" @change="$emit('release', parseFloat($event.target.value))")
</template>

<script setup lang="ts">
const props = defineProps({
  modelValue: {
    type: Number,
    required: true,
  },
  label: {
    type: String,
    default: "",
  },
  step: {
    type: Number,
    default: 0.1,
  },
  min: {
    type: Number,
    default: 0,
  },
  max: {
    type: Number,
    default: 15,
  },
});
</script>

<style lang="scss"></style>
