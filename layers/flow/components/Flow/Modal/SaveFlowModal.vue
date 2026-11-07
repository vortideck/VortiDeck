<template lang="pug">
div(
  v-if="show"
  class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-[10000]"
  @mousedown="handleBackdropMouseDown"
  @mouseup="handleBackdropMouseUp"
)
  div(
    class="bg-themeBackground2 rounded-2xl p-6 max-w-md w-full mx-4 border border-themeBorder shadow-2xl"
    @click.stop=""
    @mousedown.stop=""
    @mouseup.stop=""
  )
    div(class="flex items-center gap-3 mb-4")
      div(class="w-10 h-10 bg-emerald-500/15 border border-emerald-500/30 rounded-xl flex items-center justify-center")
        span(class="text-emerald-500 text-lg") 💾
      div
        h3(class="text-base font-bold text-themeText") {{ isNewFlow ? 'Save New Flow' : 'Save Flow' }}
        p(class="text-xs text-themeText3") {{ isNewFlow ? 'Enter a name and description for your new flow' : 'Update flow name and description' }}
    
    div(class="space-y-4 mb-6")
      div
        label(class="block text-xs font-semibold text-themeText mb-1.5") Flow Name
        input(
          v-model="localFlowName"
          type="text"
          placeholder="Enter flow name..."
          class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-xl text-themeText text-sm placeholder-themeText3 focus:outline-none focus:border-themePrimary focus:ring-1 focus:ring-themePrimary/20"
          @keydown.enter="saveFlow"
          ref="nameInput"
        )
      
      div
        label(class="block text-xs font-semibold text-themeText mb-1.5") Description (Optional)
        textarea(
          v-model="localFlowDescription"
          placeholder="Enter flow description..."
          rows="3"
          class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-xl text-themeText text-sm placeholder-themeText3 focus:outline-none focus:border-themePrimary focus:ring-1 focus:ring-themePrimary/20 resize-none"
        )
    
    div(class="flex justify-end gap-2.5")
      button(
        class="px-4 py-2 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder rounded-xl text-xs font-semibold text-themeText transition-colors cursor-pointer shadow-2xs"
        @click="$emit('cancel')"
      ) Cancel
      button(
        class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors cursor-pointer shadow-xs disabled:opacity-50 disabled:cursor-not-allowed"
        :disabled="!localFlowName.trim()"
        @click="saveFlow"
      ) {{ isNewFlow ? 'Save Flow' : 'Update Flow' }}
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';

// Props
interface Props {
  show: boolean;
  isNewFlow?: boolean;
  currentName?: string;
  currentDescription?: string;
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  isNewFlow: true,
  currentName: '',
  currentDescription: '',
});

// Emits
const emit = defineEmits<{
  save: [name: string, description: string];
  cancel: [];
}>();

// Local state
const localFlowName = ref('');
const localFlowDescription = ref('');
const backdropMouseDownTarget = ref<EventTarget | null>(null);
const nameInput = ref<HTMLInputElement | null>(null);

// Watch for prop changes to update local state
watch(() => props.show, (newShow) => {
  if (newShow) {
    localFlowName.value = props.currentName || '';
    localFlowDescription.value = props.currentDescription || '';
    // Reset backdrop click tracking
    backdropMouseDownTarget.value = null;
    
    // Focus the name input when modal opens
    nextTick(() => {
      if (nameInput.value) {
        nameInput.value.focus();
        nameInput.value.select();
      }
    });
  }
}, { immediate: true });

// Methods
const saveFlow = () => {
  if (localFlowName.value.trim()) {
    emit('save', localFlowName.value.trim(), localFlowDescription.value.trim());
  }
};

// Handle backdrop clicks properly to avoid closing during text selection
const handleBackdropMouseDown = (event: MouseEvent) => {
  // Only track mousedown if it's directly on the backdrop (not on modal content)
  if (event.target === event.currentTarget) {
    backdropMouseDownTarget.value = event.target;
  }
};

const handleBackdropMouseUp = (event: MouseEvent) => {
  // Only close if both mousedown and mouseup occurred on the same backdrop element
  // This prevents closing during text selection that ends outside the modal
  if (
    event.target === event.currentTarget && 
    backdropMouseDownTarget.value === event.target
  ) {
    emit('cancel');
  }
  
  // Reset tracking
  backdropMouseDownTarget.value = null;
};
</script>

<style lang="scss"></style> 