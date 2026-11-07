<template lang="pug">
  div(class="flex-1 bg-themeBackground relative overflow-hidden")
    //- Canvas toolbar
    div(class="absolute top-4 left-1/2 -translate-x-1/2 z-10 bg-themeBackground2 rounded-xl shadow-md border border-themeBorder flex items-center p-1 gap-1.5 backdrop-blur-sm")
      div(class="flex items-center gap-1 bg-themeBackground3 p-0.5 rounded-lg")
        button(
          v-for="size in canvasSizes"
          :key="size.id"
          @click="$emit('update:canvasSize', size.id)"
          :class="canvasSize === size.id ? 'bg-themeBackground2 shadow-xs font-semibold text-themeText' : 'text-themeText2 hover:text-themeText'"
          class="px-3 py-1.5 rounded-md text-sm transition-all flex items-center gap-1.5 cursor-pointer"
        )
          Icon(v-if="size.icon" :icon="size.icon" class="w-4 h-4")
          span {{ size.label }}
      
      div(class="h-5 w-px bg-themeBorder")

      //- Canvas Settings Button
      button(
        @click="$emit('open-settings')"
        class="p-1.5 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-lg transition-colors flex items-center gap-1 text-xs font-medium cursor-pointer"
        title="Canvas & Resolution Settings"
      )
        svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z")
    
    //- Zoom controls
    div(class="absolute bottom-4 right-4 z-10 bg-themeBackground2 rounded-xl shadow-md border border-themeBorder flex items-center gap-2 px-3 py-2 text-themeText backdrop-blur-sm")
      button(@click="$emit('zoom-out')" class="text-themeText2 hover:text-themeText cursor-pointer")
        svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4")
      span(class="text-xs font-semibold text-themeText min-w-[3rem] text-center font-mono") {{ zoom }}%
      button(@click="$emit('zoom-in')" class="text-themeText2 hover:text-themeText cursor-pointer")
        svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4")
    
    //- Artboard with aspect ratio container
    div(class="absolute inset-0 flex items-center justify-center p-8")
      div(
        class="responsive-canvas-wrapper w-full h-full flex items-center justify-center"
        @dragover.prevent="onDragOver"
        @dragenter.prevent
        @drop="onDrop"
      )
        div(
          ref="canvasContainerRef"
          @click="$emit('canvas-click', $event)"
          @dragover.prevent="onDragOver"
          @dragenter.prevent="onDragEnter"
          @dragleave="onDragLeave"
          @drop="onDrop"
          class="relative canvas-container bg-white dark:bg-gradient-to-br dark:from-[#0e131f] dark:via-[#111827] dark:to-[#0a0d14] shadow-2xl shadow-black/30 overflow-hidden"
          :class="{ 'ring-2 ring-blue-500 ring-offset-2 dark:ring-offset-black/50 shadow-blue-500/20': isDragOverCanvas }"
          :style="canvasContainerStyle"
        )
          div(
            :style="{ width: canvasWidth + 'px', height: canvasHeight + 'px', transform: `scale(${canvasDisplayScale * (zoom / 100)})`, transformOrigin: 'top left' }"
            class="absolute inset-0 overflow-visible"
          )
            //- Grid overlay
            svg(v-if="showGrid" class="absolute inset-0 w-full h-full pointer-events-none z-10" preserveAspectRatio="none")
              defs
                pattern(id="grid" width="20" height="20" patternUnits="userSpaceOnUse")
                  path(d="M 20 0 L 0 0 0 20" fill="none" stroke="rgba(156, 163, 175, 0.3)" stroke-width="0.5")
              rect(width="100%" height="100%" fill="url(#grid)")
            
            //- Safe zone indicators
            div(v-if="showSafeZones" class="absolute inset-0 pointer-events-none z-10")
              //- Title safe area (90%)
              div(class="absolute inset-[5%] border border-yellow-400/30 border-dashed")
              //- Action safe area (93%)
              div(class="absolute inset-[3.5%] border border-green-400/30 border-dashed")
            
  
            //- Main content area (game capture simulation - Theme Adaptive)
            div(class="absolute inset-0 flex items-center justify-center pointer-events-none")
              div(class="text-center select-none")
                svg(class="w-14 h-14 mx-auto text-neutral-300 dark:text-neutral-600/70 mb-2.5 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z")
                p(class="text-neutral-500 dark:text-neutral-400/80 text-sm font-medium tracking-wide transition-colors") Game Capture Area
                p(class="text-neutral-400 dark:text-neutral-500/70 text-xs mt-0.5 font-mono transition-colors") {{ canvasWidth }}×{{ canvasHeight }} Resolution
                p(class="text-neutral-400/80 dark:text-neutral-500/60 text-[11px] font-mono transition-colors") Scale: {{ canvasDisplayScale.toFixed(3) }}

            //- Coordinate system debug markers (optional)
            div(v-if="showGrid" class="absolute inset-0 pointer-events-none z-20")
              //- Corner markers to show coordinate bounds
              div(class="absolute top-0 left-0 w-3 h-3 bg-red-500 opacity-50" title="(0,0)")
              div(class="absolute top-0 right-0 w-3 h-3 bg-blue-500 opacity-50" :title="`(${canvasWidth},0)`")
              div(class="absolute bottom-0 left-0 w-3 h-3 bg-green-500 opacity-50" :title="`(0,${canvasHeight})`")
              div(class="absolute bottom-0 right-0 w-3 h-3 bg-purple-500 opacity-50" :title="`(${canvasWidth},${canvasHeight})`")
  
            //- Interactive overlay elements - UNIFIED SYSTEM
            div(v-if="showPreviewElements")
              //- Render each canvas element using unified component
              div(
                v-for="(element, index) in visibleCanvasElements"
                :key="element.id"
                @mousedown="$emit('element-mousedown', $event, element)"
                @click="$emit('element-click', element)"
                @contextmenu="$emit('element-contextmenu', $event, element)"
                :class="[element.themeClass, { 'selected': selectedElement?.id === element.id, 'dragging': isDragging && selectedElement?.id === element.id }]"
                class="absolute cursor-move select-none element-interactive"
                :data-element-id="element.id"
                :style="{  left: element.x + 'px', top: element.y + 'px',   width: element.width + 'px',  height: element.height + 'px',  zIndex: (selectedElement?.id === element.id && element.type !== 'effect') ? 1000 : (element.zIndex ?? 0)  }"
              )
                //- Universal element - same as OBS rendering
                div(v-if="element.pendingImportReview" class="p-3 text-sm text-themeText bg-themeBackground2") Imported custom content — review before activation
                //- Scale content mode: render at design size then CSS-scale to current size
                //- Uses uniform scale (min of X/Y) so content is never distorted
                div(
                  v-else-if="element.type !== 'component' && element.scaleContent && element.designWidth && element.designHeight"
                  class="pointer-events-none absolute inset-0"
                )
                  div(
                    :style="{ width: element.designWidth + 'px', height: element.designHeight + 'px', transform: `scale(${Math.min(element.width / element.designWidth, element.height / element.designHeight)})`, transformOrigin: 'top left' }"
                  )
                    UniversalElement(:element="element")


                //- Normal mode: content fills the wrapper using 100%/inset:0
                UniversalElement(
                  v-else-if="element.type !== 'component'"
                  :element="element"
                  class="pointer-events-none"
                )
                
                //- Component type - render child elements as a grouped unit  
                div(v-else class="relative w-full h-full pointer-events-none")
                  //- Optional debug info (can be removed in production)
                  div(v-if="false" class="absolute top-0 right-0 bg-blue-600 text-white px-1 text-xs z-50 rounded-bl")
                    span {{ element.name }} ({{ element.childElements?.length || 0 }})
                  
                  //- Render child elements within the component using their EXACT positions from design
                  div(
                    v-for="child in element.childElements"
                    :key="child.id"
                    class="absolute"
                    :style="{ left: child.relativeX + 'px',top: child.relativeY + 'px',width: child.relativeWidth + 'px', height: child.relativeHeight + 'px',  zIndex: 5  }"
                  )
                    //- Render child element content based on its type with full styling
                    div(v-if="child.type === 'text'" class="h-full flex items-center justify-center overflow-hidden" :style="{ ...child.style }")
                      div(class="overlay-text" :style="{ ...child.style }") {{ child.content || child.preview || 'Sample Text' }}
                    
                    div(v-else-if="child.type === 'rectangle'" class="h-full" :style="{ ...child.style }")
                    
                    div(v-else-if="child.type === 'webcam'" class="flex items-center justify-center h-full" :style="{ backgroundColor: child.style?.backgroundColor || '#f3f4f6', borderColor: child.style?.borderColor, borderWidth: child.style?.borderWidth, borderStyle: 'solid', borderRadius: child.style?.borderRadius }")
                      svg(class="w-6 h-6 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z")
                    
                    div(v-else-if="child.type === 'counter'" class="p-2 text-center h-full flex flex-col justify-center" :style="{ ...child.style }")
                      div(class="text-xs font-medium" :style="{ color: child.style?.color }") {{ child.name || 'Counter' }}
                      div(class="text-lg font-bold" :style="{ color: child.style?.color }") 123
                    
                    div(v-else-if="child.type === 'progress_bar'" class="p-2 h-full flex flex-col justify-center" :style="{ ...child.style }")
                      div(class="text-xs font-medium mb-1" :style="{ color: child.style?.color }") {{ child.name || 'Progress' }}
                      div(class="w-full bg-gray-300 rounded-full h-2")
                        div(class="bg-blue-500 h-2 rounded-full transition-all" style="width: 50%")
                    
                    div(v-else-if="child.type === 'timer'" class="p-2 text-center h-full flex items-center justify-center" :style="{ ...child.style }")
                      div(class="text-lg font-bold font-mono" :style="{ color: child.style?.color }") {{ child.content || '00:00' }}
                    
                    div(v-else-if="child.type === 'container'" class="h-full border-2 border-dashed border-gray-400 flex items-center justify-center" :style="{ backgroundColor: child.style?.backgroundColor || 'transparent' }")
                      div(class="text-xs text-gray-500 opacity-50") {{ child.name || 'Container' }}
                    
                    div(v-else class="p-1 bg-yellow-100 border border-yellow-300 rounded text-xs text-center h-full flex items-center justify-center")
                      span {{ child.type }}
                
                //- Selection outline and resize handles
                div(v-if="selectedElement?.id === element.id" class="absolute -inset-1 border-2 border-blue-500 pointer-events-none selection-outline" data-element-overlay)
                  //- Resize handles
                  div(@mousedown="$emit('resize-start', $event, element, 'nw')" class="absolute -top-1 -left-1 w-3 h-3 bg-blue-500 cursor-nw-resize resize-handle pointer-events-auto")
                  div(@mousedown="$emit('resize-start', $event, element, 'n')" class="absolute -top-1 left-1/2 -translate-x-1/2 w-3 h-3 bg-blue-500 cursor-n-resize resize-handle pointer-events-auto")
                  div(@mousedown="$emit('resize-start', $event, element, 'ne')" class="absolute -top-1 -right-1 w-3 h-3 bg-blue-500 cursor-ne-resize resize-handle pointer-events-auto")
                  div(@mousedown="$emit('resize-start', $event, element, 'w')" class="absolute top-1/2 -translate-y-1/2 -left-1 w-3 h-3 bg-blue-500 cursor-w-resize resize-handle pointer-events-auto")
                  div(@mousedown="$emit('resize-start', $event, element, 'e')" class="absolute top-1/2 -translate-y-1/2 -right-1 w-3 h-3 bg-blue-500 cursor-e-resize resize-handle pointer-events-auto")
                  div(@mousedown="$emit('resize-start', $event, element, 'sw')" class="absolute -bottom-1 -left-1 w-3 h-3 bg-blue-500 cursor-sw-resize resize-handle pointer-events-auto")
                  div(@mousedown="$emit('resize-start', $event, element, 's')" class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-3 h-3 bg-blue-500 cursor-s-resize resize-handle pointer-events-auto")
                  div(@mousedown="$emit('resize-start', $event, element, 'se')" class="absolute -bottom-1 -right-1 w-3 h-3 bg-blue-500 cursor-se-resize resize-handle pointer-events-auto")
  
    //- Transparent drag overlay (covers entire screen during drag)
    div(
      v-if="isDragging"
      @mouseup="$emit('drag-end')"
      @mouseleave="$emit('drag-end')"
      @contextmenu.prevent.stop
      @mousedown.right.prevent.stop
      @selectstart.prevent
      @dragstart.prevent
      class="fixed inset-0 z-[9999] cursor-grabbing"
      style="background: transparent; pointer-events: auto; user-select: none;"
    )
    
    //- Transparent resize overlay (covers entire screen during resize)
    div(
      v-if="isElementResizing"
      @contextmenu.prevent.stop
      @mousedown.right.prevent.stop
      @selectstart.prevent
      @dragstart.prevent
      class="fixed inset-0 z-[9999]"
      :class="resizeHandle ? `cursor-${resizeHandle}-resize` : 'cursor-default'"
      style="background: transparent; pointer-events: auto; user-select: none;"
    )
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, nextTick, watch } from "vue";
import { Icon } from "@iconify/vue";
import UniversalElement from "./UniversalElement.vue";
import { getElementDefaults, activeDraggingElement } from "../../composables/overlay/componentSchema";

