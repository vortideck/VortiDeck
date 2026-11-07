<template lang="pug">
div(class="flex flex-col gap-y-1") 
    p(class="text-xs text-gray-400") Grid Size 
    div(class="flex gap-x-2")
        input(:value="modelValue.split(',')[0]" @input="setTwoValues($event.target.value, 0)" type="number" min="1" max="10" class="w-16 bg-[#2d2d2d] border border-gray-700 rounded px-2 py-1 text-sm")
        span(class="text-gray-400 text-sm") x
        input(:value="modelValue.split(',')[1]" @input="setTwoValues($event.target.value, 1)" type="number" min="1" max="10" class="w-16 bg-[#2d2d2d] border border-gray-700 rounded px-2 py-1 text-sm")
         
</template>

<script setup lang="ts">
const props = defineProps({
  modelValue: {
    type: String,
    required: true,
  },
});

const emit = defineEmits(["update"]);
const twoValues = ref([props.modelValue.split(",")[0], props.modelValue.split(",")[1]]);
const setTwoValues = (value: string, index: number) => {
  twoValues.value[index] = value;
  emit("update", twoValues.value.join(","));
};
</script>

<style lang="scss"></style>
