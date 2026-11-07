<template>
  <div class="flex items-center gap-2 w-full">
    <!-- File path display -->
    <input
      type="text"
      :value="displayPath"
      readonly
      @click.stop
      @mousedown.stop
      :placeholder="placeholder"
      class="flex-1 px-2.5 py-1.5 text-xs bg-themeBackground text-themeText border border-themeBorder rounded-xl cursor-default truncate placeholder-themeText3"
      :title="modelValue || 'No file selected'"
    />

    <!-- Browse button -->
    <button
      @click.stop="handleBrowse"
      @mousedown.stop
      class="px-3 py-1.5 text-xs bg-themeButton hover:bg-themeButtonHover text-white rounded-xl transition-colors duration-200 flex items-center gap-1.5 whitespace-nowrap cursor-pointer shadow-xs font-medium"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
      </svg>
      Browse
    </button>

    <!-- Clear button (only show if file selected) -->
    <button
      v-if="modelValue"
      @click.stop="handleClear"
      @mousedown.stop
      class="px-2 py-1.5 text-xs bg-red-500 hover:bg-red-600 text-white rounded-xl transition-colors duration-200 cursor-pointer shadow-xs"
      title="Clear selection"
    >
      ×
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { open, save } from '@tauri-apps/plugin-dialog';

interface Props {
  modelValue: string;
  accept?: string[]; // File extensions like [".mp3", ".wav", ".ogg"]
  placeholder?: string;
  mode?: 'open' | 'save' | 'directory'; // Selection mode
}

const props = withDefaults(defineProps<Props>(), {
  accept: () => [],
  placeholder: 'No file selected',
  mode: 'open',
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

// Display only the filename, not the full path
const displayPath = computed(() => {
  if (!props.modelValue) return '';

  // Extract filename from path (works for both Windows and Unix paths)
  const parts = props.modelValue.replace(/\\/g, '/').split('/');
  return parts[parts.length - 1];
});

const handleBrowse = async () => {
  try {
    // Build filters from accept array
    const filters = props.accept && props.accept.length > 0
      ? [{
          name: 'Allowed Files',
          extensions: props.accept.map(ext => ext.replace('.', '')),
        }]
      : undefined;

    let selected;

    if (props.mode === 'directory') {
      // Select directory
      selected = await open({
        directory: true,
        multiple: false,
      });
    } else if (props.mode === 'save') {
      // Save-as dialog (for specifying new file path)
      selected = await save({
        filters,
      });
    } else {
      // Open existing file (default behavior)
      selected = await open({
        multiple: false,
        directory: false,
        filters,
      });
    }

    if (selected) {
      // selected is the full file/directory path
      emit('update:modelValue', selected as string);
    }
  } catch (error) {
    console.error('File selection error:', error);
  }
};

const handleClear = () => {
  emit('update:modelValue', '');
};
</script>

<style scoped>
/* Ensure button text doesn't wrap */
button {
  flex-shrink: 0;
}
</style>
