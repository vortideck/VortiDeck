<template lang="pug">
div(v-if="show" class="fixed inset-0 z-50 flex items-center justify-center p-4")
  //- Backdrop
  Transition(name="backdrop" appear)
    div(v-if="show" @click="$emit('close')" class="absolute inset-0 bg-black/50 backdrop-blur-sm")

  //- Modal content
  Transition(name="modal" appear)
    div(v-if="show" class="relative bg-white rounded-2xl shadow-2xl w-full max-w-6xl max-h-[90vh] h-[80vh] flex flex-col")
      //- Header
      div(class="flex items-center justify-between p-6 border-b border-neutral-200")
        div
          h2(class="text-xl font-bold text-neutral-800") Theme Browser
          p(class="text-sm text-neutral-500") Transform your overlay with professional themes
        button(@click="$emit('close')" class="p-2 hover:bg-neutral-100 rounded-lg transition-colors")
          svg(class="w-5 h-5 text-neutral-600" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
      
      //- Content
      div(class="flex-1 flex overflow-hidden")
        //- Categories sidebar
        div(class="w-64 bg-neutral-50 border-r border-neutral-200 p-4")
          h3(class="text-sm font-medium text-neutral-700 mb-3") Categories
          div(class="space-y-1")
            button(
              v-for="category in themeCategories"
              :key="category.id"
              @click="$emit('update:activeCategory', category.id)"
              :class="activeCategory === category.id ? 'bg-white text-blue-600 border-blue-200' : 'text-neutral-700'"
              class="w-full flex items-center justify-between p-2 text-left hover:bg-white rounded-lg border border-transparent transition-all"
            )
              div(class="flex items-center gap-2")
                Icon(:icon="category.icon" class="w-4 h-4")
                span(class="text-sm") {{ category.name }}
              span(class="text-xs text-neutral-500") {{ category.count }}
        
        //- Themes grid
        div(class="flex-1 p-6 overflow-y-auto")
          //- Category title
          div(class="mb-6")
            h3(class="text-lg font-semibold text-neutral-800 mb-2") {{ currentCategoryName }}
            p(class="text-sm text-neutral-500") Complete overlay themes with matching elements and scenes
          
          //- Theme cards
          div(class="grid grid-cols-3 gap-4")
            div(
              v-for="theme in currentThemes"
              :key="theme.id"
              class="bg-white border border-neutral-200 rounded-xl overflow-hidden hover:shadow-lg transition-all group cursor-pointer"
            )
              //- Preview
              div(class="aspect-video bg-gradient-to-br relative overflow-hidden" :style="`background: linear-gradient(135deg, ${theme.colors[0]}, ${theme.colors[1]})`")
                //- Theme preview mockup
                div(class="absolute inset-0 p-4 flex items-end justify-between")
                  div(class="bg-black/60 backdrop-blur-sm rounded-lg p-2 text-white text-xs") 
                    span {{ theme.style }}
                  div(class="flex gap-2")
                    div(v-for="color in theme.colors.slice(0, 3)" :style="`background-color: ${color}`" class="w-4 h-4 rounded border border-white/30")
                
                //- Current theme indicator
                div(v-if="currentTheme && currentTheme.id === theme.id" class="absolute top-2 right-2 bg-green-500 text-white px-2 py-1 rounded text-xs font-medium")
                  span ✓ Active
                
                //- Hover overlay
                div(class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2")
                  button(v-if="!currentTheme || currentTheme.id !== theme.id" @click="$emit('apply-theme', theme)" class="px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-colors")
                    span Apply Theme
                  button(@click="$emit('preview-theme', theme)" class="px-3 py-2 bg-white hover:bg-neutral-100 text-neutral-800 rounded-lg text-sm font-medium transition-colors") Preview
              
              //- Info
              div(class="p-4")
                div(class="flex items-start justify-between mb-2")
                  div
                    h4(class="font-medium text-neutral-800 group-hover:text-blue-600 transition-colors") {{ theme.name }}
                    p(class="text-xs text-neutral-500") {{ theme.description }}
                  span(v-if="theme.isPro" class="px-2 py-1 bg-blue-100 text-blue-600 rounded text-xs font-medium") PRO
                
                div(class="flex items-center gap-3 text-xs text-neutral-500")
                  span {{ theme.elements || 20 }} elements
                  span • 
                  span {{ theme.scenes || 4 }} scenes
      
      //- Footer
      div(class="p-6 border-t border-neutral-200 flex items-center justify-between")
        div(class="flex items-center gap-2")
          svg(class="w-5 h-5 text-amber-500" fill="currentColor" viewBox="0 0 24 24")
            path(d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z")
          span(class="text-sm text-neutral-600") Theme changes apply to all overlay elements instantly
        div(class="flex gap-3")
          button(@click="$emit('close')" class="px-4 py-2 text-neutral-600 hover:text-neutral-800 transition-colors") Close
          button(@click="$emit('browse-store')" class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors") 
            span Browse Store
</template>

<script setup lang="ts">
import { Icon } from "@iconify/vue";

interface Theme {
  id: string
  name: string
  description: string
  colors: string[]
  style: string
  isPro: boolean
  elements?: number
  scenes?: number
}

interface ThemeCategory {
  id: string
  name: string
  icon: string
  count: number
}

defineProps<{
  show: boolean
  themeCategories: ThemeCategory[]
  currentThemes: Theme[]
  activeCategory: string
  currentCategoryName: string
  currentTheme: Theme | null
}>()

defineEmits<{
  close: []
  'update:activeCategory': [category: string]
  'apply-theme': [theme: Theme]
  'preview-theme': [theme: Theme]
  'browse-store': []
}>()
</script>

<style scoped>
.backdrop-enter-active,
.backdrop-leave-active {
  transition: opacity 0.25s ease;
}

.backdrop-enter-from,
.backdrop-leave-to {
  opacity: 0;
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>