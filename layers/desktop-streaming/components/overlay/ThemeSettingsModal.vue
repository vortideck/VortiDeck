<template lang="pug">
OverlayModal(
  :show="show"
  title="Theme Settings"
  subtitle="Customize theme colors and properties"
  modal-class="max-w-4xl max-h-[90vh] h-[70vh]"
  @close="$emit('close')"
)
  div(class="flex-1 flex overflow-hidden")
    //- Theme info sidebar
    div(class="w-80 bg-themeBackground/50 border-r border-themeBorder p-6 overflow-y-auto")
      div(v-if="currentTheme")
        //- Theme preview
        div(class="mb-6 p-4 bg-themeBackground2 rounded-xl border border-themeBorder shadow-2xs")
          div(class="flex items-center justify-between mb-3")
            h3(class="text-base font-bold text-themePrimary") {{ currentTheme.name }}
            span(v-if="currentTheme.isPro" class="px-2 py-0.5 bg-themePrimary/15 text-themePrimary rounded text-[10px] font-bold") PRO
          p(class="text-xs text-themeText2 mb-3") {{ currentTheme.style }} • {{ currentTheme.elements || 20 }} Elements
          
          //- Theme colors preview
          div(class="flex gap-2 mt-3")
            div(v-for="(color, index) in currentTheme.colors" :key="index" :style="`background-color: ${color}`" class="w-8 h-8 rounded-lg border border-themeBorder shadow-2xs")
        
        //- Theme info
        div(class="space-y-4")
          div
            h4(class="text-xs font-bold text-themeText3 uppercase tracking-wider mb-1.5") Description
            p(class="text-xs text-themeText2") {{ currentTheme.description || 'Professional overlay theme with matching elements and animations.' }}
          
          div
            h4(class="text-xs font-bold text-themeText3 uppercase tracking-wider mb-1.5") Category
            span(class="px-2.5 py-1 bg-themeBackground3 text-themeText rounded-full text-xs font-medium") {{ currentTheme.category || 'Gaming' }}
          
          div
            h4(class="text-xs font-bold text-themeText3 uppercase tracking-wider mb-1.5") Compatibility
            div(class="flex gap-2")
              span(class="px-2 py-0.5 bg-green-500/15 text-green-400 border border-green-500/30 rounded text-xs font-medium") OBS Studio
              span(class="px-2 py-0.5 bg-themePrimary/15 text-themePrimary border border-themePrimary/30 rounded text-xs font-medium") Streamlabs
    
    //- Settings panel
    div(class="flex-1 p-6 overflow-y-auto bg-themeBackground")
      div(class="space-y-6")
        //- Color customization
        div
          h3(class="text-base font-bold text-themeText mb-3") Color Palette
          div(class="grid grid-cols-2 gap-4")
            div(v-for="(color, index) in editableColors" :key="index")
              label(class="block text-xs font-semibold text-themeText2 mb-1.5") {{ color.name }}
              div(class="flex items-center gap-3")
                input(
                  type="color"
                  :value="color.value"
                  @input="updateColor(index, $event.target.value)"
                  class="w-10 h-10 border border-themeBorder rounded-lg cursor-pointer bg-themeBackground2 p-0.5"
                )
                input(
                  type="text"
                  :value="color.value"
                  @input="updateColor(index, $event.target.value)"
                  class="flex-1 px-3 py-2 bg-themeBackground2 border border-themeBorder text-themeText rounded-lg text-xs font-mono focus:outline-none focus:border-themePrimary"
                )
                button(@click="resetColor(index)" class="p-2 text-themeText3 hover:text-themeText hover:bg-themeBackground3 rounded-lg transition-colors cursor-pointer" title="Reset color")
                  svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                    path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15")
        
        //- Theme properties
        div(class="border-t border-themeBorder pt-4")
          h3(class="text-base font-bold text-themeText mb-3") Theme Properties
          div(class="space-y-4")
            div
              label(class="block text-xs font-semibold text-themeText2 mb-1.5") Border Radius
              input(
                type="range"
                min="0"
                max="24"
                :value="themeSettings.borderRadius"
                @input="updateSetting('borderRadius', $event.target.value)"
                class="w-full accent-themePrimary"
              )
              span(class="text-xs font-mono text-themeText3") {{ themeSettings.borderRadius }}px
            
            div
              label(class="block text-xs font-semibold text-themeText2 mb-1.5") Shadow Intensity
              input(
                type="range"
                min="0"
                max="100"
                :value="themeSettings.shadowIntensity"
                @input="updateSetting('shadowIntensity', $event.target.value)"
                class="w-full accent-themePrimary"
              )
              span(class="text-xs font-mono text-themeText3") {{ themeSettings.shadowIntensity }}%
            
            div
              label(class="flex items-center gap-2 cursor-pointer")
                input(
                  type="checkbox"
                  :checked="themeSettings.animations"
                  @change="updateSetting('animations', $event.target.checked)"
                  class="rounded border-themeBorder text-themePrimary focus:ring-themePrimary"
                )
                span(class="text-xs font-semibold text-themeText") Enable Animations
            
            div
              label(class="flex items-center gap-2 cursor-pointer")
                input(
                  type="checkbox"
                  :checked="themeSettings.gradients"
                  @change="updateSetting('gradients', $event.target.checked)"
                  class="rounded border-themeBorder text-themePrimary focus:ring-themePrimary"
                )
                span(class="text-xs font-semibold text-themeText") Use Gradient Backgrounds
        
        //- Preview
        div(class="border-t border-themeBorder pt-4")
          h3(class="text-base font-bold text-themeText mb-3") Live Preview
          div(class="border border-themeBorder rounded-xl p-4 bg-themeBackground2")
            div(class="grid grid-cols-2 gap-4")
              //- Sample elements with theme applied
              div(class="p-3 rounded-lg shadow-sm" :style="getPreviewStyle('primary')")
                h4(class="font-medium text-white mb-1 text-xs") Chat Widget
                p(class="text-white/80 text-[11px]") Live chat messages
              
              div(class="p-3 rounded-lg shadow-sm" :style="getPreviewStyle('secondary')")
                h4(class="font-medium text-white mb-1 text-xs") Alert Box
                p(class="text-white/80 text-[11px]") New follower alert

  template(#footer)
    div(class="flex items-center justify-between")
      div(class="flex items-center gap-3")
        button(@click="resetTheme" class="px-4 py-2 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-xl transition-colors cursor-pointer text-xs font-medium") Reset to Default
        button(@click="$emit('export-theme')" class="px-4 py-2 text-themePrimary hover:bg-themePrimary/10 rounded-xl transition-colors cursor-pointer text-xs font-medium") Export Theme
      div(class="flex gap-3")
        button(@click="$emit('close')" class="px-4 py-2 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-xl transition-colors cursor-pointer text-xs font-medium") Cancel
        button(@click="applyChanges" class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors shadow-2xs cursor-pointer") Apply Changes
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

interface Theme {
  id: string
  name: string
  description?: string
  style: string
  category?: string
  colors: string[]
  elements?: number
  isPro?: boolean
}

interface ThemeSettings {
  borderRadius: number
  shadowIntensity: number
  animations: boolean
  gradients: boolean
}

const props = withDefaults(defineProps<{
  show?: boolean
  currentTheme?: Theme | null
}>(), {
  show: false,
  currentTheme: null
})

const emit = defineEmits<{
  close: []
  'apply-theme': [settings: any]
  'export-theme': []
}>()

const editableColors = ref([
  { name: 'Primary', value: '#8B5CF6' },
  { name: 'Secondary', value: '#EC4899' },
  { name: 'Accent', value: '#06B6D4' },
  { name: 'Background', value: '#1F2937' },
  { name: 'Text', value: '#FFFFFF' },
  { name: 'Border', value: '#374151' }
])

const themeSettings = ref<ThemeSettings>({
  borderRadius: 8,
  shadowIntensity: 50,
  animations: true,
  gradients: true
})

const updateColor = (index: number, value: string) => {
  editableColors.value[index].value = value
}

const resetColor = (index: number) => {
  const defaults = ['#8B5CF6', '#EC4899', '#06B6D4', '#1F2937', '#FFFFFF', '#374151']
  editableColors.value[index].value = defaults[index]
}

const updateSetting = (key: keyof ThemeSettings, value: any) => {
  themeSettings.value[key] = value
}

const resetTheme = () => {
  editableColors.value.forEach((color, index) => resetColor(index))
  themeSettings.value = {
    borderRadius: 8,
    shadowIntensity: 50,
    animations: true,
    gradients: true
  }
}

const getPreviewStyle = (type: 'primary' | 'secondary') => {
  const colorIndex = type === 'primary' ? 0 : 1
  const backgroundColor = editableColors.value[colorIndex].value
  
  return {
    backgroundColor,
    borderRadius: `${themeSettings.value.borderRadius}px`,
    boxShadow: `0 4px 8px rgba(0,0,0,${themeSettings.value.shadowIntensity / 100 * 0.3})`
  }
}

const applyChanges = () => {
  emit('apply-theme', {
    colors: editableColors.value.map(c => c.value),
    settings: themeSettings.value
  })
  emit('close')
}
</script>