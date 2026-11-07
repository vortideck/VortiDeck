<template lang="pug">
OverlayModal(
  :show="show"
  title="Template Library"
  subtitle="Quick-start with professional overlay designs"
  @close="$emit('close')"
)
  div(class="flex-1 flex overflow-hidden")
    //- Categories sidebar
    div(class="w-64 bg-themeBackground/50 border-r border-themeBorder p-4 flex flex-col overflow-y-auto")
      h3(class="text-xs font-bold text-themeText3 uppercase tracking-wider mb-3") Categories
      div(class="space-y-1")
        button(
          v-for="category in templateCategories"
          :key="category.id"
          @click="$emit('update:activeCategory', category.id)"
          :class="activeCategory === category.id ? 'bg-themePrimary/15 text-themePrimary border border-themePrimary/30 shadow-2xs font-semibold' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground3 border border-transparent'"
          class="w-full flex items-center justify-between p-2.5 text-left rounded-xl transition-all text-xs cursor-pointer"
        )
          div(class="flex items-center gap-2")
            Icon(:icon="category.icon" class="w-4 h-4")
            span(class="text-xs") {{ category.name }}
          span(class="text-[11px] text-themeText3 px-2 py-0.5 rounded-full bg-themeBackground3 font-medium") {{ category.count }}
    
    //- Templates grid
    div(class="flex-1 p-6 overflow-y-auto bg-themeBackground")
      //- Category title
      div(class="mb-6")
        h3(class="text-base font-bold text-themeText mb-1") {{ currentCategoryName }}
        p(class="text-xs text-themeText2") Pre-designed overlay scenes ready to customize
      
      //- Template cards
      div(class="grid grid-cols-3 gap-4")
        div(
          v-for="template in currentTemplates"
          :key="template.id"
          class="bg-themeBackground2 border border-themeBorder hover:border-themePrimary/40 rounded-xl overflow-hidden hover:shadow-lg transition-all group cursor-pointer shadow-2xs"
        )
          //- Preview
          div(class="aspect-video bg-themeBackground3/60 relative overflow-hidden")
            //- Template preview mockup
            div(class="absolute inset-0 p-4")
              div(v-if="template.type === 'gaming'" class="h-full flex items-end justify-between")
                div(class="bg-black/80 backdrop-blur-sm rounded-lg p-2 text-white text-xs") 
                  span Webcam
                div(class="bg-themeButton backdrop-blur-sm rounded-lg px-3 py-1 text-white text-xs")
                  span Followers: 1.2K
              div(v-else-if="template.type === 'just-chatting'" class="h-full flex flex-col justify-between")
                div(class="bg-gradient-to-r from-blue-500 to-pink-500 rounded-lg p-2 text-white text-xs text-center")
                  span Just Chatting
                div(class="flex gap-2")
                  div(class="bg-white/90 backdrop-blur-sm rounded p-1 text-xs flex-1 text-neutral-800") Chat
                  div(class="bg-white/90 backdrop-blur-sm rounded p-1 text-xs text-neutral-800") Cam
              div(v-else class="h-full flex items-center justify-center")
                Icon(:icon="template.icon" class="w-10 h-10 text-themeText3")
            
            //- Saved template indicator
            div(v-if="savedTemplates.includes(template.id)" class="absolute top-2 right-2 bg-amber-500 text-white px-2 py-1 rounded text-xs font-medium")
              Icon(icon="heroicons:bookmark-20-solid" class="w-3.5 h-3.5 inline")
              span  Saved
            
            //- Hover overlay
            div(class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2")
              button(v-if="!savedTemplates.includes(template.id)" @click="$emit('save-template', template)" class="px-3 py-2 bg-amber-500 hover:bg-amber-600 text-white rounded-lg text-xs font-semibold transition-colors flex items-center gap-1 cursor-pointer")
                svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
                span Save
              button(v-else @click="$emit('use-template', template)" class="px-3 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg text-xs font-semibold transition-colors flex items-center gap-1 cursor-pointer")
                svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z")
                span Use
              button(@click="$emit('preview-template', template)" class="px-3 py-2 bg-themeBackground2 hover:bg-themeBackground3 text-themeText border border-themeBorder rounded-lg text-xs font-semibold transition-colors cursor-pointer") Preview
          
          //- Info
          div(class="p-3.5")
            div(class="flex items-start justify-between mb-2")
              div
                h4(class="font-semibold text-sm text-themeText group-hover:text-themePrimary transition-colors") {{ template.name }}
                p(class="text-[11px] text-themeText2") {{ template.description }}
              span(v-if="template.isPro" class="px-2 py-0.5 bg-themePrimary/15 text-themePrimary rounded text-[10px] font-bold") PRO
            
            div(class="flex items-center gap-3 text-xs text-themeText3")
              span {{ template.elements }} elements
              span • 
              span {{ template.animations ? 'Animated' : 'Static' }}

  template(#footer)
    div(class="flex items-center justify-between")
      div(class="flex items-center gap-2")
        svg(class="w-5 h-5 text-amber-500" fill="currentColor" viewBox="0 0 24 24")
          path(d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z")
        span(class="text-xs text-themeText2") {{ savedTemplates.length }} templates in your library
      div(class="flex gap-3")
        button(@click="$emit('close')" class="px-4 py-2 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-xl transition-colors cursor-pointer text-xs font-medium") Close
        button(@click="$emit('browse-marketplace')" class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors shadow-2xs cursor-pointer") 
          span Browse Marketplace
</template>

<script setup lang="ts">
import { Icon } from "@iconify/vue";

interface Template {
  id: string
  name: string
  description: string
  type: string
  icon?: string
  elements: number
  animations: boolean
  isPro?: boolean
}

interface TemplateCategory {
  id: string
  name: string
  icon: string
  count: number
}

const props = withDefaults(defineProps<{
  show?: boolean
  activeCategory?: string
  templateCategories?: TemplateCategory[]
  currentTemplates?: Template[]
  savedTemplates?: string[]
  currentCategoryName?: string
}>(), {
  show: false,
  activeCategory: 'gaming',
  templateCategories: () => [],
  currentTemplates: () => [],
  savedTemplates: () => [],
  currentCategoryName: 'Gaming Templates'
})

const emit = defineEmits<{
  close: []
  'update:activeCategory': [id: string]
  'save-template': [template: Template]
  'use-template': [template: Template]
  'preview-template': [template: Template]
  'browse-marketplace': []
}>()
</script>