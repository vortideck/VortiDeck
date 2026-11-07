<template lang="pug">
  div(class="h-full w-full")
      div(v-if="!isNative" id="deck"  :style="theme.backgroundCss" :class="isLandscape ? 'flex-col' : 'flex-row'" class="relative h-full flex justify-between items-center w-full " @pointerdown="$emit('grid-pointerdown', $event)")
          DeckSettings(:isDeckSettingsOpen="isDeckSettingsOpen" :fillPage="false" :isLandscape="!isLandscape" class="z-[45]")
          div(:class="isLandscape ? 'deck-floating-controls--horizontal' : 'deck-floating-controls--vertical'" class="deck-floating-controls")
            button(type="button" title="Deck settings" aria-label="Deck settings" @click="isDeckSettingsOpen = !isDeckSettingsOpen")
              IconParkOutlineConfig(class="w-8 h-8 text-[#a6a6a6]")
            button(
              v-if="showWidgetPalette"
              type="button"
              class="grid h-8 w-8 place-items-center rounded-lg border border-blue-400/40 bg-blue-500/15 text-xl font-semibold text-blue-300 hover:bg-blue-500/30"
              title="Add custom widget"
              aria-label="Add custom widget"
              @click="$emit('open-widget-palette', $event)"
            ) +
            button(v-if="deckStore.isTipIconInDeck" type="button" title="Toggle alternate actions" aria-label="Toggle alternate actions" @click="deckStore.setIsAlt(!deckStore.getIsAlt)")
              Fa6SolidCircleInfo(:class="deckStore.getIsAlt ? 'w-8 h-8 text-blue-400 opacity-100 scale-110 transition-all' : 'w-8 h-8 text-blue-400 opacity-50 hover:opacity-80 transition-all'")
          div(v-if="squareGridTracks" class="deck-grid-viewport grid min-h-0 min-w-0 flex-1 self-stretch place-items-center")
            div(:style="squareDesktopGridStyle" data-deck-widget-grid class="p-3 grid place-items-center relative min-w-0 min-h-0 overflow-visible")
              slot(name="page")
          div(v-else :style="`display: grid; grid-template-columns: repeat(${desktopGridColumns}, minmax(0, 1fr)); grid-template-rows: repeat(${desktopGridRows}, minmax(0, 1fr)); gap: ${gridSettings?.button_spacing || 8}px; width: ${desktopWidth}; height: ${desktopHeight}`" data-deck-widget-grid class="p-3 grid place-items-center relative min-w-0 min-h-0 overflow-visible")
            slot(name="page")
      div(v-else id="deck" :style="theme.backgroundCss" :class="orientation === 'portrait' ? 'flex-col' : 'flex-row'" class="relative h-full flex items-center justify-between w-full")
          DeckSettings(:isDeckSettingsOpen="isDeckSettingsOpen" :fillPage="false" :isLandscape="isLandscapeMobile" class="z-[45]")
          div(:class="orientation === 'portrait' ? 'deck-floating-controls--horizontal' : 'deck-floating-controls--vertical'" class="deck-floating-controls")
            button(type="button" aria-label="Deck settings" @click="isDeckSettingsOpen = !isDeckSettingsOpen")
              IconParkOutlineConfig(class="w-8 h-8 text-[#a6a6a6]")
            button(v-if="deckStore.isTipIconInDeck" type="button" aria-label="Toggle alternate actions" @click="deckStore.setIsAlt(!deckStore.getIsAlt)")
              Fa6SolidCircleInfo(:class="deckStore.getIsAlt ? 'w-8 h-8 text-blue-400 opacity-100 scale-110 transition-all' : 'w-8 h-8 text-blue-400 opacity-50 hover:opacity-80 transition-all'")

          
          div(:style="`display: grid; grid-template-columns: repeat(${orientation === 'portrait' ? (gridSettings?.grid || '4,4').split(',')[0] : (gridSettings?.grid || '4,4').split(',')[1]}, minmax(0, 1fr)); grid-template-rows: repeat(${orientation === 'portrait' ? (gridSettings?.grid || '4,4').split(',')[1] : (gridSettings?.grid || '4,4').split(',')[0]}, minmax(0, 1fr)); gap: ${gridSettings?.button_spacing || 8}px; width: ${mobileWidth};  height: ${mobileHeight}`" class="p-4 grid place-items-center relative min-w-0 min-h-0 overflow-visible" @pointerdown="$emit('grid-pointerdown', $event)")

            slot(name="page")
      div(class="cursor cursor--small")
      
