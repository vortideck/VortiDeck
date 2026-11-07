<template>
  <div class="flex flex-col gap-2 w-full relative" ref="containerRef">
    <!-- Selected tags display -->
    <div v-if="selectedItems.length > 0" class="flex flex-wrap gap-1.5">
      <div
        v-for="item in selectedItems"
        :key="item"
        class="inline-flex items-center gap-1 px-2 py-0.5 text-xs bg-themePrimary/15 text-themePrimary border border-themePrimary/25 rounded-lg group"
      >
        <span class="capitalize">{{ item }}</span>
        <button
          @click.stop="removeItem(item)"
          @mousedown.stop
          class="hover:bg-themePrimary/20 rounded-sm transition-colors cursor-pointer"
          :title="'Remove ' + item"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Custom dropdown button -->
    <button
      ref="buttonRef"
      @click.stop="toggleDropdown"
      @mousedown.stop
      class="w-full px-2.5 py-1.5 text-xs bg-themeBackground text-themeText border border-themeBorder rounded-xl flex items-center justify-between hover:bg-themeBackground3 hover:border-themePrimary/40 transition-colors cursor-pointer shadow-2xs"
      :class="{ 'border-themePrimary ring-1 ring-themePrimary/20': isOpen }"
    >
      <span class="text-left truncate">
        {{ selectedItems.length === 0 ? 'Select items...' : `${selectedItems.length} selected` }}
      </span>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="w-3.5 h-3.5 text-themeText3 transition-transform flex-shrink-0"
        :class="{ 'rotate-180': isOpen }"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
      </svg>
    </button>

    <!-- Custom dropdown menu (teleported to body) -->
    <Teleport to="body">
      <div
        v-if="isOpen"
        ref="dropdownRef"
        class="fixed bg-themeBackground2 border border-themeBorder rounded-2xl shadow-xl z-[9999] max-h-64 overflow-hidden flex flex-col"
        :style="dropdownStyle"
        @click.stop
        @mousedown.stop
      >
        <!-- Search input -->
        <div class="p-2 border-b border-themeBorder">
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            type="text"
            placeholder="Search..."
            class="w-full px-2.5 py-1.5 text-xs bg-themeBackground text-themeText border border-themeBorder rounded-xl placeholder-themeText3 focus:outline-none focus:border-themePrimary focus:ring-1 focus:ring-themePrimary/20 transition-all"
            @click.stop
            @mousedown.stop
          />
        </div>

        <!-- Options list with checkboxes -->
        <div class="overflow-y-auto flex-1 p-1">
          <div v-if="filteredOptions.length === 0" class="px-3 py-2 text-xs text-themeText3 italic">
            No options found
          </div>
          <label
            v-for="option in filteredOptions"
            :key="option"
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg hover:bg-themeBackground3 cursor-pointer transition-colors"
            @click.stop
          >
            <input
              type="checkbox"
              :checked="selectedItems.includes(option)"
              @change="toggleItem(option)"
              @click.stop
              class="w-3.5 h-3.5 rounded border-themeBorder text-themePrimary focus:ring-themePrimary focus:ring-offset-0 bg-themeBackground"
            />
            <span class="text-xs text-themeText capitalize flex-1">{{ option }}</span>
          </label>
        </div>

        <!-- Action buttons -->
        <div class="flex gap-2 p-2 border-t border-themeBorder">
          <button
            @click.stop="selectAll"
            @mousedown.stop
            class="flex-1 px-2.5 py-1 text-xs bg-themeButton hover:bg-themeButtonHover text-white rounded-lg transition-colors cursor-pointer shadow-xs font-medium"
          >
            Select All
          </button>
          <button
            @click.stop="clearAll"
            @mousedown.stop
            class="flex-1 px-2.5 py-1 text-xs bg-themeBackground hover:bg-themeBackground3 border border-themeBorder text-themeText rounded-lg transition-colors cursor-pointer shadow-2xs font-medium"
          >
            Clear All
          </button>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, nextTick, onMounted, onUnmounted } from 'vue';

interface Props {
  modelValue: string[];
  options: string[];
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: () => [],
  options: () => [],
});

const emit = defineEmits<{
  'update:modelValue': [value: string[]];
}>();

