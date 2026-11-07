<template lang="pug">
div(class="simulator-v2-root flex flex-col justify-between h-[calc(100vh-30px)] bg-slate-950 text-white select-none overflow-hidden")
  //- Top Bar
  div(class="flex flex-wrap items-center justify-between gap-4 px-6 py-2 bg-slate-900 border-b border-slate-800 z-30 flex-shrink-0")
    div(class="flex items-center gap-3")
      span(class="text-sm font-bold text-slate-100") Deck Simulator V2
      span(class="px-2 py-0.5 rounded-full bg-blue-600/30 text-blue-400 border border-blue-500/40 text-[10px] font-bold uppercase tracking-wider")
        | Multi-Span 2D Grid Engine

    //- Interactive vs Edit Mode Toggle
    div(class="flex items-center gap-3")
      label(class="relative inline-flex items-center cursor-pointer gap-2 bg-slate-800 px-3 py-1.5 rounded-lg border border-slate-700 hover:border-slate-600 transition-all")
        input(
          type="checkbox"
          v-model="isInteractiveMode"
          class="sr-only peer"
        )
        div(
          class="w-8 h-4 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:bg-blue-500"
        )
        span(class="text-xs font-semibold text-slate-200")
          | {{ isInteractiveMode ? 'Interactive Mode (Test Controls)' : 'Edit Mode (Drag & Resize Any Widget)' }}

    //- Quick Actions
    div(class="flex items-center gap-2")
      button(
        type="button"
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-bold shadow transition-all"
        @click="isPaletteOpen = true"
      )
        span + Add Widget

      button(
        type="button"
        class="w-8 h-8 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-300 hover:text-white flex items-center justify-center text-sm transition-all"
        @click="isLandscape = !isLandscape"
        :title="isLandscape ? 'Switch to Portrait' : 'Switch to Landscape'"
      )
        SolarSmartphoneRotate2Linear(class="w-4 h-4")

  //- Main Phone Canvas Screen
  div(class="flex-1 flex items-center justify-center p-4 overflow-hidden min-h-0 relative")
    //- Realistic Hardware Deck Canvas
    div(
      class="rotatable-phone-container relative rounded-2xl bg-[#090b10] border border-slate-800/80 shadow-2xl p-2 flex flex-col justify-between overflow-hidden"
      :class="isLandscape ? 'landscape' : 'portrait'"
    )
      //- Simulated OLED Screen Surface
      div(
        ref="gridCanvasRef"
        class="phone-display-surface relative w-full h-full rounded-xl bg-[#0c0f17] select-none overflow-hidden"
        @mousemove="handleGridMouseMove"
        @mouseup="handleGridMouseUp"
        @mouseleave="handleGridMouseUp"
      )
        //- Live Drag Ghost Target Preview
        div(
          v-if="draggingWidget && dragTargetPos"
          class="absolute rounded-xl border-2 border-dashed border-blue-400 bg-blue-500/20 pointer-events-none transition-all duration-75 z-40 shadow-[0_0_15px_rgba(59,130,246,0.3)]"
          :style="ghostStyle"
        )

        //- Live Corner Resize Ghost Target Preview
        div(
          v-if="resizingWidget && resizeGhostBox"
          class="absolute rounded-xl border-2 border-dashed border-cyan-400 bg-cyan-500/20 pointer-events-none transition-all duration-75 z-40 shadow-[0_0_15px_rgba(6,182,212,0.3)]"
          :style="resizeGhostStyle"
        )

        //- Positioned Multi-Span Widgets
        div(
          v-for="widget in widgets"
          :key="widget.id"
          class="absolute transition-all rounded-xl overflow-hidden flex flex-col bg-[#161a26] shadow-md border border-white/[0.05]"
          :style="getWidgetAbsoluteStyle(widget)"
          :class="[selectedWidgetId === widget.id && !isInteractiveMode ? 'ring-2 ring-blue-500 z-30' : 'z-10', !isInteractiveMode ? 'cursor-grab active:cursor-grabbing hover:border-blue-400/40' : 'active:scale-[0.96] active:brightness-125 cursor-pointer transition-transform duration-75']"
          @mousedown="handleWidgetMouseDown(widget, $event)"
          @click="handleWidgetClick(widget)"
          @contextmenu.prevent="openContextMenu(widget, $event)"
        )
          ActionButtonWidget(
            v-if="widget.type === 'action_button'"
            :item="widget"
            :isInteractive="isInteractiveMode"
            @action="handleWidgetAction"
          )
          VerticalFaderWidget(
            v-else-if="widget.type === 'vertical_fader'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )
          MediaBarWidget(
            v-else-if="widget.type === 'media_bar'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )
          RotaryDialWidget(
            v-else-if="widget.type === 'rotary_dial'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )
          SystemGaugeWidget(
            v-else-if="widget.type === 'system_gauge'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )
          RockerToggleWidget(
            v-else-if="widget.type === 'rocker_toggle'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )
          LiveChatWidget(
            v-else-if="widget.type === 'live_chat'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )
          VideoFeedWidget(
            v-else-if="widget.type === 'video_feed'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )
          NotesWidget(
            v-else-if="widget.type === 'stream_notes'"
            :item="widget"
            :isInteractive="isInteractiveMode"
          )

          //- Multi-Directional Resize Handles (in Edit Mode)
          template(v-if="!isInteractiveMode && selectedWidgetId === widget.id")
            div(
              class="absolute bottom-1 right-1 w-4 h-4 rounded bg-blue-600 hover:bg-blue-500 text-white flex items-center justify-center cursor-se-resize shadow-md z-40 select-none text-[8px] font-bold"
              title="Resize"
              @mousedown.stop="startCornerResize(widget, 'se', $event)"
            ) ⤡
            div(
              class="absolute bottom-1 left-1 w-4 h-4 rounded bg-blue-600 hover:bg-blue-500 text-white flex items-center justify-center cursor-sw-resize shadow-md z-40 select-none text-[8px] font-bold"
              title="Resize"
              @mousedown.stop="startCornerResize(widget, 'sw', $event)"
            ) ⤢
            div(
              class="absolute top-1 right-1 w-4 h-4 rounded bg-blue-600 hover:bg-blue-500 text-white flex items-center justify-center cursor-ne-resize shadow-md z-40 select-none text-[8px] font-bold"
              title="Resize"
              @mousedown.stop="startCornerResize(widget, 'ne', $event)"
            ) ⤢
            div(
              class="absolute top-1 left-1 w-4 h-4 rounded bg-blue-600 hover:bg-blue-500 text-white flex items-center justify-center cursor-nw-resize shadow-md z-40 select-none text-[8px] font-bold"
              title="Resize"
              @mousedown.stop="startCornerResize(widget, 'nw', $event)"
            ) ⤡

    //- Floating Context Menu Window
    div(
      v-if="isContextMenuOpen && contextMenuWidget"
      :style="contextMenuPositionStyle"
      class="fixed z-50 bg-[#141822]/95 border border-slate-700/80 rounded-xl shadow-2xl overflow-hidden min-w-[13rem] backdrop-blur-xl animate-in fade-in zoom-in-95 duration-100"
      @mousedown.stop=""
    )
      div(class="px-3 py-2 bg-gradient-to-r from-blue-600/90 to-purple-600/90 text-white font-bold text-xs flex items-center justify-between")
        span {{ contextMenuWidget.title || contextMenuWidget.type }}
        span(class="text-[10px] font-mono opacity-80") ({{ contextMenuWidget.w }}×{{ contextMenuWidget.h }})

      div(class="p-1.5 space-y-1 text-xs text-slate-200")
        div(class="px-2 py-0.5 font-semibold text-[10px] text-slate-400 uppercase tracking-wider") Quick Size
        div(class="grid grid-cols-3 gap-1")
          button(
            v-for="sz in [{ w: 1, h: 1, l: '1×1' }, { w: 2, h: 1, l: '2×1' }, { w: 3, h: 1, l: '3×1' }, { w: 1, h: 2, l: '1×2' }, { w: 2, h: 2, l: '2×2' }, { w: 3, h: 2, l: '3×2' }]"
            :key="sz.l"
            type="button"
            class="py-1 rounded text-center text-xs font-semibold transition-colors"
            :class="contextMenuWidget.w === sz.w && contextMenuWidget.h === sz.h ? 'bg-blue-600 text-white' : 'bg-slate-800/80 hover:bg-slate-700 text-slate-300'"
            @click="resizeWidget(contextMenuWidget, sz.w, sz.h); closeContextMenu()"
          ) {{ sz.l }}

        div(class="h-px bg-slate-700/60 my-1")

        div(class="px-2 py-0.5 font-semibold text-[10px] text-slate-400 uppercase tracking-wider") Type
        div(class="space-y-0.5 max-h-36 overflow-y-auto")
          button(
            v-for="t in [{ id: 'action_button', label: 'Action Button' }, { id: 'vertical_fader', label: 'Vertical Fader' }, { id: 'media_bar', label: 'Media Player' }, { id: 'rotary_dial', label: 'Rotary Dial' }, { id: 'live_chat', label: 'Twitch Chat' }, { id: 'video_feed', label: 'OBS Video Feed' }]"
            :key="t.id"
            type="button"
            class="w-full text-left px-2 py-1 rounded hover:bg-slate-700/70 flex items-center justify-between transition-colors text-xs"
            :class="contextMenuWidget.type === t.id ? 'text-blue-400 font-bold' : 'text-slate-300'"
            @click="contextMenuWidget.type = t.id; closeContextMenu()"
          )
            span {{ t.label }}
            span(v-if="contextMenuWidget.type === t.id" class="text-blue-400") ✓

        div(class="h-px bg-slate-700/60 my-1")

        button(
          type="button"
          class="w-full text-left px-2 py-1.5 rounded hover:bg-red-500/20 text-red-400 hover:text-red-300 flex items-center gap-1.5 transition-colors font-semibold text-xs"
          @click="deleteWidget(contextMenuWidget.id); closeContextMenu()"
        )
          span 🗑 Delete Widget

  //- Sleek Pro Compact Bottom Control Bar
  div(class="w-full bg-[#0d1017]/95 border-t border-slate-800/80 z-30 flex-shrink-0 px-5 py-2.5 flex items-center justify-between gap-4 backdrop-blur-md")
    //- Left: Grid & Layout Presets
    div(class="flex items-center gap-3")
      div(class="flex items-center gap-1 bg-[#141822] p-0.5 rounded-lg border border-slate-800")
        button(
          v-for="preset in gridPresets"
          :key="preset.name"
          type="button"
          class="px-2.5 py-1 rounded text-xs font-mono font-bold transition-all"
          :class="currentGridPreset === preset.name ? 'bg-blue-600 text-white shadow-sm' : 'text-slate-400 hover:text-slate-200'"
          @click="setGridPreset(preset)"
        ) {{ preset.name }}

      div(class="h-4 w-px bg-slate-800")

      button(
        type="button"
        class="px-2.5 py-1 rounded-lg bg-[#141822] border border-slate-800 text-xs font-semibold text-slate-300 hover:text-white flex items-center gap-1.5 transition-colors"
        @click="isLandscape = !isLandscape"
      )
        span 🔄
        span {{ isLandscape ? 'Landscape' : 'Portrait' }}

    //- Center: Contextual Widget Controls (When selected)
    div(v-if="selectedWidget && !isInteractiveMode" class="flex items-center gap-2 bg-[#141822] px-3 py-1 rounded-xl border border-blue-500/30 animate-in fade-in duration-100")
      span(class="text-xs font-bold text-blue-400 truncate max-w-[100px]") {{ selectedWidget.title || selectedWidget.type }}
      div(class="h-3.5 w-px bg-slate-800")
      div(class="flex items-center gap-1")
        button(
          v-for="sz in [{ w: 1, h: 1, l: '1×1' }, { w: 2, h: 1, l: '2×1' }, { w: 1, h: 2, l: '1×2' }, { w: 2, h: 2, l: '2×2' }]"
          :key="sz.l"
          type="button"
          class="px-1.5 py-0.5 rounded text-[11px] font-semibold transition-all"
          :class="selectedWidget.w === sz.w && selectedWidget.h === sz.h ? 'bg-blue-600 text-white' : 'text-slate-400 hover:text-white'"
          @click="resizeWidget(selectedWidget, sz.w, sz.h)"
        ) {{ sz.l }}
      div(class="h-3.5 w-px bg-slate-800")
      button(
        type="button"
        class="text-red-400 hover:text-red-300 text-xs p-0.5"
        title="Delete Widget"
        @click="deleteWidget(selectedWidget.id)"
      ) 🗑

    //- Right: Action Buttons
    div(class="flex items-center gap-2 ml-auto")
      button(
        type="button"
        class="px-3.5 py-1 bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 text-white rounded-lg text-xs font-bold shadow-md transition-all active:scale-95"
        @click="saveDeck"
      ) Save Setup

  //- Widget Palette Modal
  WidgetPaletteModal(
    :isOpen="isPaletteOpen"
    @close="isPaletteOpen = false"
    @add="addWidgetFromPalette"
  )
</template>

<script setup lang="ts">
import type { DeckWidgetItem, WidgetManifest } from '../../components/DeckV2/types';
import ActionButtonWidget from '../../components/DeckV2/widgets/ActionButtonWidget.vue';
import VerticalFaderWidget from '../../components/DeckV2/widgets/VerticalFaderWidget.vue';
import MediaBarWidget from '../../components/DeckV2/widgets/MediaBarWidget.vue';
import RotaryDialWidget from '../../components/DeckV2/widgets/RotaryDialWidget.vue';
import SystemGaugeWidget from '../../components/DeckV2/widgets/SystemGaugeWidget.vue';
import RockerToggleWidget from '../../components/DeckV2/widgets/RockerToggleWidget.vue';
import LiveChatWidget from '../../components/DeckV2/widgets/LiveChatWidget.vue';
import VideoFeedWidget from '../../components/DeckV2/widgets/VideoFeedWidget.vue';
import NotesWidget from '../../components/DeckV2/widgets/NotesWidget.vue';
import WidgetPaletteModal from '../../components/DeckV2/WidgetPaletteModal.vue';
import SolarSmartphoneRotate2Linear from '~icons/solar/smartphone-rotate-2-linear?width=24px&height=24px';

definePageMeta({
  layout: 'simulator',
  middleware: [() => navigateTo('/deck/simulator', { replace: true })],
});

interface GridWidget extends DeckWidgetItem {
  x: number; // 0-indexed column
  y: number; // 0-indexed row
  w: number; // width in columns
  h: number; // height in rows
}

const isInteractiveMode = ref(false);
const isLandscape = ref(true);
const isPaletteOpen = ref(false);
const selectedWidgetId = ref<string | null>(null);
const activeBottomTab = ref<'deck' | 'widget' | 'templates'>('deck');
const gridGap = ref(8);

const gridPresets = [
  { name: '3×5', cols: 5, rows: 3 },
  { name: '3×6', cols: 6, rows: 3 },
  { name: '4×6', cols: 6, rows: 4 },
];
const currentGridPreset = ref('3×5');
const gridCols = ref(5);
const gridRows = ref(3);

const effectiveCols = computed(() => (isLandscape.value ? gridCols.value : gridRows.value));
const effectiveRows = computed(() => (isLandscape.value ? gridRows.value : gridCols.value));
const totalSlots = computed(() => effectiveCols.value * effectiveRows.value);

// Multi-Span Layout Preset (2x2 Twitch Chat, 2x2 OBS Feed, 1x2 Vertical Fader, 2x1 Spotify, 1x1 Keys)
const streamerWidgets: GridWidget[] = [
  { id: 'w-chat', type: 'live_chat', title: 'Twitch Chat', x: 0, y: 0, w: 2, h: 2, colSpan: 2, rowSpan: 2, config: {} },
  { id: 'w-obs', type: 'video_feed', title: 'OBS Feed', x: 2, y: 0, w: 2, h: 2, colSpan: 2, rowSpan: 2, config: {} },
  { id: 'w-fader', type: 'vertical_fader', title: 'Mic Gain', x: 4, y: 0, w: 1, h: 2, colSpan: 1, rowSpan: 2, config: { defaultLevel: 80 } },
  { id: 'w-media', type: 'media_bar', title: 'Spotify', x: 0, y: 2, w: 2, h: 1, colSpan: 2, rowSpan: 1, config: {} },
  { id: 'w-rotary', type: 'rotary_dial', title: 'Monitor', x: 2, y: 2, w: 1, h: 1, colSpan: 1, rowSpan: 1, config: { defaultValue: 65 } },
  { id: 'w-mic', type: 'rocker_toggle', title: 'Mic Mute', x: 3, y: 2, w: 1, h: 1, colSpan: 1, rowSpan: 1, config: {} },
  { id: 'w-clip', type: 'action_button', title: 'Clip That', x: 4, y: 2, w: 1, h: 1, colSpan: 1, rowSpan: 1, config: { iconName: 'record', badge: 'REC' } },
];

const widgets = ref<GridWidget[]>(JSON.parse(JSON.stringify(streamerWidgets)));

const templatePresets = [
  { name: 'Streamer Pro', data: streamerWidgets },
];
const activeTemplate = ref('Streamer Pro');

const loadTemplate = (tpl: any) => {
  activeTemplate.value = tpl.name;
  widgets.value = JSON.parse(JSON.stringify(tpl.data));
  selectedWidgetId.value = null;
};

const selectedWidget = computed(() => {
  return widgets.value.find((w) => w.id === selectedWidgetId.value) || null;
});

const bgGridStyle = computed(() => {
  return {
    gridTemplateColumns: `repeat(${effectiveCols.value}, minmax(0, 1fr))`,
    gridTemplateRows: `repeat(${effectiveRows.value}, minmax(0, 1fr))`,
    gap: `${gridGap.value}px`,
  };
});

// Dragging State with Movement Threshold and Pointer Anchor
const gridCanvasRef = ref<HTMLElement | null>(null);
const pendingDragWidget = ref<GridWidget | null>(null);
const draggingWidget = ref<GridWidget | null>(null);
const dragTargetPos = ref<{ x: number; y: number } | null>(null);
const dragStartPos = ref<{ mouseX: number; mouseY: number }>({ mouseX: 0, mouseY: 0 });
const grabOffset = ref<{ col: number; row: number }>({ col: 0, row: 0 });
const hasExceededDragThreshold = ref(false);

const DRAG_THRESHOLD_PX = 6;

const triggerHaptic = () => {
  if (typeof window !== 'undefined' && 'vibrate' in navigator) {
    navigator.vibrate?.(15);
  }
  try {
    const AudioCtx = window.AudioContext || (window as any).webkitAudioContext;
    if (AudioCtx) {
      const ctx = new AudioCtx();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'triangle';
      osc.frequency.setValueAtTime(450, ctx.currentTime);
      osc.frequency.exponentialRampToValueAtTime(80, ctx.currentTime + 0.035);
      gain.gain.setValueAtTime(0.12, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.035);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.04);
    }
  } catch (_) {}
};

const handleWidgetClick = (widget: GridWidget) => {
  if (isInteractiveMode.value) {
    triggerHaptic();
  } else {
    selectedWidgetId.value = widget.id;
  }
};

const handleWidgetAction = (item: any) => {
  triggerHaptic();
};

const handleWidgetMouseDown = (widget: GridWidget, e: MouseEvent) => {
  if (isInteractiveMode.value) return;
  selectedWidgetId.value = widget.id;
  activeBottomTab.value = 'widget';

  const rect = gridCanvasRef.value?.getBoundingClientRect();
  if (rect) {
    const cellWidth = rect.width / effectiveCols.value;
    const cellHeight = rect.height / effectiveRows.value;
    const clickRelX = e.clientX - rect.left;
    const clickRelY = e.clientY - rect.top;
    const clickedCol = Math.floor(clickRelX / cellWidth);
    const clickedRow = Math.floor(clickRelY / cellHeight);

    grabOffset.value = {
      col: Math.max(0, Math.min(widget.w - 1, clickedCol - widget.x)),
      row: Math.max(0, Math.min(widget.h - 1, clickedRow - widget.y)),
    };
  }

  pendingDragWidget.value = widget;
  dragStartPos.value = { mouseX: e.clientX, mouseY: e.clientY };
  hasExceededDragThreshold.value = false;
};

const handleGridMouseMove = (e: MouseEvent) => {
  if (!pendingDragWidget.value || !gridCanvasRef.value) return;

  // Check if moved beyond threshold
  if (!hasExceededDragThreshold.value) {
    const dist = Math.hypot(e.clientX - dragStartPos.value.mouseX, e.clientY - dragStartPos.value.mouseY);
    if (dist >= DRAG_THRESHOLD_PX) {
      hasExceededDragThreshold.value = true;
      draggingWidget.value = pendingDragWidget.value;
      dragTargetPos.value = { x: pendingDragWidget.value.x, y: pendingDragWidget.value.y };
    } else {
      return;
    }
  }

  const rect = gridCanvasRef.value.getBoundingClientRect();
  const relX = e.clientX - rect.left;
  const relY = e.clientY - rect.top;

  const cellWidth = rect.width / effectiveCols.value;
  const cellHeight = rect.height / effectiveRows.value;

  const rawCol = Math.floor(relX / cellWidth);
  const rawRow = Math.floor(relY / cellHeight);

  const targetCol = rawCol - grabOffset.value.col;
  const targetRow = rawRow - grabOffset.value.row;

  const hoverCol = Math.max(0, Math.min(effectiveCols.value - draggingWidget.value!.w, targetCol));
  const hoverRow = Math.max(0, Math.min(effectiveRows.value - draggingWidget.value!.h, targetRow));

  dragTargetPos.value = { x: hoverCol, y: hoverRow };
};

const handleGridMouseUp = () => {
  if (hasExceededDragThreshold.value && draggingWidget.value && dragTargetPos.value) {
    const targetX = dragTargetPos.value.x;
    const targetY = dragTargetPos.value.y;
    resolvePlacement(draggingWidget.value, targetX, targetY);
  }

  pendingDragWidget.value = null;
  draggingWidget.value = null;
  dragTargetPos.value = null;
  hasExceededDragThreshold.value = false;
};

const getWidgetAbsoluteStyle = (widget: GridWidget) => {
  const colWidthPct = 100 / effectiveCols.value;
  const rowHeightPct = 100 / effectiveRows.value;
  const gap = gridGap.value;

  const left = `calc(${widget.x * colWidthPct}% + ${gap / 2}px)`;
  const top = `calc(${widget.y * rowHeightPct}% + ${gap / 2}px)`;
  const width = `calc(${widget.w * colWidthPct}% - ${gap}px)`;
  const height = `calc(${widget.h * rowHeightPct}% - ${gap}px)`;

  const isBeingDragged = draggingWidget.value && draggingWidget.value.id === widget.id;

  return {
    left,
    top,
    width,
    height,
    opacity: isBeingDragged ? 0.35 : 1,
    transform: isBeingDragged ? 'scale(0.98)' : 'none',
    zIndex: isBeingDragged ? 40 : 10,
  };
};

const ghostStyle = computed(() => {
  if (!draggingWidget.value || !dragTargetPos.value) return {};
  const colWidthPct = 100 / effectiveCols.value;
  const rowHeightPct = 100 / effectiveRows.value;
  const gap = gridGap.value;

  const left = `calc(${dragTargetPos.value.x * colWidthPct}% + ${gap / 2}px)`;
  const top = `calc(${dragTargetPos.value.y * rowHeightPct}% + ${gap / 2}px)`;
  const width = `calc(${draggingWidget.value.w * colWidthPct}% - ${gap}px)`;
  const height = `calc(${draggingWidget.value.h * rowHeightPct}% - ${gap}px)`;

  return {
    left,
    top,
    width,
    height,
  };
});

// --- Rock-Solid 2D Occupancy Matrix Engine ---

const buildOccupancyMatrix = (items: GridWidget[], ignoreWidgetId: string | null = null): (string | null)[][] => {
  const matrix: (string | null)[][] = Array.from({ length: effectiveRows.value }, () =>
    Array(effectiveCols.value).fill(null)
  );

  for (const item of items) {
    if (item.id === ignoreWidgetId) continue;
    for (let r = item.y; r < item.y + item.h && r < effectiveRows.value; r++) {
      for (let c = item.x; c < item.x + item.w && c < effectiveCols.value; c++) {
        matrix[r][c] = item.id;
      }
    }
  }
  return matrix;
};

const canFitAt = (matrix: (string | null)[][], x: number, y: number, w: number, h: number): boolean => {
  if (x < 0 || y < 0 || x + w > effectiveCols.value || y + h > effectiveRows.value) return false;
  for (let r = y; r < y + h; r++) {
    for (let c = x; c < x + w; c++) {
      if (matrix[r][c] !== null) return false;
    }
  }
  return true;
};

const markInMatrix = (matrix: (string | null)[][], item: GridWidget) => {
  for (let r = item.y; r < item.y + item.h && r < effectiveRows.value; r++) {
    for (let c = item.x; c < item.x + item.w && c < effectiveCols.value; c++) {
      matrix[r][c] = item.id;
    }
  }
};

const findFirstAvailableSpot = (matrix: (string | null)[][], w: number, h: number): { x: number; y: number } | null => {
  for (let r = 0; r <= effectiveRows.value - h; r++) {
    for (let c = 0; c <= effectiveCols.value - w; c++) {
      if (canFitAt(matrix, c, r, w, h)) {
        return { x: c, y: r };
      }
    }
  }
  return null;
};

const resolvePlacement = (
  targetWidget: GridWidget,
  targetX: number,
  targetY: number,
  targetW: number = targetWidget.w,
  targetH: number = targetWidget.h
) => {
  // 1. Snapshot original state before ANY mutations
  const originalPositions = new Map<string, { x: number; y: number; w: number; h: number }>();
  for (const w of widgets.value) {
    originalPositions.set(w.id, { x: w.x, y: w.y, w: w.w, h: w.h });
  }

  // 2. Clamp target dimensions within grid bounds
  const nextW = Math.max(1, Math.min(effectiveCols.value, targetW));
  const nextH = Math.max(1, Math.min(effectiveRows.value, targetH));
  const nextX = Math.max(0, Math.min(effectiveCols.value - nextW, targetX));
  const nextY = Math.max(0, Math.min(effectiveRows.value - nextH, targetY));

  const origState = originalPositions.get(targetWidget.id)!;

  // 3. Find colliding widgets at target region
  const collidingWidgets: GridWidget[] = [];
  for (const item of widgets.value) {
    if (item.id === targetWidget.id) continue;
    const overlapX = nextX < item.x + item.w && nextX + nextW > item.x;
    const overlapY = nextY < item.y + item.h && nextY + nextH > item.y;
    if (overlapX && overlapY) {
      collidingWidgets.push(item);
    }
  }

  // 4. Build occupancy matrix for non-involved widgets
  const excludeIds = new Set([targetWidget.id, ...collidingWidgets.map(w => w.id)]);
  const matrix: (string | null)[][] = Array.from({ length: effectiveRows.value }, () =>
    Array(effectiveCols.value).fill(null)
  );

  for (const item of widgets.value) {
    if (!excludeIds.has(item.id)) {
      markInMatrix(matrix, item);
    }
  }

  // 5. Apply target placement to targetWidget
  targetWidget.x = nextX;
  targetWidget.y = nextY;
  targetWidget.w = nextW;
  targetWidget.h = nextH;
  targetWidget.colSpan = nextW;
  targetWidget.rowSpan = nextH;
  markInMatrix(matrix, targetWidget);

  // 6. Relocate colliding widgets
  let allFit = true;
  for (const colliding of collidingWidgets) {
    let placed = false;

    // A. Try vacated spot if same size
    if (canFitAt(matrix, origState.x, origState.y, colliding.w, colliding.h)) {
      colliding.x = origState.x;
      colliding.y = origState.y;
      markInMatrix(matrix, colliding);
      placed = true;
      continue;
    }

    // B. Scan for open spot matching full size
    const openSpot = findFirstAvailableSpot(matrix, colliding.w, colliding.h);
    if (openSpot) {
      colliding.x = openSpot.x;
      colliding.y = openSpot.y;
      markInMatrix(matrix, colliding);
      placed = true;
      continue;
    }

    // C. Fallback: adapt down to available slots
    for (let h = colliding.h; h >= 1 && !placed; h--) {
      for (let w = colliding.w; w >= 1 && !placed; w--) {
        const fallbackSpot = findFirstAvailableSpot(matrix, w, h);
        if (fallbackSpot) {
          colliding.w = w;
          colliding.h = h;
          colliding.colSpan = w;
          colliding.rowSpan = h;
          colliding.x = fallbackSpot.x;
          colliding.y = fallbackSpot.y;
          markInMatrix(matrix, colliding);
          placed = true;
        }
      }
    }

    if (!placed) {
      allFit = false;
      break;
    }
  }

  // 7. If any widget could not fit, rollback entire operation safely!
  if (!allFit) {
    for (const [id, pos] of originalPositions.entries()) {
      const item = widgets.value.find(w => w.id === id);
      if (item) {
        item.x = pos.x;
        item.y = pos.y;
        item.w = pos.w;
        item.h = pos.h;
        item.colSpan = pos.w;
        item.rowSpan = pos.h;
      }
    }
  }
};

const adjustWidgetSpan = (widget: GridWidget, deltaW: number, deltaH: number, direction: 'right' | 'left' | 'bottom' | 'top' = 'right') => {
  let nextW = Math.max(1, widget.w + deltaW);
  let nextH = Math.max(1, widget.h + deltaH);
  let nextX = widget.x;
  let nextY = widget.y;

  if (direction === 'left') {
    nextX = Math.max(0, widget.x - deltaW);
  } else if (direction === 'top') {
    nextY = Math.max(0, widget.y - deltaH);
  }

  resolvePlacement(widget, nextX, nextY, nextW, nextH);
};

const resizeWidget = (widget: GridWidget, newW: number, newH: number) => {
  resolvePlacement(widget, widget.x, widget.y, newW, newH);
};

// Resizing State
const resizingWidget = ref<GridWidget | null>(null);
const resizeGhostBox = ref<{ x: number; y: number; w: number; h: number } | null>(null);

const resizeGhostStyle = computed(() => {
  if (!resizeGhostBox.value) return {};
  const colWidthPct = 100 / effectiveCols.value;
  const rowHeightPct = 100 / effectiveRows.value;
  const gap = gridGap.value;

  const left = `calc(${resizeGhostBox.value.x * colWidthPct}% + ${gap / 2}px)`;
  const top = `calc(${resizeGhostBox.value.y * rowHeightPct}% + ${gap / 2}px)`;
  const width = `calc(${resizeGhostBox.value.w * colWidthPct}% - ${gap}px)`;
  const height = `calc(${resizeGhostBox.value.h * rowHeightPct}% - ${gap}px)`;

  return {
    left,
    top,
    width,
    height,
  };
});

const startCornerResize = (widget: GridWidget, handleDir: 'se' | 'sw' | 'ne' | 'nw', e: MouseEvent) => {
  if (isInteractiveMode.value) return;
  const rect = gridCanvasRef.value?.getBoundingClientRect();
  if (!rect) return;

  const cellWidth = rect.width / effectiveCols.value;
  const cellHeight = rect.height / effectiveRows.value;

  const startMouseX = e.clientX;
  const startMouseY = e.clientY;
  const startX = widget.x;
  const startY = widget.y;
  const startW = widget.w;
  const startH = widget.h;

  resizingWidget.value = widget;
  resizeGhostBox.value = { x: startX, y: startY, w: startW, h: startH };

  const onMouseMove = (moveEvent: MouseEvent) => {
    const deltaX = moveEvent.clientX - startMouseX;
    const deltaY = moveEvent.clientY - startMouseY;

    const colsDelta = Math.round(deltaX / cellWidth);
    const rowsDelta = Math.round(deltaY / cellHeight);

    let nextX = startX;
    let nextY = startY;
    let nextW = startW;
    let nextH = startH;

    if (handleDir === 'se') {
      nextW = Math.max(1, Math.min(effectiveCols.value - startX, startW + colsDelta));
      nextH = Math.max(1, Math.min(effectiveRows.value - startY, startH + rowsDelta));
    } else if (handleDir === 'sw') {
      const maxExpandLeft = startX;
      const leftShift = Math.max(-maxExpandLeft, Math.min(startW - 1, colsDelta));
      nextX = startX + leftShift;
      nextW = startW - leftShift;
      nextH = Math.max(1, Math.min(effectiveRows.value - startY, startH + rowsDelta));
    } else if (handleDir === 'ne') {
      const maxExpandTop = startY;
      const topShift = Math.max(-maxExpandTop, Math.min(startH - 1, rowsDelta));
      nextY = startY + topShift;
      nextH = startH - topShift;
      nextW = Math.max(1, Math.min(effectiveCols.value - startX, startW + colsDelta));
    } else if (handleDir === 'nw') {
      const maxExpandLeft = startX;
      const leftShift = Math.max(-maxExpandLeft, Math.min(startW - 1, colsDelta));
      nextX = startX + leftShift;
      nextW = startW - leftShift;

      const maxExpandTop = startY;
      const topShift = Math.max(-maxExpandTop, Math.min(startH - 1, rowsDelta));
      nextY = startY + topShift;
      nextH = startH - topShift;
    }

    resizeGhostBox.value = { x: nextX, y: nextY, w: nextW, h: nextH };
  };

  const onMouseUp = () => {
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);

    if (resizeGhostBox.value) {
      resolvePlacement(
        widget,
        resizeGhostBox.value.x,
        resizeGhostBox.value.y,
        resizeGhostBox.value.w,
        resizeGhostBox.value.h
      );
    }
    resizingWidget.value = null;
    resizeGhostBox.value = null;
  };

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);
};

// Floating Context Menu Window
const isContextMenuOpen = ref(false);
const contextMenuWidget = ref<GridWidget | null>(null);
const contextMenuPosition = ref<{ x: number; y: number }>({ x: 0, y: 0 });

const contextMenuPositionStyle = computed(() => {
  if (typeof window === 'undefined') return {};
  const left = Math.min(window.innerWidth - 230, Math.max(10, contextMenuPosition.value.x));
  const top = Math.min(window.innerHeight - 300, Math.max(10, contextMenuPosition.value.y));
  return {
    left: `${left}px`,
    top: `${top}px`,
  };
});

const openContextMenu = (widget: GridWidget, e: MouseEvent) => {
  if (isInteractiveMode.value) return;
  contextMenuWidget.value = widget;
  contextMenuPosition.value = { x: e.clientX, y: e.clientY };
  isContextMenuOpen.value = true;
  selectedWidgetId.value = widget.id;
  activeBottomTab.value = 'widget';
};

const closeContextMenu = () => {
  isContextMenuOpen.value = false;
  contextMenuWidget.value = null;
};

const deleteWidget = (widgetId: string) => {
  widgets.value = widgets.value.filter(w => w.id !== widgetId);
  if (selectedWidgetId.value === widgetId) {
    selectedWidgetId.value = null;
  }
};

const setGridPreset = (preset: { name: string; cols: number; rows: number }) => {
  currentGridPreset.value = preset.name;
  gridCols.value = preset.cols;
  gridRows.value = preset.rows;
};



const addWidgetFromPalette = (manifest: WidgetManifest) => {
  // Find first available slot
  let placeX = 0;
  let placeY = 0;
  const newWidget: GridWidget = {
    id: `w-${Date.now()}`,
    type: manifest.type,
    title: manifest.name,
    x: placeX,
    y: placeY,
    w: manifest.defaultSpan.cols || 1,
    h: manifest.defaultSpan.rows || 1,
    colSpan: manifest.defaultSpan.cols || 1,
    rowSpan: manifest.defaultSpan.rows || 1,
    config: { ...manifest.defaultConfig },
  };

  widgets.value.push(newWidget);
  selectedWidgetId.value = newWidget.id;
  activeBottomTab.value = 'widget';
};


const saveDeck = () => {
  console.log('Deck saved:', widgets.value);
};
</script>

<style lang="scss" scoped>
.simulator-v2-root {
  font-family: inherit;
}

.rotatable-phone-container {
  box-shadow: 0 25px 60px -10px rgba(0, 0, 0, 0.9), 0 0 0 1px rgba(255, 255, 255, 0.08);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  margin: auto;
}

.rotatable-phone-container.landscape {
  max-height: calc(100vh - 14rem);
  max-width: calc(100% - 1rem);
  aspect-ratio: 19 / 9;
  width: min(780px, calc((100vh - 14rem) * (19 / 9)));
  height: auto;
}

.rotatable-phone-container.portrait {
  max-height: calc(100vh - 14rem);
  max-width: calc(100% - 1rem);
  aspect-ratio: 9 / 19;
  height: min(620px, calc(100vh - 14rem));
  width: auto;
}
</style>