interface CanvasSize {
  id: string;
  label: string;
  icon?: string;
  width: number;
  height: number;
}

interface CanvasElement {
  id: string;
  type: string;
  x: number;
  y: number;
  width: number;
  height: number;
  style?: Record<string, any>;
  themeClass?: string;
}

const props = withDefaults(
  defineProps<{
    canvasSize?: string;
    zoom?: number;
    canvasSizes?: CanvasSize[];
    canvasWidth?: number;
    canvasHeight?: number;
    canvasAspectRatio?: string;
    canvasBaseWidth?: number;
    canvasBaseHeight?: number;
    canvasScale?: number;
    showGrid?: boolean;
    showSafeZones?: boolean;
    showPreviewElements?: boolean;
    visibleCanvasElements?: CanvasElement[];
    selectedElement?: CanvasElement | null;
    isDragging?: boolean;
    isElementResizing?: boolean;
    resizeHandle?: string;
  }>(),
  {
    canvasSize: "desktop",
    zoom: 100,
    canvasSizes: () => [],
    canvasWidth: 1920,
    canvasHeight: 1080,
    canvasAspectRatio: "16/9",
    canvasBaseWidth: 1920,
    canvasBaseHeight: 1080,
    canvasScale: 1,
    showGrid: false,
    showSafeZones: false,
    showPreviewElements: true,
    visibleCanvasElements: () => [],
    selectedElement: null,
    isDragging: false,
    isElementResizing: false,
    resizeHandle: "",
  }
);

