<template lang="pug">
div(v-if="show" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50")
  div(class="bg-white rounded-lg shadow-xl w-[90vw] h-[90vh] flex flex-col")
    //- Header
    div(class="flex items-center justify-between p-6 border-b border-neutral-200")
      div(class="flex items-center gap-4")
        h2(class="text-xl font-semibold text-neutral-900") Component Creator
        span(class="px-2 py-1 bg-blue-100 text-blue-700 text-xs font-medium rounded") Pro Feature
      div(class="flex items-center gap-3")
        button(
          @click="previewComponent"
          class="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm font-medium hover:bg-blue-700 transition-colors"
        ) Preview
        button(
          @click="saveComponent"
          class="px-4 py-2 bg-green-600 text-white rounded-lg text-sm font-medium hover:bg-green-700 transition-colors"
        ) Save Component
        button(
          @click="$emit('close')"
          class="p-2 text-neutral-400 hover:text-neutral-600 transition-colors"
        )
          svg(class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
    
    //- Main Content
    div(class="flex-1 flex overflow-hidden")
      //- Left Panel - Component Library
      div(class="w-80 bg-neutral-50 border-r border-neutral-200 flex flex-col")
        div(class="p-4 border-b border-neutral-200")
          h3(class="text-sm font-medium text-neutral-700 mb-3") Component Library
          input(
            v-model="searchQuery"
            type="text"
            placeholder="Search components..."
            class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          )
        
        div(class="flex-1 overflow-y-auto p-4")
          //- Building Blocks
          div(class="mb-6")
            h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Building Blocks
            div(class="space-y-2")
              div(
                v-for="block in basicBuildingBlocks"
                :key="block.type"
                @click="addBuildingBlockToCanvas(block)"
                class="p-3 bg-white border border-neutral-200 rounded-lg cursor-pointer hover:border-blue-300 hover:shadow-sm transition-all"
              )
                div(class="flex items-center gap-3")
                  div(class="w-8 h-8 bg-blue-100 rounded flex items-center justify-center")
                    Icon(:icon="block.icon" class="w-5 h-5 text-blue-600")
                  div(class="flex-1")
                    p(class="text-sm font-medium text-neutral-900") {{ block.name }}
                    p(class="text-xs text-neutral-500") {{ block.description }}
          
          //- Component Templates
          div(class="mb-6")
            h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Templates (Start From)
            div(class="space-y-2")
              div(
                v-for="template in componentTemplates"
                :key="template.id"
                @click="loadTemplate(template)"
                class="p-3 bg-white border border-neutral-200 rounded-lg cursor-pointer hover:border-blue-300 hover:shadow-sm transition-all"
              )
                div(class="flex items-center gap-3")
                  div(class="w-8 h-8 bg-blue-100 rounded flex items-center justify-center")
                    Icon(:icon="template.icon" class="w-5 h-5 text-blue-600")
                  div(class="flex-1")
                    p(class="text-sm font-medium text-neutral-900") {{ template.name }}
                    p(class="text-xs text-neutral-500") {{ template.description }}
                  div(class="text-xs text-blue-600") Load
          
          //- Data Sources
          div
            h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Data Sources
            div(class="space-y-2")
              div(
                v-for="source in dataSources"
                :key="source.type"
                @click="addDataSource(source)"
                class="p-3 bg-white border border-neutral-200 rounded-lg cursor-pointer hover:border-green-300 hover:shadow-sm transition-all"
              )
                div(class="flex items-center gap-3")
                  div(class="w-8 h-8 bg-green-100 rounded flex items-center justify-center")
                    Icon(:icon="source.icon" class="w-5 h-5 text-green-600")
                  div(class="flex-1")
                    p(class="text-sm font-medium text-neutral-900") {{ source.name }}
                    p(class="text-xs text-neutral-500") {{ source.description }}
      
      //- Center - Canvas
      div(class="flex-1 flex flex-col")
        //- Canvas Toolbar
        div(class="p-4 border-b border-neutral-200 bg-white")
          div(class="flex items-center justify-between")
            div(class="flex items-center gap-4")
              button(
                @click="undo"
                :disabled="!canUndo"
                class="p-2 text-neutral-400 hover:text-neutral-600 disabled:opacity-50"
              )
                svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6")
              button(
                @click="redo"
                :disabled="!canRedo"
                class="p-2 text-neutral-400 hover:text-neutral-600 disabled:opacity-50"
              )
                svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6")
              div(class="w-px h-6 bg-neutral-200")
              button(
                @click="clearCanvas"
                class="p-2 text-red-400 hover:text-red-600"
              )
                svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16")
            div(class="flex items-center gap-2")
              span(class="text-sm text-neutral-500") Fixed Design Area:
              span(class="text-sm text-neutral-700 font-medium") {{ canvasWidth }}×{{ canvasHeight }}px
        
        //- Canvas Area
        div(class="flex-1 bg-neutral-100 p-6 overflow-auto")
          //- Grid toggle and canvas info
          div(class="flex items-center justify-between mb-4")
            div(class="flex items-center gap-3")
              label(class="flex items-center gap-2 text-sm text-neutral-600")
                input(type="checkbox" v-model="showGrid" class="rounded")
                span Show Grid
              label(class="flex items-center gap-2 text-sm text-neutral-600")
                input(type="checkbox" v-model="snapToGrid" class="rounded")
                span Snap to Grid
            div(class="text-xs text-neutral-500")
              span Fixed Design Area: {{ canvasWidth }}×{{ canvasHeight }}px - Position elements exactly as they should appear in the component
          
          div(
            class="bg-white border-2 border-dashed border-neutral-300 rounded-lg relative canvas-container"
            :style="{ width: canvasWidth + 'px', height: canvasHeight + 'px' }"
            @click="deselectAll"
            @dragover.prevent
            @drop="handleDrop"
          )
            //- Grid overlay
            div(v-if="showGrid" class="absolute inset-0 pointer-events-none opacity-30")
              svg(width="100%" height="100%")
                defs
                  pattern(id="grid-pattern" width="20" height="20" patternUnits="userSpaceOnUse")
                    path(d="M 20 0 L 0 0 0 20" fill="none" stroke="#e5e7eb" stroke-width="1")
                rect(width="100%" height="100%" fill="url(#grid-pattern)")
            //- Canvas Elements
            div(
              v-for="element in canvasElements"
              :key="element.id"
              :class="['absolute cursor-move', { 'ring-2 ring-blue-500': selectedElement?.id === element.id }]"
              :style="{left: element.x + 'px', top: element.y + 'px', width: element.width + 'px',  height: element.height + 'px' }"
              @click.stop="selectElement(element)"
              @mousedown="startDrag($event, element)"
            )
              //- Element Preview
              div(
                class="w-full h-full border border-neutral-200 rounded bg-white flex items-center justify-center text-xs text-neutral-500"
                :style="element.style"  )
                p {{ element.preview || element.name }}
              
              //- Resize Handles
              div(
                v-if="selectedElement?.id === element.id"
                class="absolute inset-0 pointer-events-none"
              )
                div(
                  v-for="handle in resizeHandles"
                  :key="handle"
                  :class="['absolute w-2 h-2 bg-blue-500 border border-white rounded pointer-events-auto', handle]"
                  @mousedown.stop="startResize($event, element, handle)"
                )
      
      //- Right Panel - Properties
      div(class="w-80 bg-white border-l border-neutral-200 flex flex-col")
        div(class="p-4 border-b border-neutral-200")
          h3(class="text-sm font-medium text-neutral-700") Properties
          p(v-if="!selectedElement" class="text-xs text-neutral-500 mt-1") Select an element to edit properties
        
        div(class="flex-1 overflow-y-auto p-4")
          //- Component Metadata (always visible)
          div(class="space-y-6 mb-6 pb-6 border-b border-neutral-200")
            h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Component Info
            div(class="space-y-3")
              div
                label(class="text-xs text-neutral-500 block mb-1") Component Name
                input(
                  v-model="componentName"
                  type="text"
                  class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="Enter component name..."
                )
              div
                label(class="text-xs text-neutral-500 block mb-1") Description
                textarea(
                  v-model="componentDescription"
                  rows="2"
                  class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                  placeholder="Describe what this component does..."
                )
          
          //- Element Properties
          div(v-if="selectedElement" class="space-y-6")
            //- Basic Properties
            div
              h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Basic Properties
              div(class="space-y-3")
                div
                  label(class="text-xs text-neutral-500 block mb-1") Name
                  input(
                    v-model="selectedElement.name"
                    type="text"
                    class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  )
                
                div(class="grid grid-cols-2 gap-3")
                  div
                    label(class="text-xs text-neutral-500 block mb-1") X
                    input(
                      v-model.number="selectedElement.x"
                      type="number"
                      class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    )
                  div
                    label(class="text-xs text-neutral-500 block mb-1") Y
                    input(
                      v-model.number="selectedElement.y"
                      type="number"
                      class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    )
                
                div(class="grid grid-cols-2 gap-3")
                  div
                    label(class="text-xs text-neutral-500 block mb-1") Width
                    input(
                      v-model.number="selectedElement.width"
                      type="number"
                      class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    )
                  div
                    label(class="text-xs text-neutral-500 block mb-1") Height
                    input(
                      v-model.number="selectedElement.height"
                      type="number"
                      class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                    )
            
            //- Component-Specific Properties
            div(v-if="selectedElement.properties")
              h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Component Properties
              div(class="space-y-3")
                div(
                  v-for="(prop, key) in selectedElement.properties"
                  :key="key"
                )
                  label(class="text-xs text-neutral-500 block mb-1") {{ prop.label || key }}
                  
                  //- Text Input
                  input(
                    v-if="prop.type === 'text'"
                    v-model="prop.value"
                    type="text"
                    class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  )
                  
                  //- Number Input
                  input(
                    v-else-if="prop.type === 'number'"
                    v-model.number="prop.value"
                    type="number"
                    :min="prop.min"
                    :max="prop.max"
                    class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  )
                  
                  //- Color Input
                  input(
                    v-else-if="prop.type === 'color'"
                    v-model="prop.value"
                    type="color"
                    class="w-full h-10 border border-neutral-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  )
                  
                  //- Select Input
                  select(
                    v-else-if="prop.type === 'select'"
                    v-model="prop.value"
                    class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  )
                    option(
                      v-for="option in prop.options"
                      :key="option"
                      :value="option"
                    ) {{ option }}
            
            //- Instance Override Settings
            div
              h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Instance Override Settings
              p(class="text-xs text-neutral-400 mb-3") Control which properties can be modified when this component is used in overlays
              div(class="space-y-2")
                label(class="flex items-center gap-2 text-xs text-neutral-600")
                  input(
                    type="checkbox"
                    :checked="selectedElement.instanceOverrides?.allowStyleOverride !== false"
                    @change="updateInstanceOverride('allowStyleOverride', $event.target.checked)"
                    class="rounded"
                  )
                  span Allow style changes (colors, fonts, etc.)
                label(class="flex items-center gap-2 text-xs text-neutral-600")
                  input(
                    type="checkbox"
                    :checked="selectedElement.instanceOverrides?.allowContentOverride !== false"
                    @change="updateInstanceOverride('allowContentOverride', $event.target.checked)"
                    class="rounded"
                  )
                  span Allow content changes (text, values, etc.)
                label(class="flex items-center gap-2 text-xs text-neutral-600")
                  input(
                    type="checkbox"
                    :checked="selectedElement.instanceOverrides?.allowPositionOverride !== false"
                    @change="updateInstanceOverride('allowPositionOverride', $event.target.checked)"
                    class="rounded"
                  )
                  span Allow position & size changes
                label(class="flex items-center gap-2 text-xs text-neutral-600")
                  input(
                    type="checkbox"
                    :checked="selectedElement.instanceOverrides?.lockAspectRatio === true"
                    @change="updateInstanceOverride('lockAspectRatio', $event.target.checked)"
                    class="rounded"
                  )
                  span Lock aspect ratio when resizing

            //- Data Binding
            div(v-if="selectedElement.dataBinding")
              h4(class="text-xs font-medium text-neutral-500 uppercase tracking-wide mb-3") Data Binding
              div(class="space-y-3")
                div(
                  v-for="(binding, key) in selectedElement.dataBinding"
                  :key="key"
                )
                  label(class="text-xs text-neutral-500 block mb-1") {{ binding.label || key }}
                  select(
                    v-model="binding.source"
                    class="w-full px-3 py-2 border border-neutral-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  )
                    option(value="") None
                    option(
                      v-for="source in dataSources"
                      :key="source.type"
                      :value="source.type"
                    ) {{ source.name }}
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Icon } from "@iconify/vue";

// Props and Emits
defineProps<{
  show: boolean;
}>();

const emit = defineEmits<{
  close: [];
  save: [component: any];
}>();

// Basic Building Blocks (not components, just elements)
const basicBuildingBlocks = ref([
  {
    type: "rectangle",
    name: "Rectangle",
    icon: "heroicons:square-2-stack",
    description: "Basic rectangle shape",
    category: "shape",
    properties: {
      backgroundColor: { type: "color", label: "Fill Color", value: "#3b82f6" },
      borderColor: { type: "color", label: "Border Color", value: "#1e40af" },
      borderWidth: { type: "number", label: "Border Width", value: 2, min: 0, max: 10 },
      borderRadius: { type: "number", label: "Corner Radius", value: 4, min: 0, max: 50 },
      opacity: { type: "number", label: "Opacity", value: 100, min: 0, max: 100 },
    },
  },
  {
    type: "text",
    name: "Text Label",
    icon: "heroicons:bars-3-bottom-left",
    description: "Single line of text",
    category: "text",
    properties: {
      content: { type: "text", label: "Text", value: "Label" },
      fontSize: { type: "number", label: "Size", value: 14, min: 8, max: 72 },
      color: { type: "color", label: "Color", value: "#ffffff" },
      fontWeight: {
        type: "select",
        label: "Weight",
        value: "normal",
        options: ["normal", "bold", "light"],
      },
      textAlign: {
        type: "select",
        label: "Align",
        value: "center",
        options: ["left", "center", "right"],
      },
    },
  },
  {
    type: "container",
    name: "Container",
    icon: "heroicons:rectangle-group",
    description: "Group other elements",
    category: "layout",
    properties: {
      backgroundColor: { type: "color", label: "Background", value: "transparent" },
      padding: { type: "number", label: "Padding", value: 8, min: 0, max: 20 },
      borderColor: { type: "color", label: "Border", value: "#e5e7eb" },
      borderWidth: { type: "number", label: "Border Width", value: 1, min: 0, max: 5 },
    },
  },
  {
    type: "image",
    name: "Image Box",
    icon: "heroicons:photo",
    description: "Image placeholder",
    category: "media",
    properties: {
      imageUrl: { type: "text", label: "Image URL", value: "" },
      fallbackColor: { type: "color", label: "Fallback Color", value: "#6b7280" },
      objectFit: {
        type: "select",
        label: "Fit",
        value: "cover",
        options: ["cover", "contain", "fill", "none"],
      },
    },
  },
  {
    type: "icon",
    name: "Icon",
    icon: "heroicons:star",
    description: "Icon or emoji",
    category: "decoration",
    properties: {
      icon: { type: "text", label: "Icon/Emoji", value: "star" },
      size: { type: "number", label: "Size", value: 24, min: 12, max: 64 },
      color: { type: "color", label: "Color", value: "#fbbf24" },
    },
  },
]);

// Component Templates - Pre-made combinations to start from
const componentTemplates = ref([
  {
    id: "follower_counter",
    name: "Follower Counter",
    icon: "heroicons:user-group",
    description: "Display follower count with label",
    elements: [
      { type: "container", x: 0, y: 0, width: 200, height: 60 },
      { type: "text", x: 10, y: 10, width: 180, height: 20, content: "Followers" },
      { type: "text", x: 10, y: 30, width: 180, height: 20, content: "1,234", fontSize: 18 },
    ],
  },
  {
    id: "custom_alert",
    name: "Alert Box",
    icon: "heroicons:bell-alert",
    description: "Customizable alert notification",
    elements: [
      { type: "rectangle", x: 0, y: 0, width: 300, height: 80, borderRadius: 8 },
      { type: "icon", x: 20, y: 25, width: 30, height: 30, icon: "party-popper" },
      { type: "text", x: 60, y: 20, width: 220, height: 20, content: "New Follower!" },
      { type: "text", x: 60, y: 40, width: 220, height: 20, content: "Username", fontSize: 12 },
    ],
  },
  {
    id: "stream_timer",
    name: "Stream Timer",
    icon: "heroicons:clock",
    description: "Elapsed time display",
    elements: [
      { type: "container", x: 0, y: 0, width: 150, height: 50 },
      { type: "text", x: 10, y: 15, width: 130, height: 20, content: "00:00:00", fontSize: 20 },
    ],
  },
]);

const dataSources = ref([
  {
    type: "twitch_followers",
    name: "Twitch Followers",
    icon: "heroicons:tv",
    description: "Live follower count",
  },
  {
    type: "twitch_subs",
    name: "Twitch Subscribers",
    icon: "heroicons:star",
    description: "Live subscriber count",
  },
  {
    type: "youtube_viewers",
    name: "YouTube Viewers",
    icon: "heroicons:play",
    description: "Live viewer count",
  },
  {
    type: "custom_api",
    name: "Custom API",
    icon: "heroicons:link",
    description: "Custom data source",
  },
]);

// Canvas State - Fixed component design area (resolution independent)
// Fixed design area for components - all components will be designed in this size
const COMPONENT_DESIGN_WIDTH = 300;
const COMPONENT_DESIGN_HEIGHT = 200;

const canvasWidth = computed(() => COMPONENT_DESIGN_WIDTH);
const canvasHeight = computed(() => COMPONENT_DESIGN_HEIGHT);

const canvasElements = ref<any[]>([]);
const selectedElement = ref<any>(null);
const searchQuery = ref("");

// Canvas options
const showGrid = ref(true);
const snapToGrid = ref(true);
const gridSize = ref(20);

// Component metadata
const componentName = ref("My Custom Component");
const componentDescription = ref("A custom component created with Component Creator");

// Drag and Drop
const isDragging = ref(false);
const dragElement = ref<any>(null);
const dragOffset = ref({ x: 0, y: 0 });

// Resize
const isResizing = ref(false);
const resizeElement = ref<any>(null);
const resizeHandle = ref("");

const resizeHandles = [
  "top-0 left-0 cursor-nw-resize",
  "top-0 left-1/2 -translate-x-1/2 cursor-n-resize",
  "top-0 right-0 cursor-ne-resize",
  "top-1/2 -translate-y-1/2 left-0 cursor-w-resize",
  "top-1/2 -translate-y-1/2 right-0 cursor-e-resize",
  "bottom-0 left-0 cursor-sw-resize",
  "bottom-0 left-1/2 -translate-x-1/2 cursor-s-resize",
  "bottom-0 right-0 cursor-se-resize",
];

// History
const history = ref<any[]>([]);
const historyIndex = ref(-1);

const canUndo = computed(() => historyIndex.value > 0);
const canRedo = computed(() => historyIndex.value < history.value.length - 1);

// Methods
const addBuildingBlockToCanvas = (block: any) => {
  const newElement = {
    id: `element_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    type: block.type,
    name: block.name,
    x: Math.random() * (canvasWidth.value - 200),
    y: Math.random() * (canvasHeight.value - 100),
    width: block.type === 'icon' ? 40 : 200,
    height: block.type === 'icon' ? 40 : block.type === 'text' ? 30 : 100,
    properties: JSON.parse(JSON.stringify(block.properties)),
    dataBinding: {},
    style: {},
    preview: block.name,
    // Initialize instance override settings
    instanceOverrides: {
      allowStyleOverride: true,
      allowContentOverride: true,
      allowPositionOverride: true,
      lockAspectRatio: false
    }
  };

  canvasElements.value.push(newElement);
  selectElement(newElement);
  saveToHistory();
};

const loadTemplate = (template: any) => {
  // Clear canvas first
  if (canvasElements.value.length > 0) {
    if (!confirm('Loading a template will clear your current design. Continue?')) {
      return;
    }
  }
  
  canvasElements.value = [];
  
  // Load all template elements
  template.elements.forEach((element: any) => {
    const newElement = {
      ...element,
      id: `element_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      properties: element.properties || {},
      dataBinding: {},
      style: element.style || {},
      preview: element.name || element.type,
      instanceOverrides: {
        allowStyleOverride: true,
        allowContentOverride: true,
        allowPositionOverride: true,
        lockAspectRatio: false
      }
    };
    canvasElements.value.push(newElement);
  });
  
  componentName.value = template.name;
  componentDescription.value = template.description;
  saveToHistory();
};

const updateInstanceOverride = (property: string, value: boolean) => {
  if (selectedElement.value) {
    if (!selectedElement.value.instanceOverrides) {
      selectedElement.value.instanceOverrides = {};
    }
    selectedElement.value.instanceOverrides[property] = value;
    saveToHistory();
  }
};

const addDataSource = (source: any) => {
  if (selectedElement.value) {
    if (!selectedElement.value.dataBinding) {
      selectedElement.value.dataBinding = {};
    }
    selectedElement.value.dataBinding[source.type] = {
      source: source.type,
      label: source.name,
    };
  }
};

const selectElement = (element: any) => {
  selectedElement.value = element;
};

const deselectAll = () => {
  selectedElement.value = null;
};

const startDrag = (event: MouseEvent, element: any) => {
  if (event.button !== 0) return;

  isDragging.value = true;
  dragElement.value = element;

  const rect = (event.target as HTMLElement).getBoundingClientRect();
  dragOffset.value = {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top,
  };

  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
};

const onDragMove = (event: MouseEvent) => {
  if (!isDragging.value || !dragElement.value) return;

  const canvas = document.querySelector(".canvas-container");
  if (!canvas) return;

  const canvasRect = canvas.getBoundingClientRect();
  let x = event.clientX - canvasRect.left - dragOffset.value.x;
  let y = event.clientY - canvasRect.top - dragOffset.value.y;

  // Snap to grid if enabled
  if (snapToGrid.value) {
    x = Math.round(x / gridSize.value) * gridSize.value;
    y = Math.round(y / gridSize.value) * gridSize.value;
  }

  dragElement.value.x = Math.max(
    0,
    Math.min(x, canvasWidth.value - dragElement.value.width)
  );
  dragElement.value.y = Math.max(
    0,
    Math.min(y, canvasHeight.value - dragElement.value.height)
  );
};

const onDragEnd = () => {
  isDragging.value = false;
  dragElement.value = null;
  document.removeEventListener("mousemove", onDragMove);
  document.removeEventListener("mouseup", onDragEnd);
  saveToHistory();
};

const startResize = (event: MouseEvent, element: any, handle: string) => {
  event.stopPropagation();

  isResizing.value = true;
  resizeElement.value = element;
  resizeHandle.value = handle;

  const startX = event.clientX;
  const startY = event.clientY;
  const startWidth = element.width;
  const startHeight = element.height;
  const startLeft = element.x;
  const startTop = element.y;

  const onResizeMove = (e: MouseEvent) => {
    const deltaX = e.clientX - startX;
    const deltaY = e.clientY - startY;

    let newWidth = startWidth;
    let newHeight = startHeight;
    let newX = startLeft;
    let newY = startTop;

    // Calculate new dimensions based on handle
    if (handle.includes("e")) newWidth = startWidth + deltaX;
    if (handle.includes("w")) {
      newWidth = startWidth - deltaX;
      newX = startLeft + deltaX;
    }
    if (handle.includes("s")) newHeight = startHeight + deltaY;
    if (handle.includes("n")) {
      newHeight = startHeight - deltaY;
      newY = startTop + deltaY;
    }

    // Enforce minimum size
    newWidth = Math.max(50, newWidth);
    newHeight = Math.max(30, newHeight);

    // Constrain to canvas
    newX = Math.max(0, Math.min(newX, canvasWidth.value - newWidth));
    newY = Math.max(0, Math.min(newY, canvasHeight.value - newHeight));

    element.width = newWidth;
    element.height = newHeight;
    element.x = newX;
    element.y = newY;
  };

  const onResizeEnd = () => {
    isResizing.value = false;
    resizeElement.value = null;
    resizeHandle.value = "";
    document.removeEventListener("mousemove", onResizeMove);
    document.removeEventListener("mouseup", onResizeEnd);
    saveToHistory();
  };

  document.addEventListener("mousemove", onResizeMove);
  document.addEventListener("mouseup", onResizeEnd);
};

const handleDrop = (event: DragEvent) => {
  // Handle file drops or other drag operations
};

const clearCanvas = () => {
  canvasElements.value = [];
  selectedElement.value = null;
  saveToHistory();
};

const undo = () => {
  if (canUndo.value) {
    historyIndex.value--;
    restoreFromHistory();
  }
};

const redo = () => {
  if (canRedo.value) {
    historyIndex.value++;
    restoreFromHistory();
  }
};

const saveToHistory = () => {
  const state = JSON.parse(JSON.stringify(canvasElements.value));
  history.value = history.value.slice(0, historyIndex.value + 1);
  history.value.push(state);
  historyIndex.value = history.value.length - 1;
};

const restoreFromHistory = () => {
  if (historyIndex.value >= 0 && historyIndex.value < history.value.length) {
    canvasElements.value = JSON.parse(JSON.stringify(history.value[historyIndex.value]));
  }
};

const previewComponent = () => {
  // Generate preview of the component
  const componentDefinition = generateComponentDefinition();
  console.log("Component Preview:", componentDefinition);
  
  // Show user feedback
  if (canvasElements.value.length === 0) {
    alert("Add some elements to the canvas before previewing.");
    return;
  }
  
  // Validate component
  const validation = validateComponent(componentDefinition);
  if (!validation.valid) {
    alert(`Component Validation Issues:\n${validation.issues.join('\n')}\n\nFix these issues before saving.`);
    return;
  }
  
  // Create a preview popup/modal or highlight the canvas
  alert(`✅ Component Preview: "${componentDefinition.name}"\n\n` +
        `📝 Description: ${componentDefinition.description}\n` +
        `🧩 Elements: ${componentDefinition.elements.length}\n` +
        `📐 Resolution Independent: ${componentDefinition.isResponsive ? 'Yes' : 'No'}\n\n` +
        `Check the console for full definition.`);
};

const validateComponent = (component: any) => {
  const issues = [];
  
  if (!component.name || component.name.trim() === "") {
    issues.push("- Component name is required");
  }
  
  if (!component.description || component.description.trim() === "") {
    issues.push("- Component description is required");
  }
  
  if (component.elements.length === 0) {
    issues.push("- Component must contain at least one element");
  }
  
  // Overlapping elements are now allowed and encouraged for complex components
  
  return {
    valid: issues.length === 0,
    issues: issues
  };
};

const saveComponent = () => {
  const componentDefinition = generateComponentDefinition();
  
  // Validate before saving
  const validation = validateComponent(componentDefinition);
  if (!validation.valid) {
    alert(`Cannot Save Component:\n${validation.issues.join('\n')}\n\nPlease fix these issues first.`);
    return;
  }
  
  console.log("Saving component:", componentDefinition);
  console.log("Component elements:", componentDefinition.elements);
  console.log("Element positions:", componentDefinition.elementPositions);
  
  // Emit the save event with the component definition
  emit("save", componentDefinition);
  
  // Show success feedback
  alert(`✅ Component "${componentDefinition.name}" saved successfully!\n\n` +
        `📝 Description: ${componentDefinition.description}\n` +
        `🧩 Elements: ${componentDefinition.elements.length}\n` +
        `📐 Fixed Design: ${COMPONENT_DESIGN_WIDTH}×${COMPONENT_DESIGN_HEIGHT}px\n\n` +
        `Your component will be placed as a single group on the overlay!`);
};

const generateComponentDefinition = () => {
  return {
    type: "custom_component",
    name: componentName.value,
    description: componentDescription.value,
    version: "1.0.0",
    author: "User",
    
    // Store elements with their EXACT positions from the design area
    // These positions will be used directly as relative positions within the component
    elements: canvasElements.value.map(el => ({
      id: el.id,
      type: el.type,
      name: el.name,
      
      // Use exact pixel positions from design area as relative positions
      x: el.x,
      y: el.y,
      width: el.width,
      height: el.height,
      
      // Preserve all styling and content
      style: { ...el.style },
      content: el.content,
      preview: el.preview,
      
      // Mark which properties are component defaults vs user-customizable
      componentTemplate: {
        type: el.type,
        defaultStyle: { ...el.style },
        defaultProperties: { ...el.properties },
        structure: {
          defaultWidth: el.width,
          defaultHeight: el.height,
          defaultName: el.name
        }
      },
      
      // Properties that can be overridden in overlay instances
      instanceOverrides: {
        allowStyleOverride: true,
        allowContentOverride: true,
        allowPositionOverride: false, // Position is fixed within component
        lockAspectRatio: false
      }
    })),
    
    // Fixed design area dimensions
    designWidth: COMPONENT_DESIGN_WIDTH,
    designHeight: COMPONENT_DESIGN_HEIGHT,
    
    // This is NOT responsive - it uses fixed positioning
    isResponsive: false,
    createdAt: new Date().toISOString(),
    
    // No percentage positioning needed - using direct pixel positions
  };
};

// Lifecycle
onMounted(() => {
  saveToHistory();
});

onUnmounted(() => {
  document.removeEventListener("mousemove", onDragMove);
  document.removeEventListener("mouseup", onDragEnd);
});
</script>

<style scoped>
.canvas-container {
  position: relative;
  background: white;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.element {
  user-select: none;
}

.element.selected {
  z-index: 10;
}

.resize-handle {
  position: absolute;
  width: 8px;
  height: 8px;
  background: #3b82f6;
  border: 2px solid white;
  border-radius: 50%;
  cursor: pointer;
}

.resize-handle:hover {
  background: #2563eb;
  transform: scale(1.2);
}
</style>
