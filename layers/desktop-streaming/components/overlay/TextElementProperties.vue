<template lang="pug">
div(class="space-y-4")
  //- Text Content
  div
    h3(class="text-sm font-medium text-neutral-700 mb-3") Text Content
    div(class="space-y-3")
      div
        label(class="text-xs text-neutral-500 block mb-1") Text Content
        textarea(
          :value="element.content || 'Sample Text'"
          @input="$emit('update-property', { property: 'content', value: $event.target.value })"
          rows="3"
          class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm text-neutral-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
          placeholder="Enter your text here..."
        )
  
  //- Typography
  div
    h3(class="text-sm font-medium text-neutral-700 mb-3") Typography
    div(class="space-y-3")
      div
        label(class="text-xs text-neutral-500 block mb-1") Font Family
        select(
          :value="element.style?.fontFamily || 'Arial, sans-serif'"
          @change="$emit('update-style', { property: 'fontFamily', value: $event.target.value })"
          class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm text-neutral-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        )
          option(value="Arial, sans-serif") Arial
          option(value="Helvetica, sans-serif") Helvetica
          option(value="Times New Roman, serif") Times New Roman
          option(value="Georgia, serif") Georgia
          option(value="Verdana, sans-serif") Verdana
          option(value="Courier New, monospace") Courier New
          option(value="Impact, sans-serif") Impact
          option(value="Comic Sans MS, cursive") Comic Sans MS
          option(value="'Orbitron', monospace") Orbitron (Gaming)
          option(value="'Press Start 2P', monospace") Press Start 2P (Retro)
          option(value="'Rajdhani', sans-serif") Rajdhani (Modern)
      
      div(class="grid grid-cols-2 gap-3")
        div
          label(class="text-xs text-neutral-500 block mb-1") Font Size
          input(
            type="number"
            :value="parseInt(element.style?.fontSize) || 16"
            @input="$emit('update-style', { property: 'fontSize', value: $event.target.value + 'px' })"
            min="8"
            max="200"
            class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm text-neutral-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          )
        div
          label(class="text-xs text-neutral-500 block mb-1") Line Height
          input(
            type="number"
            :value="parseFloat(element.style?.lineHeight) || 1.2"
            @input="$emit('update-style', { property: 'lineHeight', value: $event.target.value })"
            min="0.5"
            max="3"
            step="0.1"
            class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm text-neutral-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          )
      
      div
        label(class="text-xs text-neutral-500 block mb-1") Font Weight
        select(
          :value="element.style?.fontWeight || 'normal'"
          @change="$emit('update-style', { property: 'fontWeight', value: $event.target.value })"
          class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm text-neutral-900 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        )
          option(value="normal") Normal
          option(value="bold") Bold
          option(value="100") Thin (100)
          option(value="200") Extra Light (200)
          option(value="300") Light (300)
          option(value="400") Regular (400)
          option(value="500") Medium (500)
          option(value="600") Semi Bold (600)
          option(value="700") Bold (700)
          option(value="800") Extra Bold (800)
          option(value="900") Black (900)
      
      div
        label(class="text-xs text-neutral-500 block mb-1") Text Color
        input(
          type="color"
          :value="element.style?.color || '#000000'"
          @input="$emit('update-style', { property: 'color', value: $event.target.value })"
          class="w-full h-10 border border-neutral-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
        )
      
      div
        label(class="text-xs text-neutral-500 block mb-1") Text Align
        div(class="grid grid-cols-3 gap-2")
          button(
            @click="$emit('update-style', { property: 'textAlign', value: 'left' })"
            :class="element.style?.textAlign === 'left' ? 'bg-blue-100 border-blue-300 text-blue-700' : 'bg-neutral-50 border-neutral-200 text-neutral-700'"
            class="px-3 py-2 border rounded-lg text-xs font-medium transition-colors"
          ) Left
          button(
            @click="$emit('update-style', { property: 'textAlign', value: 'center' })"
            :class="element.style?.textAlign === 'center' ? 'bg-blue-100 border-blue-300 text-blue-700' : 'bg-neutral-50 border-neutral-200 text-neutral-700'"
            class="px-3 py-2 border rounded-lg text-xs font-medium transition-colors"
          ) Center
          button(
            @click="$emit('update-style', { property: 'textAlign', value: 'right' })"
            :class="element.style?.textAlign === 'right' ? 'bg-blue-100 border-blue-300 text-blue-700' : 'bg-neutral-50 border-neutral-200 text-neutral-700'"
            class="px-3 py-2 border rounded-lg text-xs font-medium transition-colors"
          ) Right
  
  //- Text Effects
  div
    h3(class="text-sm font-medium text-neutral-700 mb-3") Text Effects
    div(class="space-y-3")
      div(class="flex items-center justify-between")
        label(class="text-xs text-neutral-500") Text Shadow
        input(
          type="checkbox"
          :checked="element.style?.textShadow && element.style.textShadow !== 'none'"
          @change="$emit('update-style', { property: 'textShadow', value: $event.target.checked ? '2px 2px 4px rgba(0,0,0,0.5)' : 'none' })"
          class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
        )
      
      div(class="flex items-center justify-between")
        label(class="text-xs text-neutral-500") Text Stroke
        input(
          type="checkbox"
          :checked="element.style?.webkitTextStroke && element.style.webkitTextStroke !== '0px'"
          @change="$emit('update-style', { property: 'webkitTextStroke', value: $event.target.checked ? '1px black' : '0px' })"
          class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
        )
      
      div(class="flex items-center justify-between")
        label(class="text-xs text-neutral-500") Gradient Text
        input(
          type="checkbox"
          :checked="element.style?.background && element.style.background.includes('linear-gradient')"
          @change="$emit('update-style', { property: 'background', value: $event.target.checked ? 'linear-gradient(45deg, #ff6b6b, #4ecdc4)' : 'transparent' })"
          class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
        )
      
      div(v-if="element.style?.background && element.style.background.includes('linear-gradient')")
        label(class="text-xs text-neutral-500 block mb-1") Gradient Colors
        div(class="grid grid-cols-2 gap-2")
          input(
            type="color"
            :value="getGradientColor1(element.style.background)"
            @input="updateGradient($event.target.value, getGradientColor2(element.style.background))"
            class="w-full h-8 border border-neutral-200 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          )
          input(
            type="color"
            :value="getGradientColor2(element.style.background)"
            @input="updateGradient(getGradientColor1(element.style.background), $event.target.value)"
            class="w-full h-8 border border-neutral-200 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          )
</template>

<script setup lang="ts">
interface Element {
  id: string
  type: string
  name: string
  content?: string
  style?: Record<string, any>
}

const props = defineProps<{
  element: Element
}>()

const emit = defineEmits<{
  'update-property': [event: { property: string, value: any }]
  'update-style': [event: { property: string, value: string }]
}>()

// Helper functions for gradient handling
const getGradientColor1 = (background: string) => {
  const match = background.match(/#[a-fA-F0-9]{6}/g)
  return match ? match[0] : '#ff6b6b'
}

const getGradientColor2 = (background: string) => {
  const match = background.match(/#[a-fA-F0-9]{6}/g)
  return match && match.length > 1 ? match[1] : '#4ecdc4'
}

const updateGradient = (color1: string, color2: string) => {
  emit('update-style', { 
    property: 'background', 
    value: `linear-gradient(45deg, ${color1}, ${color2})` 
  })
}
</script>