const emit = defineEmits<{
  "update:canvasSize": [size: string];
  "open-settings": [];
  "open-add-element": [];
  "zoom-in": [];
  "zoom-out": [];
  "canvas-click": [event: MouseEvent];
  "element-mousedown": [event: MouseEvent, element: CanvasElement];
  "element-click": [element: CanvasElement];
  "element-contextmenu": [event: MouseEvent, element: CanvasElement];
  "resize-start": [event: MouseEvent, element: CanvasElement, handle: string];
  "drag-end": [];
  "update:canvasScale": [scale: number];
  "drop-element": [payload: { element: any; x?: number; y?: number }];
}>();

// Responsive canvas sizing logic
const maxCanvasWidth = ref(800);
const maxCanvasHeight = ref(600);

const canvasContainerRef = ref<HTMLElement | null>(null);
const isDragOverCanvas = ref(false);
const dragPreviewPos = ref<{ x: number; y: number; width: number; height: number; name?: string } | null>(null);

const onDragOver = (event: DragEvent) => {
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }

  const container = canvasContainerRef.value;
  if (!container) return;

  isDragOverCanvas.value = true;

  const rect = container.getBoundingClientRect();
  const actualWidth = props.canvasWidth || props.canvasBaseWidth;
  const actualHeight = props.canvasHeight || props.canvasBaseHeight;
  const zoomFactor = (props.zoom || 100) / 100;
  const scale = canvasDisplayScale.value * zoomFactor;

  const mouseX = (event.clientX - rect.left) / scale;
  const mouseY = (event.clientY - rect.top) / scale;

  const draggingItem = activeDraggingElement.value;
  const defaults = draggingItem
    ? getElementDefaults(draggingItem.schemaId || draggingItem.id)
    : undefined;
  const itemWidth = defaults?.width || draggingItem?.width || 120;
  const itemHeight = defaults?.height || draggingItem?.height || 80;

  const targetX = Math.round(
    Math.max(0, Math.min(mouseX - itemWidth / 2, actualWidth - itemWidth))
  );
  const targetY = Math.round(
    Math.max(0, Math.min(mouseY - itemHeight / 2, actualHeight - itemHeight))
  );

  dragPreviewPos.value = {
    x: targetX,
    y: targetY,
    width: itemWidth,
    height: itemHeight,
    name: draggingItem?.name || 'New Element',
  };
};

