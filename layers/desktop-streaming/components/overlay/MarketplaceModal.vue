<template lang="pug">
OverlayModal(
  :show="show"
  title="Overlay Component Library"
  subtitle="Add components from installed and built-in packages"
  @close="$emit('close')"
)
  div(class="flex-1 h-full flex overflow-hidden")
    //- Categories sidebar
    div(class="w-64 h-full bg-themeBackground/50 border-r border-themeBorder p-4 flex flex-col overflow-y-auto")
      h3(class="text-xs font-bold text-themeText3 uppercase tracking-wider mb-3") Categories
      div(class="space-y-1")
        button(
          v-for="category in marketplaceCategories"
          :key="category.id"
          @click="$emit('update:activeCategory', category.id)"
          :class="activeCategory === category.id ? 'bg-themePrimary/15 text-themePrimary border border-themePrimary/30 shadow-2xs font-semibold' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground3 border border-transparent'"
          class="w-full flex items-center justify-between p-2.5 text-left rounded-xl transition-all text-xs cursor-pointer"
        )
          span {{ category.name }}
          span(class="text-[11px] text-themeText3 px-2 py-0.5 rounded-full bg-themeBackground3 font-medium") {{ category.count }}

    //- Items grid
    div(class="flex-1 p-6 overflow-y-auto bg-themeBackground")
      //- Search
      div(class="mb-6")
        div(class="relative")
          input(
            type="text"
            placeholder="Search installed overlay components..."
            :value="searchQuery"
            @input="$emit('update:searchQuery', $event.target.value)"
            class="w-full pl-10 pr-4 py-2.5 bg-themeBackground2 border border-themeBorder rounded-xl text-sm text-themeText placeholder-themeText3 focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary shadow-2xs"
          )
          svg(class="absolute left-3.5 top-3 w-4 h-4 text-themeText3" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z")

      //- Items
      div(v-if="marketplaceItems.length === 0" class="flex flex-col items-center justify-center py-16 text-themeText3 text-center")
        div(class="w-12 h-12 rounded-full bg-themeBackground3 flex items-center justify-center mb-3 text-themeText3")
          svg(class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4")
        p(class="text-sm font-medium text-themeText mb-1") No marketplace items found
        p(class="text-xs text-themeText3 mb-4") Try another category, or install a widget package from the package marketplace.
        button(
          @click="$emit('open-marketplace')"
          class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors shadow-2xs cursor-pointer"
        ) Open Package Marketplace

      div(v-else class="grid grid-cols-2 xl:grid-cols-3 gap-4")
        div(
          v-for="item in marketplaceItems"
          :key="item.id"
          class="bg-themeBackground2 border border-themeBorder hover:border-themePrimary/40 rounded-xl overflow-hidden hover:shadow-lg transition-all group flex flex-col shadow-2xs"
        )
          //- Image
          div(class="aspect-video bg-themeBackground3/60 relative overflow-hidden flex items-center justify-center")
            Icon(:icon="item.icon || 'heroicons:sparkles'" class="h-9 w-9 text-themePrimary opacity-90 transition-transform duration-200 group-hover:scale-110")
            div(class="absolute top-2 right-2 px-2 py-0.5 bg-themeBackground/90 backdrop-blur-sm text-themeText border border-themeBorder rounded text-[11px] font-semibold") {{ item.trust === 'builtin' ? 'Built in' : 'Installed' }}

          //- Content
          div(class="p-3.5 flex-1 flex flex-col")
            h4(class="font-semibold text-sm text-themeText mb-0.5 group-hover:text-themePrimary transition-colors line-clamp-1") {{ item.name }}
            p(class="text-[11px] text-themeText2") by {{ item.author }}
            p(class="text-[11px] text-themeText3 mt-2 mb-3 line-clamp-2") {{ item.description }}

            div(class="flex items-center justify-between mt-auto pt-2 border-t border-themeBorder")
              span(class="text-[11px] text-themeText3 font-medium") {{ item.versionLabel }}
              button(
                @click.stop="$emit('add-item', item)"
                class="px-3 py-1 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold transition-colors shadow-2xs cursor-pointer"
              ) {{ item.requiresConsent ? 'Review access' : 'Add to canvas' }}

  template(#footer)
    div(class="flex items-center justify-between")
      p(class="text-sm text-themeText2") Installed components can be placed more than once.
      div(class="flex gap-3")
        button(@click="$emit('close')" class="px-4 py-2 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-xl transition-colors cursor-pointer text-xs font-medium") Close
        button(@click="$emit('open-marketplace')" class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors shadow-2xs cursor-pointer") Browse Package Marketplace
</template>

<script setup lang="ts">
import { Icon } from '@iconify/vue'

interface MarketplaceItem {
  id: string
  name: string
  author: string
  description: string
  icon?: string
  versionLabel: string
  trust: string
  requiresConsent: boolean
  sourceItem: unknown
}

interface MarketplaceCategory {
  id: string
  name: string
  count: number
}

const props = withDefaults(defineProps<{
  show?: boolean
  activeCategory?: string
  searchQuery?: string
  marketplaceCategories?: MarketplaceCategory[]
  marketplaceItems?: MarketplaceItem[]
}>(), {
  show: false,
  activeCategory: 'all',
  searchQuery: '',
  marketplaceCategories: () => [],
  marketplaceItems: () => []
})

const emit = defineEmits<{
  close: []
  'update:activeCategory': [id: string]
  'update:searchQuery': [query: string]
  'add-item': [item: MarketplaceItem]
  'open-marketplace': []
}>()
</script>