</template>

<script setup lang="ts">
import Fa6SolidCircleInfo from "~icons/fa6-solid/circle-info?width=16px&height=16px";

import IconParkOutlineConfig from "~icons/icon-park-outline/config?width=48px&height=48px";
import { onMounted, onUnmounted } from "vue";
import { useDeckStore } from "@/stores/Deck";
import type { DeckSettings } from "@base/types/deck";

const deckStore = useDeckStore();

const theme = computed(() => {
  return deckStore.getActiveTheme;
});

const isDeckSettingsOpen = ref(false);

const props = withDefaults(defineProps<{
  isLandscape?: boolean;
  isNative?: boolean;
  showWidgetPalette?: boolean;
  squareGridTracks?: boolean;
  gridSettings?: DeckSettings;
}>(), {
  isLandscape: false,
  isNative: false,
  showWidgetPalette: false,
  squareGridTracks: false,
  gridSettings: undefined,
});

defineEmits<{
  (event: "open-widget-palette", mouseEvent: MouseEvent): void;
  (event: "grid-pointerdown", pointerEvent: PointerEvent): void;
}>();

const effectiveSettings = computed(() => props.gridSettings ?? deckStore.activeDeck?.settings);

const fillPage = computed(() => {
  return effectiveSettings.value?.maximize_button_space ?? false;
});

type Orientation = "portrait" | "landscape";
const orientation = ref<Orientation>("portrait");
const isLandscapeMobile = computed(() => {
  return orientation.value === "landscape";
});

const updateOrientation = () => {
  orientation.value = window.screen.orientation.type.startsWith("landscape")
    ? "landscape"
    : "portrait";
};

onMounted(() => {
  updateOrientation();
  window.screen.orientation.addEventListener("change", updateOrientation);
});

onUnmounted(() => {
  window.screen.orientation.removeEventListener("change", updateOrientation);
});

const firstGridDimension = computed(() => {
  const first = Number.parseInt(effectiveSettings.value?.grid.split(",", 1)[0] ?? "4", 10);
  return Number.isFinite(first) && first > 0 ? first : 4;
});

const secondGridDimension = computed(() => {
  const second = Number.parseInt(effectiveSettings.value?.grid.split(",")[1] ?? "4", 10);
  return Number.isFinite(second) && second > 0 ? second : 4;
});

const desktopGridColumns = computed(() => (
  props.isLandscape ? firstGridDimension.value : secondGridDimension.value
));
const desktopGridRows = computed(() => (
  props.isLandscape ? secondGridDimension.value : firstGridDimension.value
));

const squareDesktopGridStyle = computed(() => {
  const columns = desktopGridColumns.value;
  const rows = desktopGridRows.value;
  const gap = Number(effectiveSettings.value?.button_spacing ?? 8);
  const outerMargin = fillPage.value
    ? 0
    : Math.max(0, Number(effectiveSettings.value?.grid_margin ?? 0));
  const padding = 12;
  const horizontalFixed = padding * 2 + gap * Math.max(0, columns - 1);
  const verticalFixed = padding * 2 + gap * Math.max(0, rows - 1);
  const widthFromHeightRatio = columns / rows;
  const heightFromWidthRatio = rows / columns;
  const base = {
    display: "grid",
    gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
    gridTemplateRows: `repeat(${rows}, minmax(0, 1fr))`,
    gap: `${gap}px`,
  };
  return {
    ...base,
    width: `min(calc(100cqw - ${outerMargin * 2}px), calc(${100 * widthFromHeightRatio}cqh + ${horizontalFixed - (verticalFixed + outerMargin * 2) * widthFromHeightRatio}px))`,
    height: `min(calc(100cqh - ${outerMargin * 2}px), calc(${100 * heightFromWidthRatio}cqw + ${verticalFixed - (horizontalFixed + outerMargin * 2) * heightFromWidthRatio}px))`,
  };
});

