<template lang="pug">
div(class="plugin-card-content")
  div(class="flex p-4 gap-4 border-b border-themeBorder")
    img(:src="plugin.icon" :alt="plugin.name" class="w-12 h-12 rounded-lg object-cover")
    div(class="flex-1")
      h3(class="text-lg font-semibold mb-2 text-themeText") {{ plugin.name }}
      div(class="flex gap-2 flex-wrap")
        span(class="text-xs px-2 py-1 rounded bg-themeBackground3 text-themeText3") {{ plugin.category }}
        span(v-if="plugin.installed" class="text-xs px-2 py-1 rounded bg-green-600 text-white") ✓ Enabled
        span(v-else-if="plugin.price > 0" class="text-xs px-2 py-1 rounded bg-purple-800 text-white") ${{ plugin.price.toFixed(2) }}
        span(v-else class="text-xs px-2 py-1 rounded bg-green-700 text-white") Free
  
  p(class="p-4 flex-1 text-themeText3 text-sm leading-relaxed") {{ plugin.description }}
  
  div(class="flex p-4 gap-2 border-t border-themeBorder")
    button(
      v-if="plugin.installed"
      @click="$emit('uninstall', plugin)"
      class="flex-1 py-2 px-4 rounded-lg bg-red-500 text-white font-medium text-sm hover:bg-red-600 transition-colors"
    )
      span Disable
    button(
      v-else
      @click="$emit('install', plugin)"
      class="flex-2 py-2 px-4 rounded-lg bg-themeSecondary text-white font-medium text-sm hover:bg-opacity-90 transition-colors"
    )
      span {{ plugin.price > 0 ? 'Buy & Install' : 'Install' }}
    
    button(
      @click="$emit('details', plugin)"
      class="flex-1 py-2 px-4 rounded-lg bg-themeBackground3 text-themeText font-medium text-sm hover:bg-opacity-90 transition-colors"
    )
      span Details
</template>

<script setup lang="ts">
interface Plugin {
  id: string;
  name: string;
  description: string;
  fullDescription?: string;
  icon: string;
  category: string;
  price: number;
  rating: number;
  reviews: number;
  downloads: number;
  installed: boolean;
  version: string;
  author: string;
  lastUpdated: string;
  screenshots?: string[];
}

defineProps<{
  plugin: Plugin;
}>();

defineEmits<{
  install: [plugin: Plugin];
  uninstall: [plugin: Plugin];
  details: [plugin: Plugin];
}>();
</script>