const onDragEnter = (event: DragEvent) => {
  event.preventDefault();
  isDragOverCanvas.value = true;
};

const onDragLeave = (event: DragEvent) => {
  const rect = canvasContainerRef.value?.getBoundingClientRect();
  if (rect) {
    if (
      event.clientX <= rect.left ||
      event.clientX >= rect.right ||
      event.clientY <= rect.top ||
      event.clientY >= rect.bottom
    ) {
      isDragOverCanvas.value = false;
      dragPreviewPos.value = null;
    }
  } else {
    isDragOverCanvas.value = false;
    dragPreviewPos.value = null;
  }
};

const onDrop = (event: DragEvent) => {
  event.preventDefault();
  isDragOverCanvas.value = false;
  dragPreviewPos.value = null;

  if (!event.dataTransfer) return;

  const rawData =
    event.dataTransfer.getData('vortideck/overlay-element') ||
    event.dataTransfer.getData('application/json');

  let item = activeDraggingElement.value;
  if (!item && rawData) {
    try {
      item = JSON.parse(rawData);
    } catch {
      // ignore
    }
  }

  activeDraggingElement.value = null;
  if (!item || typeof item !== 'object') return;

  const container = canvasContainerRef.value;
  if (!container) {
    emit('drop-element', { element: item });
    return;
  }

  const rect = container.getBoundingClientRect();
  const actualWidth = props.canvasWidth || props.canvasBaseWidth;
  const actualHeight = props.canvasHeight || props.canvasBaseHeight;
  const zoomFactor = (props.zoom || 100) / 100;
  const scale = canvasDisplayScale.value * zoomFactor;

  // Position relative to canvas top-left in actual canvas resolution pixels
  const mouseX = (event.clientX - rect.left) / scale;
  const mouseY = (event.clientY - rect.top) / scale;

  const defaults = getElementDefaults(item.schemaId || item.id);
  const itemWidth = defaults?.width || item.width || 120;
  const itemHeight = defaults?.height || item.height || 80;

  const targetX = Math.round(
    Math.max(0, Math.min(mouseX - itemWidth / 2, actualWidth - itemWidth))
  );
  const targetY = Math.round(
    Math.max(0, Math.min(mouseY - itemHeight / 2, actualHeight - itemHeight))
  );

  emit('drop-element', { element: item, x: targetX, y: targetY });
};

