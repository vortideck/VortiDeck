<template>
  <div class="flex items-center gap-2">
    <button
      type="button"
      @click="toggle"
      @click.stop
      @mousedown.stop
      :class="[
        'relative inline-flex h-5 w-10 items-center rounded-full transition-colors duration-200 ease-in-out focus:outline-none cursor-pointer border border-themeBorder',
        modelValue ? 'bg-themePrimary border-themePrimary' : 'bg-themeBackground3'
      ]"
    >
      <span
        :class="[
          'inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-200 ease-in-out shadow-xs',
          modelValue ? 'translate-x-5' : 'translate-x-0.5'
        ]"
      />
    </button>
    <span class="text-xs text-themeText2 font-medium">
      {{ modelValue ? 'On' : 'Off' }}
    </span>
  </div>
</template>

<script setup lang="ts">
interface Props {
  modelValue: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
}>();

const toggle = () => {
  emit('update:modelValue', !props.modelValue);
};
</script>
