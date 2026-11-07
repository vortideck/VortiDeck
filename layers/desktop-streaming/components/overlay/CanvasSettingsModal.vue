<template lang="pug">
OverlayModal(
  :show="show"
  title="Canvas & Viewport Settings"
  subtitle="Configure resolution presets, grid guides, stream safe zones, and OBS broadcast links"
  modal-class="max-w-2xl max-h-[90vh] h-auto"
  @close="$emit('close')"
)
  div(class="p-6 space-y-6 text-neutral-800 dark:text-themeText overflow-y-auto max-h-[65vh]")
    //- 1. Resolution & Viewport Presets
    div(class="space-y-3")
      div(class="flex items-center justify-between")
        label(class="text-xs font-bold uppercase tracking-wider text-neutral-500 dark:text-themeText2 flex items-center gap-1.5")
          svg(class="w-4 h-4 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z")
          | Resolution Presets
        span(class="text-xs text-neutral-500 dark:text-themeText2 font-mono font-semibold") {{ customWidth }} × {{ customHeight }} px

      div(class="grid grid-cols-2 sm:grid-cols-3 gap-2.5")
        button(
          v-for="preset in resolutionPresets"
          :key="preset.id"
          @click="selectPreset(preset)"
          :class="[ selectedPresetId === preset.id ? 'border-blue-500 bg-blue-50/70 dark:bg-blue-950/40 text-blue-900 dark:text-blue-300 ring-2 ring-blue-500/20 shadow-xs' : 'border-neutral-200 dark:border-themeBorder hover:border-neutral-300 dark:hover:border-themeBorder2 bg-neutral-50/50 dark:bg-themeBackground3/40 text-neutral-700 dark:text-themeText hover:bg-neutral-50 dark:hover:bg-themeBackground3' ]"
          class="p-3 border rounded-xl text-left transition-all flex flex-col justify-between relative group cursor-pointer"
        )
          div(class="flex items-center justify-between mb-1")
            span(class="text-xs font-bold") {{ preset.name }}
            span(class="text-[10px] font-mono font-semibold px-1.5 py-0.5 rounded bg-white dark:bg-themeBackground2 text-neutral-600 dark:text-themeText2 border border-neutral-200 dark:border-themeBorder") {{ preset.aspect }}
          div(class="text-[11px] font-mono text-neutral-500 dark:text-themeText2") {{ preset.width }} × {{ preset.height }}
          span(v-if="selectedPresetId === preset.id" class="absolute top-2.5 right-2.5 w-2 h-2 rounded-full bg-blue-600")

      //- Custom Resolution Form
      div(class="flex items-center gap-3 bg-neutral-50 dark:bg-themeBackground3/50 p-3 rounded-xl border border-neutral-200/80 dark:border-themeBorder")
        div(class="flex-1")
          label(class="text-[11px] font-semibold text-neutral-600 dark:text-themeText2 block mb-1") Custom Width (px)
          input(
            v-model.number="customWidth"
            type="number"
            min="320"
            max="7680"
            @input="onCustomDimensionChange"
            class="w-full px-3 py-1.5 bg-white dark:bg-themeBackground2 border border-neutral-300 dark:border-themeBorder rounded-lg text-xs font-mono text-neutral-800 dark:text-themeText focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors"
          )
        div(class="text-neutral-400 font-bold self-end pb-2") ×
        div(class="flex-1")
          label(class="text-[11px] font-semibold text-neutral-600 dark:text-themeText2 block mb-1") Custom Height (px)
          input(
            v-model.number="customHeight"
            type="number"
            min="320"
            max="4320"
            @input="onCustomDimensionChange"
            class="w-full px-3 py-1.5 bg-white dark:bg-themeBackground2 border border-neutral-300 dark:border-themeBorder rounded-lg text-xs font-mono text-neutral-800 dark:text-themeText focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors"
          )

    //- 2. Canvas Guides & Safe Area Overlays
    div(class="space-y-2 text-xs")
      p Saved canvas: {{ currentWidth }} × {{ currentHeight }}. {{ obsCanvas ? `OBS reports ${obsCanvas.width} × ${obsCanvas.height}.` : 'OBS canvas has not been observed.' }}
      button(v-if="obsCanvas" type="button" @click="useObsDimensions" class="underline") Use OBS dimensions for this edit
      p Applying saves the composition first. Connected OBS also receives the base-canvas change. Offline edits can be applied to OBS after reconnecting.
    div(class="space-y-2 text-xs")
      p Remote OBS pairing (same-machine OBS connects automatically)
      button(type="button" @click="copyPairingKey(false)" class="underline") Copy OBS pairing key
      button(type="button" @click="copyPairingKey(true)" class="underline ml-3") Rotate key and copy
      p(v-if="pairingMessage" role="status") {{ pairingMessage }}
      p Paste the key in OBS’s VortiDeck connection dialog. Remote traffic needs a trusted LAN or encrypted tunnel; pairing does not encrypt it.
    div(class="space-y-3 pt-3 border-t border-neutral-200/80 dark:border-themeBorder")
      label(class="text-xs font-bold uppercase tracking-wider text-neutral-500 dark:text-themeText2 flex items-center gap-1.5")
        svg(class="w-4 h-4 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z")
        | Canvas Grid & Safe Zones

      div(class="grid grid-cols-1 gap-2.5")
        //- Show Grid Toggle
        div(class="flex items-center justify-between p-3.5 bg-neutral-50 dark:bg-themeBackground3/40 border border-neutral-200/80 dark:border-themeBorder rounded-xl hover:border-neutral-300 dark:hover:border-themeBorder2 transition-colors")
          div
            span(class="text-xs font-bold text-neutral-800 dark:text-themeText block") Show Grid Overlay
            span(class="text-[11px] text-neutral-500 dark:text-themeText2") Render background pixel alignment grid
          button(
            @click="gridEnabled = !gridEnabled"
            :class="gridEnabled ? 'bg-blue-600' : 'bg-neutral-300 dark:bg-neutral-700'"
            class="w-10 h-5.5 rounded-full transition-colors relative flex items-center px-0.5 cursor-pointer"
          )
            div(:class="gridEnabled ? 'translate-x-4.5' : 'translate-x-0'" class="w-4.5 h-4.5 bg-white rounded-full transition-transform shadow-sm")

        //- Snap to Grid Toggle
        div(class="flex items-center justify-between p-3.5 bg-neutral-50 dark:bg-themeBackground3/40 border border-neutral-200/80 dark:border-themeBorder rounded-xl hover:border-neutral-300 dark:hover:border-themeBorder2 transition-colors")
          div
            span(class="text-xs font-bold text-neutral-800 dark:text-themeText block") Snap to Grid
            span(class="text-[11px] text-neutral-500 dark:text-themeText2") Snap element edges to nearest grid intersection while dragging
          button(
            @click="snapEnabled = !snapEnabled"
            :class="snapEnabled ? 'bg-blue-600' : 'bg-neutral-300 dark:bg-neutral-700'"
            class="w-10 h-5.5 rounded-full transition-colors relative flex items-center px-0.5 cursor-pointer"
          )
            div(:class="snapEnabled ? 'translate-x-4.5' : 'translate-x-0'" class="w-4.5 h-4.5 bg-white rounded-full transition-transform shadow-sm")

        //- Stream Safe Zones Toggle
        div(class="flex items-center justify-between p-3.5 bg-neutral-50 dark:bg-themeBackground3/40 border border-neutral-200/80 dark:border-themeBorder rounded-xl hover:border-neutral-300 dark:hover:border-themeBorder2 transition-colors")
          div
            span(class="text-xs font-bold text-neutral-800 dark:text-themeText block") Stream Safe Zones Overlay
            span(class="text-[11px] text-neutral-500 dark:text-themeText2") Display 16:9 Broadcast & TikTok/Reels UI safety margins
          button(
            @click="safeZonesEnabled = !safeZonesEnabled"
            :class="safeZonesEnabled ? 'bg-blue-600' : 'bg-neutral-300 dark:bg-neutral-700'"
            class="w-10 h-5.5 rounded-full transition-colors relative flex items-center px-0.5 cursor-pointer"
          )
            div(:class="safeZonesEnabled ? 'translate-x-4.5' : 'translate-x-0'" class="w-4.5 h-4.5 bg-white rounded-full transition-transform shadow-sm")

    //- 3. OBS Live Broadcast Link
    div(class="space-y-3 pt-3 border-t border-neutral-200/80 dark:border-themeBorder")
      label(class="text-xs font-bold uppercase tracking-wider text-neutral-500 dark:text-themeText2 flex items-center gap-1.5")
        svg(class="w-4 h-4 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z")
        | OBS Live Broadcast Link

      div(class="p-3.5 bg-neutral-900 dark:bg-neutral-950 text-white rounded-xl space-y-2 font-mono text-xs shadow-inner border border-neutral-800")
        div(class="flex items-center justify-between")
          span(class="text-neutral-400 font-sans text-xs") OBS Browser Source URL:
          button(
            @click="copyUrl"
            :disabled="!overlayUrl"
            :class="overlayUrl ? 'bg-blue-600 hover:bg-blue-700 cursor-pointer' : 'bg-neutral-700 cursor-not-allowed opacity-60'"
            class="px-2.5 py-1 text-white font-sans text-[11px] font-semibold rounded-md transition-colors flex items-center gap-1 shadow-sm"
          )
            svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2")
            span {{ copied ? 'Copied!' : 'Copy Link' }}

        div(class="text-blue-300 bg-neutral-950 p-2 rounded-lg border border-neutral-800 break-all select-all font-mono text-[11px]") {{ overlayUrl || 'Start the overlay server to create an authorized URL.' }}

  template(#footer)
    div(class="flex items-center justify-end gap-3 w-full")
      button(
        @click="$emit('close')"
        class="px-4 py-2 text-xs font-semibold text-neutral-600 dark:text-themeText2 hover:text-neutral-900 dark:hover:text-themeText transition-colors cursor-pointer"
      ) Cancel
      button(
        @click="applySettings"
        :disabled="busy"
        class="px-5 py-2 bg-blue-600 hover:bg-blue-700 text-white text-xs font-bold rounded-lg shadow-sm transition-all transform hover:scale-[1.01] active:scale-[0.99] cursor-pointer"
      ) Apply Settings
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import OverlayModal from "./OverlayModal.vue";
import { invoke } from '@tauri-apps/api/core';

interface Preset {
  id: string;
  name: string;
  width: number;
  height: number;
  aspect: string;
}

const props = withDefaults(
  defineProps<{
    show?: boolean;
    currentWidth?: number;
    currentHeight?: number;
    showGrid?: boolean;
    snapToGrid?: boolean;
    showSafeZones?: boolean;
    overlayUrl?: string;
    obsCanvas?: { width: number; height: number } | null;
    busy?: boolean;
  }>(),
  {
    show: false,
    currentWidth: 1920,
    currentHeight: 1080,
    showGrid: false,
    snapToGrid: false,
    showSafeZones: false,
    overlayUrl: "",
    obsCanvas: null,
    busy: false,
  }
);

const emit = defineEmits<{
  close: [];
  "apply-settings": [
    settings: {
      width: number;
      height: number;
      showGrid: boolean;
      snapToGrid: boolean;
      showSafeZones: boolean;
    }
  ];
}>();

const resolutionPresets: Preset[] = [
  { id: "1080p", name: "Desktop 1080p", width: 1920, height: 1080, aspect: "16:9" },
  { id: "1440p", name: "Desktop 1440p", width: 2560, height: 1440, aspect: "16:9" },
  { id: "4k", name: "Desktop 4K", width: 3840, height: 2160, aspect: "16:9" },
  { id: "mobile-hd", name: "TikTok / Reels", width: 1080, height: 1920, aspect: "9:16" },
  { id: "ultrawide", name: "Ultrawide 21:9", width: 3440, height: 1440, aspect: "21:9" },
  { id: "square", name: "Square 1:1", width: 1080, height: 1080, aspect: "1:1" },
];

const selectedPresetId = ref("1080p");
const customWidth = ref(props.currentWidth);
const customHeight = ref(props.currentHeight);
const gridEnabled = ref(props.showGrid);
const snapEnabled = ref(props.snapToGrid);
const safeZonesEnabled = ref(props.showSafeZones);
const copied = ref(false);
const pairingMessage = ref('');
const copyPairingKey = async (rotate: boolean) => {
  try {
    const key = await invoke<string>('obs_companion_pairing_key', { rotate });
    await navigator.clipboard.writeText(key);
    pairingMessage.value = rotate ? 'New key copied. Previously paired remote companions must use this key.' : 'Pairing key copied.';
  } catch {
    pairingMessage.value = 'Could not copy the pairing key. Check clipboard access and try again.';
  }
};
const useObsDimensions = () => {
  if (!props.obsCanvas) return;
  customWidth.value = props.obsCanvas.width;
  customHeight.value = props.obsCanvas.height;
  onCustomDimensionChange();
};

watch(
  () => props.show,
  (newVal) => {
    if (newVal) {
      customWidth.value = props.currentWidth;
      customHeight.value = props.currentHeight;
      gridEnabled.value = props.showGrid;
      snapEnabled.value = props.snapToGrid;
      safeZonesEnabled.value = props.showSafeZones;

      const matched = resolutionPresets.find(
        (p) => p.width === props.currentWidth && p.height === props.currentHeight
      );
      selectedPresetId.value = matched ? matched.id : "custom";
    }
  }
);

const selectPreset = (preset: Preset) => {
  selectedPresetId.value = preset.id;
  customWidth.value = preset.width;
  customHeight.value = preset.height;
};

const onCustomDimensionChange = () => {
  const matched = resolutionPresets.find(
    (p) => p.width === customWidth.value && p.height === customHeight.value
  );
  selectedPresetId.value = matched ? matched.id : "custom";
};

const copyUrl = async () => {
  if (!props.overlayUrl) return;

  try {
    await navigator.clipboard.writeText(props.overlayUrl);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch (e) {
    console.warn("Failed to copy URL:", e);
  }
};

const applySettings = () => {
  emit("apply-settings", {
    width: customWidth.value || 1920,
    height: customHeight.value || 1080,
    showGrid: gridEnabled.value,
    snapToGrid: snapEnabled.value,
    showSafeZones: safeZonesEnabled.value,
  });
  emit("close");
};
</script>
