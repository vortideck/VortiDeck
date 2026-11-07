<template lang="pug">
div(class="w-80 bg-themeBackground2/95 backdrop-blur-xl border border-themeBorder rounded-xl shadow-lg flex flex-col max-h-[calc(100%-1rem)] overflow-hidden text-themeText")
  //- Header with tabs and close button
  div(class="flex items-center border-b border-themeBorder")
    div(class="flex flex-1")
      button(
        v-for="tab in rightTabs"
        :key="tab"
        @click="$emit('update:activeTab', tab)"
        :class="activeTab === tab ? 'border-b-2 border-themePrimary text-themePrimary font-bold' : 'text-themeText2 hover:text-themeText'"
        class="flex-1 px-3 py-2.5 text-xs font-medium -mb-px transition-colors cursor-pointer"
      ) {{ tab }}
    button(@click="$emit('close')" class="p-2 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-lg transition-colors mr-1 cursor-pointer" title="Hide panel")
      svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

  //- Properties panel
  div(v-if="activeTab === 'Properties'" class="flex-1 overflow-y-auto p-4")
    div(v-if="!selectedElement" class="text-center py-12")
      div(class="w-14 h-14 mx-auto mb-4 rounded-full bg-themeBackground3 flex items-center justify-center")
        svg(class="w-7 h-7 text-themeText3" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122")
      p(class="text-themeText text-sm font-medium") Select an element
      p(class="text-themeText2 text-xs mt-1") Click on an item in the canvas to adjust its properties, styles, and behaviors.

    div(v-else class="space-y-6")
      //- Element name and type
      div
        h3(class="text-sm font-semibold text-themeText mb-2") {{ selectedElement.name }}
        p(class="text-xs text-themeText2 mb-3")
          span {{ selectedElement.type }} element
          span(v-if="elementSchema" class="ml-2 px-2 py-0.5 bg-themePrimary/15 text-themePrimary border border-themePrimary/25 rounded-md text-xs") {{ elementSchema.category }}

      //- Position and Size (Common for all elements)
      div
        h3(class="text-sm font-semibold text-themeText mb-3") Position & Size
        div(class="grid grid-cols-2 gap-3 mb-3")
          div
            label(class="text-xs text-themeText2 block mb-1") X Position
            input(
              type="number"
              :value="selectedElement.x"
              @input="$emit('update-element-property', 'x', Number($event.target.value))"
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
            )
          div
            label(class="text-xs text-themeText2 block mb-1") Y Position
            input(
              type="number"
              :value="selectedElement.y"
              @input="$emit('update-element-property', 'y', Number($event.target.value))"
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
            )
        div(class="grid grid-cols-2 gap-3")
          div
            label(class="text-xs text-themeText2 block mb-1") Width
            input(
              type="number"
              :value="selectedElement.width"
              @input="$emit('update-element-property', 'width', Number($event.target.value))"
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
            )
          div
            label(class="text-xs text-themeText2 block mb-1") Height
            input(
              type="number"
              :value="selectedElement.height"
              @input="$emit('update-element-property', 'height', Number($event.target.value))"
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
            )

        //- Scale content toggle
        label(class="flex items-center gap-2.5 mt-3 cursor-pointer group")
          div(
            :class="selectedElement.scaleContent ? 'bg-themePrimary' : 'bg-themeBackground3'"
            class="relative w-8 h-4 rounded-full transition-colors duration-200 flex-shrink-0"
          )
            div(
              :class="selectedElement.scaleContent ? 'translate-x-4' : 'translate-x-0.5'"
              class="absolute top-0.5 w-3 h-3 bg-white rounded-full shadow-sm transition-transform duration-200"
            )
            input(
              type="checkbox"
              :checked="selectedElement.scaleContent"
              @change="$emit('update-element-property', 'scaleContent', $event.target.checked)"
              class="sr-only"
            )
          div
            p(class="text-xs font-medium text-themeText leading-tight") Scale content with size
            p(class="text-[10px] text-themeText3 leading-tight mt-0.5") Fonts &amp; spacing resize proportionally

      //- Text Content (only for text-type elements)
      div(v-if="selectedElement.type === 'text'")
        h3(class="text-sm font-semibold text-themeText mb-3") Text Content
        textarea(
          :value="selectedElement.content || 'Sample Text'"
          @input="$emit('update-element-property', 'content', $event.target.value)"
          rows="3"
          class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText placeholder-themeText3 focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary resize-none"
          placeholder="Enter your text here..."
        )

      WidgetSettingsForm(
        v-if="selectedElement.configSchema"
        :key="selectedElement.id"
        :schema="selectedElement.configSchema"
        :config="selectedElement.config || {}"
        :revision="selectedElement.draftConfigRevision ?? selectedElement.configRevision ?? 0"
        :busy="settingsBusy"
        :error="settingsError"
        @apply="(config, revision) => $emit('apply-widget-settings', config, revision)"
      )

      //- Schema-driven editable properties
      //- Split into Content (dataProp) and Styling (cssVar/styleProp) sections
      template(v-if="editableProps.length > 0")
        //- Content section — dataProp props write to element.data
        div(v-if="dataProps.length > 0")
          h3(class="text-sm font-semibold text-themeText mb-3") Content
          button(v-if="selectedElement.schemaId === 'image'" type="button" :disabled="mediaBusy" class="mb-3 text-sm underline text-themeText disabled:opacity-50" @click="pickImage") {{ mediaBusy ? 'Selecting image…' : 'Embed local image (up to 1 MiB)' }}
          p(v-if="mediaError" role="alert" class="mb-3 text-xs text-themeText2") {{ mediaError }}
          div(class="space-y-3")
            div(v-for="prop in dataProps" :key="prop.key")
              label(class="text-xs text-themeText2 block mb-1") {{ prop.label }}
              input(
                v-if="prop.type === 'text'"
                type="text"
                :value="getCurrentDataValue(prop)"
                @input="$emit('update-element-data', prop.dataProp, $event.target.value)"
                class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText placeholder-themeText3 focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
              )
              input(
                v-else-if="prop.type === 'number'"
                type="number"
                :value="getCurrentDataValue(prop)"
                @input="$emit('update-element-data', prop.dataProp, Number($event.target.value))"
                class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText placeholder-themeText3 focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
              )
              select(
                v-else-if="prop.type === 'select'"
                :value="getCurrentDataValue(prop)"
                @change="$emit('update-element-data', prop.dataProp, $event.target.value)"
                class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
              )
                option(v-for="option in prop.options" :key="option" :value="option") {{ option }}

        //- Styling section — cssVar/styleProp props
        div(v-if="styleProps.length > 0")
          h3(class="text-sm font-semibold text-themeText mb-3") Styling
          div(class="space-y-3")
            //- Color picker with solid/gradient toggle
            template(v-for="prop in styleProps" :key="prop.key")
              div(v-if="prop.type === 'color'")
                div(class="flex items-center justify-between mb-1")
                  label(class="text-xs text-themeText2") {{ prop.label }}
                  button(
                    v-if="isBgProp(prop)"
                    @click="toggleGradientMode(prop)"
                    :class="isGradientValue(prop) ? 'bg-themePrimary/15 text-themePrimary border border-themePrimary/30' : 'bg-themeBackground3 text-themeText2 border border-themeBorder'"
                    class="px-2 py-0.5 rounded text-[10px] font-medium hover:opacity-80 transition-colors"
                  ) {{ isGradientValue(prop) ? 'Gradient' : 'Solid' }}
                //- Gradient editor
                div(v-if="isGradientValue(prop)" class="space-y-2")
                  div(
                    :style="{ background: getResolvedGradient(prop) }"
                    class="w-full h-10 rounded-lg border border-themeBorder"
                  )
                  div(class="flex items-center gap-2")
                    div(class="flex-1")
                      label(class="text-[10px] text-themeText3 block mb-0.5") Start
                      div(class="relative w-full h-8 rounded-lg border border-themeBorder overflow-hidden")
                        div(
                          :style="{ background: getGradientStops(prop)[0] }"
                          class="absolute inset-0 rounded-lg"
                        )
                        input(
                          type="color"
                          :value="getGradientStops(prop)[0]"
                          @input="handleGradientStopChange(prop, 0, $event.target.value)"
                          class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                        )
                    div(class="flex-1")
                      label(class="text-[10px] text-themeText3 block mb-0.5") End
                      div(class="relative w-full h-8 rounded-lg border border-themeBorder overflow-hidden")
                        div(
                          :style="{ background: getGradientStops(prop)[1] }"
                          class="absolute inset-0 rounded-lg"
                        )
                        input(
                          type="color"
                          :value="getGradientStops(prop)[1]"
                          @input="handleGradientStopChange(prop, 1, $event.target.value)"
                          class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                        )
                    div(class="w-16")
                      label(class="text-[10px] text-themeText3 block mb-0.5") Angle
                      input(
                        type="number"
                        :value="getGradientAngle(prop)"
                        @input="handleGradientAngleChange(prop, Number($event.target.value))"
                        min="0"
                        max="360"
                        step="15"
                        class="w-full px-2 py-1 h-8 bg-themeBackground border border-themeBorder rounded-lg text-xs text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary"
                      )
                //- Standard color picker for solid colors
                div(v-else class="relative w-full h-10 rounded-lg border border-themeBorder overflow-hidden")
                  div(
                    :style="{ background: getResolvedColor(prop) }"
                    class="absolute inset-0 rounded-lg"
                  )
                  input(
                    type="color"
                    :value="getResolvedColor(prop)"
                    @input="handlePropChange(prop, $event.target.value)"
                    class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                  )

              //- Range slider
              div(v-else-if="prop.type === 'range'")
                label(class="text-xs text-themeText2 block mb-1") {{ prop.label }}
                input(
                  type="range"
                  :min="prop.min || 0"
                  :max="prop.max || 100"
                  :step="prop.step || 1"
                  :value="getResolvedNumber(prop)"
                  @input="handleRangeChange(prop, $event.target.value)"
                  class="w-full"
                )
                span(class="text-xs text-themeText3") {{ getResolvedNumber(prop) }}{{ prop.unit || '' }}

              //- Font select
              div(v-else-if="prop.type === 'font'")
                label(class="text-xs text-themeText2 block mb-1") {{ prop.label }}
                select(
                  :value="getResolvedFont(prop)"
                  @change="handlePropChange(prop, $event.target.value)"
                  class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
                )
                  option(value="Arial, sans-serif") Arial
                  option(value="Helvetica, sans-serif") Helvetica
                  option(value="Times New Roman, serif") Times New Roman
                  option(value="Georgia, serif") Georgia
                  option(value="Verdana, sans-serif") Verdana
                  option(value="Courier New, monospace") Courier New
                  option(value="Impact, sans-serif") Impact
                  option(value="'Orbitron', monospace") Orbitron (Gaming)
                  option(value="'Press Start 2P', monospace") Press Start 2P (Retro)
                  option(value="'Rajdhani', sans-serif") Rajdhani (Modern)

              //- Select dropdown
              div(v-else-if="prop.type === 'select'")
                label(class="text-xs text-themeText2 block mb-1") {{ prop.label }}
                select(
                  :value="getCurrentValue(prop)"
                  @change="handlePropChange(prop, $event.target.value)"
                  class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
                )
                  option(v-for="opt in prop.options" :key="opt" :value="opt") {{ opt }}

              //- Number input
              div(v-else-if="prop.type === 'number'")
                label(class="text-xs text-themeText2 block mb-1") {{ prop.label }}
                input(
                  type="number"
                  :min="prop.min"
                  :max="prop.max"
                  :step="prop.step || 1"
                  :value="getResolvedNumber(prop)"
                  @input="handleRangeChange(prop, $event.target.value)"
                  class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
                )

              //- Text input
              div(v-else-if="prop.type === 'text'")
                label(class="text-xs text-themeText2 block mb-1") {{ prop.label }}
                input(
                  type="text"
                  :value="getCurrentValue(prop)"
                  @input="handlePropChange(prop, $event.target.value)"
                  class="w-full px-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
                )

      //- Quick Actions
      div
        h3(class="text-sm font-semibold text-themeText mb-3") Quick Actions
        div(class="grid grid-cols-2 gap-2")
          button(@click="$emit('duplicate-element')" class="px-3 py-2 bg-themePrimary/15 hover:bg-themePrimary/25 text-themePrimary border border-themePrimary/30 rounded-xl text-xs font-semibold transition-colors cursor-pointer") Duplicate
          button(@click="$emit('reset-element-style')" class="px-3 py-2 bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder rounded-xl text-xs font-semibold transition-colors cursor-pointer") Reset Style
          button(@click="$emit('bring-to-front')" class="px-3 py-2 bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-400 border border-emerald-500/30 rounded-xl text-xs font-semibold transition-colors cursor-pointer") To Front
          button(@click="$emit('delete-element')" class="px-3 py-2 bg-red-500/15 hover:bg-red-500/25 text-red-400 border border-red-500/30 rounded-xl text-xs font-semibold transition-colors cursor-pointer") Delete

  //- Layers panel
  div(v-if="activeTab === 'Layers'" class="flex-1 overflow-y-auto p-4")
    div(class="flex items-center justify-between mb-3")
      div(class="flex items-center gap-2")
        h3(class="text-sm font-semibold text-themeText") Scene Layers
        span(class="px-2 py-0.5 bg-themeBackground3 text-themeText2 rounded-full text-xs font-medium") {{ layers.length }} total
        span(v-if="visibleLayersCount < layers.length" class="px-2 py-0.5 bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 rounded-full text-xs font-medium") {{ visibleLayersCount }} visible
      button(@click="$emit('add-layer')" class="px-3 py-1 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold transition-colors cursor-pointer") Add Layer

    div(class="space-y-2")
      div(
        v-for="layer in layers"
        :key="layer.id"
        :class="layer.selected ? 'bg-themePrimary/15 border-themePrimary/50 text-themePrimary' : 'bg-themeBackground border-themeBorder text-themeText hover:bg-themeBackground3'"
        class="flex items-center gap-2 p-3 border rounded-xl transition-colors cursor-pointer"
        @click="$emit('select-layer', layer.id)"
      )
        button(class="text-themeText3 hover:text-themeText transition-colors" @click.stop="$emit('toggle-layer-visibility', layer.id)")
          svg(v-if="layer.visible" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z")
          svg(v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21")

        div(class="flex-1")
          p(class="text-sm font-semibold text-themeText") {{ layer.name }}
          p(class="text-xs text-themeText2") {{ layer.type }}

        button(class="text-themeText3 hover:text-themeText transition-colors")
          svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z")


</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import WidgetSettingsForm from '@base/components/WidgetSettingsForm.vue'
import { useOverlayTheme } from '../../composables/overlay/useOverlayTheme'
import { getComponentSchema, buildComponentVars, type EditableProp, type ComponentSchema } from '../../composables/overlay/componentSchema'

const { resolveVar } = useOverlayTheme()

interface Element {
  id: string
  name: string
  type: string
  x: number
  y: number
  width: number
  height: number
  style?: Record<string, any>
  schemaId?: string
  [key: string]: any
}

interface Layer {
  id: string
  name: string
  type: string
  visible: boolean
  selected: boolean
}

const props = withDefaults(defineProps<{
  settingsBusy?: boolean
  settingsError?: string
  activeTab?: string
  rightTabs?: string[]
  selectedElement?: Element | null
  layers?: Layer[]
  visibleLayersCount?: number
}>(), {
  activeTab: 'Properties',
  rightTabs: () => ['Properties', 'Layers'],
  selectedElement: null,
  layers: () => [],
  visibleLayersCount: 0,
})

const emit = defineEmits<{
  'apply-widget-settings': [config: Record<string, unknown>, expectedRevision: number]
  'close': []
  'update:activeTab': [tab: string]
  'update-element-property': [property: string, value: any]
  'update-element-style': [property: string, value: string]
  'update-element-data': [key: string, value: string | number]
  'duplicate-element': []
  'reset-element-style': []
  'bring-to-front': []
  'delete-element': []
  'add-layer': []
  'select-layer': [id: string]
  'toggle-layer-visibility': [id: string]
}>()

const mediaBusy = ref(false)
const mediaError = ref('')
watch(() => props.selectedElement?.id, () => { mediaError.value = '' })
async function pickImage() {
  const selectedId = props.selectedElement?.id
  mediaBusy.value = true; mediaError.value = ''
  try {
    const source = await invoke<string | null>('pick_overlay_image')
    if (source && selectedId === props.selectedElement?.id) emit('update-element-data', 'src', source)
  } catch (error) { mediaError.value = String(error) }
  finally { mediaBusy.value = false }
}

// ── Schema lookup ──────────────────────────────────────────────

/** Get the component schema for the currently selected element */
const elementSchema = computed<ComponentSchema | undefined>(() => {
  const el = props.selectedElement
  if (!el?.schemaId) return undefined
  return getComponentSchema(el.schemaId)
})

/** Get editable props from schema (or empty array) */
const editableProps = computed<EditableProp[]>(() => {
  return elementSchema.value?.editableProps || []
})

/** Props that write to element.data (design-time content) */
const dataProps = computed<EditableProp[]>(() =>
  editableProps.value.filter(p => p.dataProp)
)

/** Props that write to element.style (CSS vars / style properties) */
const styleProps = computed<EditableProp[]>(() =>
  editableProps.value.filter(p => !p.dataProp)
)

/** Get the current value of a dataProp from element.data */
const getCurrentDataValue = (prop: EditableProp): string | number => {
  const data = props.selectedElement?.data
  if (data && prop.dataProp && data[prop.dataProp] !== undefined) {
    return data[prop.dataProp]
  }
  return prop.defaultValue ?? ''
}

/** Build the --c-* var map for the selected element */
const componentVars = computed<Record<string, string>>(() => {
  const schema = elementSchema.value
  if (!schema) return {}
  return buildComponentVars(schema, props.selectedElement?.style)
})

// ── Value resolution ───────────────────────────────────────────

/**
 * Resolve a CSS value through the --c-* → --theme-* chain.
 */
const resolveFullChain = (value: string): string => {
  if (!value || typeof value !== 'string') return value
  const vars = componentVars.value
  // Pass 1: resolve --c-* vars
  let resolved = value.replace(/var\(([^)]+)\)/g, (_match, expr) => {
    const [varName] = expr.split(',')
    const name = varName.trim()
    if (name.startsWith('--c-') && vars[name]) return vars[name]
    return _match
  })
  // Pass 2: resolve --theme-* vars
  resolved = resolveVar(resolved)
  return resolved
}

/**
 * Get the current raw value for a prop (before resolution).
 */
const getCurrentValue = (prop: EditableProp): string => {
  const style = props.selectedElement?.style
  if (!style) return String(prop.defaultValue || '')

  // Check cssVar override on the element
  if (prop.cssVar) {
    const val = style[prop.cssVar]
    if (val && !String(val).startsWith('var(')) return String(val)
  }

  // Check styleProp
  if (prop.styleProp) {
    const val = style[prop.styleProp]
    if (val) return String(val)
  }

  return String(prop.defaultValue || '')
}

/**
 * Get a resolved color value for a color picker.
 * If the resolved value is a gradient, extract the first hex color.
 */
const getResolvedColor = (prop: EditableProp): string => {
  const style = props.selectedElement?.style
  if (!style) return '#ffffff'

  // Check for direct non-var value on the cssVar
  if (prop.cssVar) {
    const val = style[prop.cssVar]
    if (val && typeof val === 'string' && !val.startsWith('var(')) {
      const colorMatch = val.match(/#[0-9a-fA-F]{3,8}/)
      if (colorMatch) return colorMatch[0]
    }
  }

  // Check for direct non-var value on the styleProp
  if (prop.styleProp) {
    const val = style[prop.styleProp]
    if (val && typeof val === 'string' && !val.startsWith('var(')) {
      const colorMatch = val.match(/#[0-9a-fA-F]{3,8}/)
      if (colorMatch) return colorMatch[0]
    }
  }

  // Resolve through the var chain
  const componentColor = prop.cssVar ? componentVars.value[prop.cssVar] : undefined
  if (componentColor) {
    const resolved = resolveFullChain(componentColor)
    const colorMatch = resolved.match(/#[0-9a-fA-F]{3,8}|rgb[a]?\([^)]+\)/)
    return colorMatch ? colorMatch[0] : '#ffffff'
  }

  return '#ffffff'
}

/**
 * Get a resolved number value for a range/number input.
 */
const getResolvedNumber = (prop: EditableProp): number => {
  const style = props.selectedElement?.style
  const fallback = typeof prop.defaultValue === 'number' ? prop.defaultValue : 0

  if (!style) return fallback

  // Check styleProp first (e.g., borderWidth, fontSize)
  if (prop.styleProp) {
    const val = style[prop.styleProp]
    if (val) {
      const num = parseInt(String(val))
      if (!isNaN(num)) return num
    }
    // Also check the border shorthand for borderWidth
    if (prop.styleProp === 'borderWidth' && style.border) {
      const resolved = resolveFullChain(String(style.border))
      const num = parseInt(resolved)
      if (!isNaN(num)) return num
    }
  }

  // Check cssVar
  const componentNumber = prop.cssVar ? componentVars.value[prop.cssVar] : undefined
  if (componentNumber) {
    const resolved = resolveFullChain(componentNumber)
    const num = parseInt(resolved)
    if (!isNaN(num)) return num
  }

  return fallback
}

/**
 * Check if a color prop's resolved value is a gradient (not a solid color).
 */
const isGradientValue = (prop: EditableProp): boolean => {
  if (!prop.cssVar) return false
  const style = props.selectedElement?.style
  // If the user set a direct hex/rgb value, it's not a gradient
  if (style?.[prop.cssVar] && typeof style[prop.cssVar] === 'string' && !String(style[prop.cssVar]).startsWith('var(')) {
    return String(style[prop.cssVar]).includes('gradient')
  }
  // Resolve through the var chain and check
  const componentValue = componentVars.value[prop.cssVar]
  if (componentValue) {
    const resolved = resolveFullChain(componentValue)
    return resolved.includes('gradient')
  }
  return false
}

/**
 * Get the fully resolved gradient string for the preview swatch.
 */
const getResolvedGradient = (prop: EditableProp): string => {
  if (!prop.cssVar) return 'transparent'
  const componentValue = componentVars.value[prop.cssVar]
  if (componentValue) {
    return resolveFullChain(componentValue)
  }
  return 'transparent'
}

/**
 * Parse a resolved gradient string and extract the two color stops as hex values.
 */
const getGradientStops = (prop: EditableProp): [string, string] => {
  const resolved = getResolvedGradient(prop)
  const hexMatches = resolved.match(/#[0-9a-fA-F]{3,8}/g) || []
  return [hexMatches[0] || '#ff00ff', hexMatches[1] || hexMatches[0] || '#00ffff']
}

/**
 * Parse the gradient angle (e.g., 135 from "linear-gradient(135deg, ...)").
 */
const getGradientAngle = (prop: EditableProp): number => {
  const resolved = getResolvedGradient(prop)
  const angleMatch = resolved.match(/(\d+)deg/)
  return angleMatch ? parseInt(angleMatch[1] ?? '135') : 135
}

/**
 * Check if a prop is a background-type prop (eligible for gradient toggle).
 */
const isBgProp = (prop: EditableProp): boolean => {
  return prop.cssVar === '--c-bg' || prop.cssVar === '--c-bg-alt' || prop.key === 'bg'
}

/**
 * Toggle between solid color and gradient mode.
 * Solid → Gradient: creates a gradient from current color + a shifted hue.
 * Gradient → Solid: extracts the first color from the gradient.
 */
const toggleGradientMode = (prop: EditableProp) => {
  if (isGradientValue(prop)) {
    // Gradient → Solid: use the first stop color
    const stops = getGradientStops(prop)
    if (prop.cssVar) {
      emit('update-element-style', prop.cssVar, stops[0])
    }
  } else {
    // Solid → Gradient: build from current color + shifted hue
    const currentColor = getResolvedColor(prop)
    const shifted = shiftHue(currentColor, 40)
    const gradient = `linear-gradient(135deg, ${currentColor}, ${shifted})`
    if (prop.cssVar) {
      emit('update-element-style', prop.cssVar, gradient)
    }
  }
}

/**
 * Shift a hex color's hue by a number of degrees.
 */
function shiftHue(hex: string, degrees: number): string {
  const r = parseInt(hex.slice(1, 3), 16) / 255
  const g = parseInt(hex.slice(3, 5), 16) / 255
  const b = parseInt(hex.slice(5, 7), 16) / 255

  const max = Math.max(r, g, b), min = Math.min(r, g, b)
  const l = (max + min) / 2
  let h = 0, s = 0

  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6
    else if (max === g) h = ((b - r) / d + 2) / 6
    else h = ((r - g) / d + 4) / 6
  }

  h = ((h * 360 + degrees) % 360) / 360

  const hue2rgb = (p: number, q: number, t: number) => {
    if (t < 0) t += 1; if (t > 1) t -= 1
    if (t < 1/6) return p + (q - p) * 6 * t
    if (t < 1/2) return q
    if (t < 2/3) return p + (q - p) * (2/3 - t) * 6
    return p
  }

  const q = l < 0.5 ? l * (1 + s) : l + s - l * s
  const p = 2 * l - q
  const rr = Math.round(hue2rgb(p, q, h + 1/3) * 255)
  const gg = Math.round(hue2rgb(p, q, h) * 255)
  const bb = Math.round(hue2rgb(p, q, h - 1/3) * 255)

  return `#${rr.toString(16).padStart(2, '0')}${gg.toString(16).padStart(2, '0')}${bb.toString(16).padStart(2, '0')}`
}

/**
 * Handle changing a single gradient color stop — rebuild the gradient and emit.
 */
const handleGradientStopChange = (prop: EditableProp, stopIndex: number, color: string) => {
  const stops = getGradientStops(prop)
  stops[stopIndex] = color
  const angle = getGradientAngle(prop)
  const gradient = `linear-gradient(${angle}deg, ${stops[0]}, ${stops[1]})`
  if (prop.cssVar) {
    emit('update-element-style', prop.cssVar, gradient)
  }
}

/**
 * Handle changing the gradient angle — rebuild the gradient and emit.
 */
const handleGradientAngleChange = (prop: EditableProp, angle: number) => {
  const stops = getGradientStops(prop)
  const clamped = Math.max(0, Math.min(360, angle))
  const gradient = `linear-gradient(${clamped}deg, ${stops[0]}, ${stops[1]})`
  if (prop.cssVar) {
    emit('update-element-style', prop.cssVar, gradient)
  }
}

/**
 * Get a resolved font family value for the font select.
 */
const getResolvedFont = (prop: EditableProp): string => {
  const style = props.selectedElement?.style

  // Check cssVar override
  if (prop.cssVar && style?.[prop.cssVar]) {
    const val = String(style[prop.cssVar])
    if (!val.startsWith('var(')) return val
  }

  // Check styleProp
  if (prop.styleProp && style?.[prop.styleProp]) {
    const val = String(style[prop.styleProp])
    if (!val.startsWith('var(')) return val
  }

  // Resolve through var chain
  const componentFont = prop.cssVar ? componentVars.value[prop.cssVar] : undefined
  if (componentFont) {
    return resolveFullChain(componentFont)
  }

  return 'Arial, sans-serif'
}

// ── Prop change handlers ───────────────────────────────────────

/**
 * Handle a property value change from the UI.
 * Routes to the correct style key (cssVar or styleProp).
 */
const handlePropChange = (prop: EditableProp, value: string) => {
  if (prop.cssVar) {
    // Set the --c-* var directly on the element style
    emit('update-element-style', prop.cssVar, value)
  } else if (prop.styleProp) {
    emit('update-element-style', prop.styleProp, value)
  }
}

/**
 * Handle a range/number prop change, appending the unit if needed.
 */
const handleRangeChange = (prop: EditableProp, value: string) => {
  const finalValue = prop.unit ? `${value}${prop.unit}` : value
  if (prop.cssVar) {
    emit('update-element-style', prop.cssVar, finalValue)
  } else if (prop.styleProp) {
    emit('update-element-style', prop.styleProp, finalValue)
  }
}
</script>
