<template>
  <div class="flex items-center gap-2 w-full">
    <!-- Color preview with clickable picker -->
    <label class="relative cursor-pointer group">
      <input
        type="color"
        :value="modelValue"
        @input="handleColorInput"
        @click.stop
        @mousedown.stop
        class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
      />
      <div
        class="w-10 h-8 rounded-lg border border-themeBorder group-hover:border-themePrimary/50 transition-colors overflow-hidden shadow-2xs"
        :style="{ backgroundColor: modelValue }"
      >
        <div class="w-full h-full" :style="{ backgroundColor: modelValue }"></div>
      </div>
    </label>

    <!-- Hex input -->
    <input
      type="text"
      :value="modelValue"
      @input="handleTextInput"
      @click.stop
      @mousedown.stop
      placeholder="#000000"
      maxlength="7"
      class="flex-1 px-2.5 py-1.5 text-xs bg-themeBackground text-themeText border border-themeBorder rounded-xl font-mono uppercase placeholder-themeText3 focus:outline-none focus:border-themePrimary focus:ring-1 focus:ring-themePrimary/20 transition-all duration-200"
    />
  </div>
</template>

<script setup lang="ts">
interface Props {
  modelValue: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const handleColorInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  emit('update:modelValue', target.value.toUpperCase());
};

const handleTextInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  let value = target.value.trim();

  // Ensure it starts with #
  if (value && !value.startsWith('#')) {
    value = '#' + value;
  }

  // Validate hex color format
  const hexRegex = /^#[0-9A-Fa-f]{0,6}$/;
  if (value === '' || hexRegex.test(value)) {
    emit('update:modelValue', value.toUpperCase());
  }
};
</script>

<style scoped>
/* Hide the native color picker UI, show only our custom preview */
input[type="color"] {
  cursor: pointer;
}
</style>