// Canvas display scale handles both resolution and container size scaling
// Canvas display scale - how much to scale the canvas content to fit the container
const canvasDisplayScale = computed(() => {
  // Get the current canvas resolution
  const actualWidth = props.canvasWidth || props.canvasBaseWidth;
  const actualHeight = props.canvasHeight || props.canvasBaseHeight;

  // Use available space from the computed wrapper dimensions
  const availableWidth = maxCanvasWidth.value;
  const availableHeight = maxCanvasHeight.value;

  // Calculate scale to fit resolution into available space
  const scale = Math.min(availableWidth / actualWidth, availableHeight / actualHeight);

  console.log(
    `📏 Canvas display scale: resolution=${actualWidth}x${actualHeight}, available=${availableWidth}x${availableHeight}, scale=${scale.toFixed(
      3
    )}`
  );

  return scale;
});

// Dynamic canvas container style with proper aspect ratio handling
const canvasContainerStyle = computed(() => {
  const actualWidth = props.canvasWidth || props.canvasBaseWidth;
  const actualHeight = props.canvasHeight || props.canvasBaseHeight;
  const scale = canvasDisplayScale.value;
  const zoomFactor = (props.zoom || 100) / 100;

  return {
    width: `${actualWidth * scale * zoomFactor}px`,
    height: `${actualHeight * scale * zoomFactor}px`,
    position: "relative",
    overflow: "hidden", // Prevents subpixel edge bleeding
  };
});


