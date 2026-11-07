<template lang="pug">
div(class="flex flex-col gap-y-1")
    p(class="whitespace-nowrap text-xs text-gray-400" :class="{ 'opacity-50': disabled }") {{ label }}
    label(class="relative w-12 h-6" :class="disabled ? 'cursor-not-allowed opacity-40' : 'cursor-pointer'")
        input(type="checkbox" :checked="modelValue" :disabled="disabled" class="sr-only peer" @change="$emit('update')")
        div(class="w-11 h-6 bg-[#2d2d2d] peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600")
    
</template>

<script setup lang="ts">
const props = defineProps({
  modelValue: {
    type: Boolean,
    required: true,
  },
  label: {
    type: String,
    default: "",
  },
  disabled: {
    type: Boolean,
    default: false,
  },
});
</script>

<style lang="scss"></style>
