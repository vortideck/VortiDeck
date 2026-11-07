<template>
  <div class="flex items-center gap-2 w-full">
    <input
      type="range"
      :min="min"
      :max="max"
      :step="step"
      :value="modelValue"
      @input="handleInput"
      @click.stop
      @mousedown.stop
      class="flex-1 h-2 bg-themeBackground3 border border-themeBorder rounded-lg appearance-none cursor-pointer slider-thumb"
    />
    <input
      type="number"
      :min="min"
      :max="max"
      :step="step"
      :value="modelValue"
      @input="handleInput"
      @click.stop
      @mousedown.stop
      class="w-16 px-2 py-1 text-xs bg-themeBackground text-themeText border border-themeBorder rounded-lg focus:border-themePrimary focus:ring-1 focus:ring-themePrimary/20 transition-all duration-200"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  modelValue: number;
  min?: number;
  max?: number;
  step?: number;
}

const props = withDefaults(defineProps<Props>(), {
  min: 0,
  max: 100,
  step: 1,
});

const emit = defineEmits<{
  'update:modelValue': [value: number];
}>();

const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const value = Number(target.value);

  // Clamp value within min/max range
  const clampedValue = Math.min(Math.max(value, props.min), props.max);

  emit('update:modelValue', clampedValue);
};
</script>

<style scoped>
/* Custom slider styling with proper vertical alignment */
.slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  background: hsl(var(--background3));
  outline: none;
}

/* Webkit (Chrome, Safari, Edge) */
.slider-thumb::-webkit-slider-runnable-track {
  width: 100%;
  height: 8px;
  border-radius: 4px;
  background: hsl(var(--background3));
}

.slider-thumb::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: hsl(var(--primary));
  cursor: pointer;
  box-shadow: 0 0 6px hsl(var(--primary) / 0.4);
  transition: all 0.2s ease;
  margin-top: -4px; /* Centers thumb on track */
}

.slider-thumb::-webkit-slider-thumb:hover {
  transform: scale(1.1);
  box-shadow: 0 0 10px hsl(var(--primary) / 0.6);
}

/* Firefox */
.slider-thumb::-moz-range-track {
  width: 100%;
  height: 8px;
  border-radius: 4px;
  background: hsl(var(--background3));
}

.slider-thumb::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: hsl(var(--primary));
  cursor: pointer;
  border: none;
  box-shadow: 0 0 6px hsl(var(--primary) / 0.4);
  transition: all 0.2s ease;
}

.slider-thumb::-moz-range-thumb:hover {
  transform: scale(1.1);
  box-shadow: 0 0 10px hsl(var(--primary) / 0.6);
}
</style>