// Update canvas size based on container
const updateCanvasSize = () => {
  const wrapper = document.querySelector(".responsive-canvas-wrapper");
  if (wrapper) {
    const rect = wrapper.getBoundingClientRect();

    // Use full available space with minimal padding
    maxCanvasWidth.value = rect.width - 20; // Minimal padding
    maxCanvasHeight.value = rect.height - 20;

    console.log(
      `📏 Container size: ${rect.width}x${rect.height}, max canvas: ${maxCanvasWidth.value}x${maxCanvasHeight.value}`
    );

    // Emit updated values back to parent
    emit("update:canvasScale", canvasDisplayScale.value);

    // Canvas size now computed automatically
  }
};

// Watch for changes that might affect canvas size
watch(
  [
    () => props.zoom,
    () => props.canvasSize,
    () => props.canvasBaseWidth,
    () => props.canvasBaseHeight,
  ],
  () => {
    nextTick(() => {
      updateCanvasSize();
      console.log(
        `🔄 Canvas dimensions updated: ${props.canvasBaseWidth}x${props.canvasBaseHeight}`
      );
    });
  }
);

// Watch for canvas display changes and emit them
watch([canvasDisplayScale], () => {
  emit("update:canvasScale", canvasDisplayScale.value);
});

// ResizeObserver for container size tracking
let resizeObserver: ResizeObserver | null = null;

// Setup responsive canvas sizing on mount
onMounted(() => {
  // Initial size update
  nextTick(() => {
    updateCanvasSize();
    // Canvas size now computed automatically from scale and resolution
  });

  // Update on window resize
  window.addEventListener("resize", updateCanvasSize);

  // Use ResizeObserver for more accurate container size tracking
  if (typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(() => {
      updateCanvasSize();
    });

    nextTick(() => {
      const wrapper = document.querySelector(".responsive-canvas-wrapper");
      if (wrapper && resizeObserver) {
        resizeObserver.observe(wrapper);
      }
    });
  }
});

// Cleanup on unmount
onUnmounted(() => {
  window.removeEventListener("resize", updateCanvasSize);
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
});
</script>

<style scoped>
/* Responsive canvas wrapper that maintains aspect ratio */
.responsive-canvas-wrapper {
  width: 100%;
  height: 100%;
  max-width: calc(100vw - 76px); /* 56px icon rail + 20px buffer — right panel is floating overlay */
  max-height: calc(100vh - 160px); /* Account for compact header */
  display: flex;
  align-items: center;
  justify-content: center;
  /* Allow canvas to fill available space */
  flex: 1;
  overflow: visible;
  /* Add padding to prevent selection outlines from being clipped */
  padding: 8px;
  /* Ensure background matches to prevent dark corners */
  background-color: transparent;
}