const desktopHeight = computed(() => {
  if (props.isLandscape === false && firstGridDimension.value > 2) {
    return "100%";
  } else if (firstGridDimension.value <= 2 && props.isLandscape === true) {
    return "100%";
  } else if (fillPage.value === false && props.isLandscape === false) {
    return "auto";
  } else if (fillPage.value === true && props.isLandscape === true) {
    return "100%";
  } else {
    return "auto";
  }
});

const desktopWidth = computed(() => {
  if (props.isLandscape === true && firstGridDimension.value > 2) {
    return "100%";
  } else if (firstGridDimension.value <= 2 && props.isLandscape === false) {
    return "100%";
  } else if (fillPage.value === false && props.isLandscape === false) {
    return "auto";
  } else if (fillPage.value === true && props.isLandscape === false) {
    return "100%";
  } else {
    return "auto";
  }
});

const mobileHeight = computed(() => {
  if (
    isLandscapeMobile.value === true &&
    firstGridDimension.value > 2 &&
    fillPage.value === true
  ) {
    return "100%";
  } else if (
    isLandscapeMobile.value === false &&
    firstGridDimension.value > 2 &&
    fillPage.value === false
  ) {
    return "auto";
  } else if (
    firstGridDimension.value <= 2 &&
    isLandscapeMobile.value === false
  ) {
    return "100%";
  } else if (
    firstGridDimension.value <= 2 &&
    isLandscapeMobile.value === true
  ) {
    return "auto";
  } else if (fillPage.value === false && isLandscapeMobile.value === false) {
    return "auto";
  } else if (fillPage.value === true && isLandscapeMobile.value === false) {
    return "100%";
  } else {
    return "100%";
  }
});

const mobileWidth = computed(() => {
  if (isLandscapeMobile.value === false && firstGridDimension.value > 2) {
    return "100%";
  } else if (
    firstGridDimension.value <= 2 &&
    isLandscapeMobile.value === true
  ) {
    return "100%";
  } else if (
    firstGridDimension.value <= 2 &&
    isLandscapeMobile.value === false
  ) {
    return "auto";
  } else if (fillPage.value === false && isLandscapeMobile.value === true) {
    return "auto";
  } else if (fillPage.value === true && isLandscapeMobile.value === true) {
    return "100%";
  } else {
    return "auto";
  }
});
</script>

<style lang="sass" scoped>





.deck-grid-viewport
    container-type: size

.deck-floating-controls
    position: absolute
    z-index: 50
    display: flex
    justify-content: space-between
    color: white
    pointer-events: none

    > button
        pointer-events: auto

.deck-floating-controls--vertical
    inset-block: 1.5rem
    inset-inline-start: 1rem
    flex-direction: column

.deck-floating-controls--horizontal
    inset-block-start: 1rem
    inset-inline: 1.5rem
    flex-direction: row

.app-container

    z-index: 10
    display: flex
    flex-direction: column
    justify-content: center



.panel
    margin-top: 60px
    text-align: center
    margin: 0 auto
    scroll-snap-align: center

    width: 90%
    p.title

        font-style: normal
        font-size: 5vh
        line-height: 0%
        align-self: center
        letter-spacing: 0.18em





.cursor
  position: fixed
  left: 0
  top: 0
  pointer-events: none

.cursor--small
  width: 5px
  height: 5px
  left: -2.5px
  top: -2.5px
  border-radius: 50%
  z-index: 11000
  background: var(--color-text)

.cursor--canvas
  width: 100vw
  height: 100vh
  z-index: 12000

.hidden-overlay
  visibility: hidden
  opacity: 0

.visible-overlay
  visibility: visible
  opacity: 0.4
</style>