const isOpen = ref(false);
const searchQuery = ref('');
const containerRef = ref<HTMLElement>();
const searchInputRef = ref<HTMLInputElement>();
const buttonRef = ref<HTMLButtonElement>();
const dropdownRef = ref<HTMLElement>();
const dropdownPosition = ref({ top: '0px', left: '0px', width: '0px' });
const canvasScale = ref(1);

// Selected items (from modelValue)
const selectedItems = computed(() => {
  return Array.isArray(props.modelValue) ? props.modelValue : [];
});

// Filtered options based on search
const filteredOptions = computed(() => {
  if (!searchQuery.value) {
    return props.options;
  }
  const query = searchQuery.value.toLowerCase();

  // Separate prefix matches and contains matches
  const prefixMatches: string[] = [];
  const containsMatches: string[] = [];

  props.options.forEach(opt => {
    const optLower = opt.toLowerCase();
    if (optLower.startsWith(query)) {
      prefixMatches.push(opt);
    } else if (optLower.includes(query)) {
      containsMatches.push(opt);
    }
  });

  // Prioritize prefix matches, then contains matches
  return [...prefixMatches, ...containsMatches];
});

// Update dropdown position
const updateDropdownPosition = () => {
  if (!buttonRef.value) return;

  const rect = buttonRef.value.getBoundingClientRect();

  // Calculate scale by comparing screen size vs CSS size
  // This accounts for all parent transforms (canvas zoom)
  const scale = buttonRef.value.offsetWidth > 0
    ? rect.width / buttonRef.value.offsetWidth
    : 1;

  canvasScale.value = scale;

  // Compensate width for scale so it matches button width after scaling
  const compensatedWidth = scale > 0 ? rect.width / scale : rect.width;

  dropdownPosition.value = {
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    width: `${compensatedWidth}px`,
  };
};

// Calculate dropdown position based on button with canvas scale applied
const dropdownStyle = computed(() => ({
  ...dropdownPosition.value,
  transform: `scale(${canvasScale.value})`,
  transformOrigin: 'top left',
}));

// Track position updates
let positionUpdateFrame: number | null = null;

const startPositionTracking = () => {
  const updateLoop = () => {
    if (isOpen.value) {
      updateDropdownPosition();
      positionUpdateFrame = requestAnimationFrame(updateLoop);
    }
  };
  updateLoop();
};

const stopPositionTracking = () => {
  if (positionUpdateFrame !== null) {
    cancelAnimationFrame(positionUpdateFrame);
    positionUpdateFrame = null;
  }
};

const toggleDropdown = () => {
  isOpen.value = !isOpen.value;
  if (isOpen.value) {
    // Update position and start tracking
    updateDropdownPosition();
    startPositionTracking();

    // Focus search input when opening
    nextTick(() => {
      searchInputRef.value?.focus();
    });
  } else {
    // Stop tracking and clear search when closing
    stopPositionTracking();
    searchQuery.value = '';
  }
};

const toggleItem = (item: string) => {
  if (selectedItems.value.includes(item)) {
    emit('update:modelValue', selectedItems.value.filter(i => i !== item));
  } else {
    emit('update:modelValue', [...selectedItems.value, item]);
  }
};

const removeItem = (item: string) => {
  emit('update:modelValue', selectedItems.value.filter(i => i !== item));
};

const selectAll = () => {
  emit('update:modelValue', [...props.options]);
  searchQuery.value = '';
};

const clearAll = () => {
  emit('update:modelValue', []);
  searchQuery.value = '';
};

// Close dropdown when clicking outside
const handleClickOutside = (event: MouseEvent) => {
  if (!isOpen.value) return;

  const target = event.target as Node;

  // Check if click is inside container (button) or dropdown
  const isInsideContainer = containerRef.value?.contains(target);
  const isInsideDropdown = dropdownRef.value?.contains(target);

  // Close if click is outside both
  if (!isInsideContainer && !isInsideDropdown) {
    isOpen.value = false;
    searchQuery.value = '';
  }
};

onMounted(() => {
  // Use capture phase to catch events before they're stopped by other elements
  document.addEventListener('mousedown', handleClickOutside, true);
});

onUnmounted(() => {
  document.removeEventListener('mousedown', handleClickOutside, true);
  stopPositionTracking();
});
</script>

<style scoped>
/* Custom scrollbar for options list */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: rgba(55, 65, 81, 0.3);
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: rgba(107, 114, 128, 0.5);
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: rgba(107, 114, 128, 0.7);
}
</style>