/* Theme-aware components using CSS variables */
.canvas-container {
  /* Apply theme variables to canvas and all child elements */
  font-family: var(--theme-font-primary, "Inter", sans-serif);
  color: var(--theme-text, #ffffff);
  border: none !important;
  outline: none !important;

  /* Theme-aware elements can use these variables */
  --element-border: var(--theme-border, 2px solid #3b82f6);
  --element-radius: var(--theme-border-radius, 8px);
  --element-shadow: var(--theme-shadow, 0 4px 12px rgba(0, 0, 0, 0.1));
  --element-spacing: var(--theme-spacing-lg, 16px);
  --transition-speed: var(--theme-transition-normal, 250ms);

  /* Responsive sizing while maintaining aspect ratio */
  transform-origin: center;
  border: none;
  /* Ensure no background bleeds through */

  /* Scale to fit within wrapper while maintaining aspect ratio */
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

/* Example themed components */
.canvas-container .overlay-element {
  /* Only transition visual properties, not position/size */
  transition: background var(--transition-speed) var(--theme-ease, ease-in-out),
    border var(--transition-speed) var(--theme-ease, ease-in-out),
    box-shadow var(--transition-speed) var(--theme-ease, ease-in-out);
  font-family: var(--theme-font-primary);
}

.canvas-container .overlay-button {
  background: var(--theme-button-bg, #3b82f6);
  color: var(--theme-button-text, #ffffff);
  border: var(--theme-border-thin, 1px solid #3b82f6);
  border-radius: var(--theme-border-radius-sm, 4px);
  padding: var(--theme-spacing-sm, 8px) var(--theme-spacing-md, 12px);
  font-family: var(--theme-font-secondary);
  font-weight: var(--theme-font-weight-medium, 500);
  font-size: var(--theme-font-size-base, 14px);
  /* Only transition visual properties, not position/size */
  transition: background var(--theme-transition-fast, 150ms)
      var(--theme-ease, ease-in-out),
    border var(--theme-transition-fast, 150ms) var(--theme-ease, ease-in-out),
    box-shadow var(--theme-transition-fast, 150ms) var(--theme-ease, ease-in-out);
  filter: var(--theme-glow, none);
}

.canvas-container .overlay-button:hover {
  background: var(--theme-button-hover, #2563eb);
  box-shadow: var(--theme-shadow-sm, 0 2px 4px rgba(0, 0, 0, 0.1));
}

.canvas-container .overlay-text {
  font-family: var(--theme-font-primary);
  font-size: var(--theme-font-size-base, 14px);
  color: var(--theme-text, #ffffff);
  text-shadow: var(--theme-glow, none);
}

.canvas-container .overlay-input {
  background: var(--theme-input-bg, #000000);
  border: var(--theme-input-border, 1px solid #3b82f6);
  border-radius: var(--theme-border-radius, 8px);
  color: var(--theme-text, #ffffff);
  padding: var(--theme-spacing-sm, 8px);
  font-family: var(--theme-font-primary);
  /* Only transition visual properties, not position/size */
  transition: background var(--theme-transition-fast, 150ms)
      var(--theme-ease, ease-in-out),
    border var(--theme-transition-fast, 150ms) var(--theme-ease, ease-in-out);
}

.canvas-container .overlay-card {
  filter: var(--theme-glow, none);
}

/* Interactive canvas elements - GPU accelerated (Canva-style) */
.element-interactive {
  /* Only transition properties that don't interfere with drag operations */
  transition: border-color 0.15s ease, box-shadow 0.15s ease, z-index 0s;
  z-index: 10;
  border: 2px solid transparent;
  /* GPU acceleration optimizations */
  will-change: transform;
  transform: translateZ(0);
  backface-visibility: hidden;
  perspective: 1000px;
}

.element-interactive:hover {
  border-color: rgba(59, 130, 246, 0.5);
  box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.3), 0 4px 12px rgba(59, 130, 246, 0.1);
  z-index: 20;
  transform: translateZ(0); /* Force hardware acceleration */
}

/* Override any theme transitions that might interfere with drag operations */
.element-interactive[class*="theme"],
.element-interactive .overlay-element,
.element-interactive * {
  transition: border-color 0.15s ease, background-color 0.15s ease, box-shadow 0.15s ease !important;
}

.element-interactive.selected {
  z-index: 30;
  border-color: rgba(59, 130, 246, 0.8);
}

.element-interactive.selected:hover {
  border-color: rgba(59, 130, 246, 1);
}

/* Dragging element visual improvements */
.element-interactive.dragging {
  z-index: 9999 !important;
  /* Keep transitions for smooth visual feedback */
  transition: border-color 0.15s ease, box-shadow 0.15s ease, z-index 0s;
  /* Slightly reduce opacity for visual feedback */
  opacity: 0.9;
  /* Enhanced shadow while dragging */
  box-shadow: 0 8px 25px rgba(59, 130, 246, 0.3), 0 0 0 2px rgba(59, 130, 246, 0.6);
}

.selection-outline {
  border-style: dashed;
  border-color: #3b82f6;
  animation: dash 1s linear infinite;
}

@keyframes dash {
  0% {
    border-color: #3b82f6;
  }
  50% {
    border-color: #60a5fa;
  }
  100% {
    border-color: #3b82f6;
  }
}

.resize-handle {
  border-radius: 2px;
}

.resize-handle:hover {
  background-color: #3b82f6;
  transform: scale(1.2);
}
</style>
