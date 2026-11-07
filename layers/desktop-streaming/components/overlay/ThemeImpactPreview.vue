<template lang="pug">
div(
  v-if="theme"
  :style="containerStyle"
  class="my-1 p-2.5 rounded-xl border flex flex-col gap-2 shadow-inner font-sans transition-all duration-200"
)
  //- 1. Follower Alert Box
  div(
    :style="alertStyle"
    class="p-1.5 rounded-lg border flex items-center justify-between shadow-xs"
  )
    div(class="flex items-center gap-1.5")
      span(
        :style="markStyle"
        class="grid h-6 w-6 place-items-center rounded-md border text-[8px] font-extrabold tracking-wider"
      ) NEW
      div
        p(:style="textPrimaryStyle" class="font-bold text-[10px]") New Follower!
        p(:style="textMutedStyle" class="text-[9px]") Alex_Streamer
    span(
      :style="badgeStyle"
      class="px-1.5 py-0.5 rounded text-[8px] font-extrabold uppercase tracking-wider"
    ) ALERT

  //- 2. Goal Progress Bar
  div(class="space-y-1")
    div(class="flex justify-between text-[9px] font-bold")
      span(:style="textPrimaryStyle") Sub Goal
      span(:style="textAccentStyle") 420 / 500
    div(
      :style="trackStyle"
      class="w-full h-1.5 rounded-full overflow-hidden"
    )
      div(
        :style="progressFillStyle"
        class="h-full rounded-full w-[84%]"
      )

  //- 3. Chat Message
  div(
    :style="chatBoxStyle"
    class="p-1.5 rounded-lg text-[9px]"
  )
    span(:style="textAccentStyle" class="font-bold mr-1") Gamer42:
    span(:style="textPrimaryStyle") Stream looks sharp and consistent.
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { ThemeDef } from "../../composables/overlay/useOverlayTheme";

const props = defineProps<{
  theme: ThemeDef | null | undefined;
}>();

// Map colors to exact useOverlayTheme.buildCssVars specification:
// colors[0] = --theme-primary
// colors[1] = --theme-secondary
// colors[2] = --theme-accent
// colors[3] = --theme-bg
// colors[4] = --theme-text
// colors[5] = --theme-surface
// colors[6] = --theme-muted

const primaryAccent = computed(() => props.theme?.colors?.[0] || "#a855f7");
const secondaryAccent = computed(() => props.theme?.colors?.[1] || "#ec4899");
const highlightAccent = computed(() => props.theme?.colors?.[2] || primaryAccent.value);
const mainBg = computed(() => props.theme?.colors?.[3] || "#0a0a0a");
const textColor = computed(() => props.theme?.colors?.[4] || "#ffffff");
const surfaceBg = computed(() => props.theme?.colors?.[5] || mainBg.value);
const mutedText = computed(() => props.theme?.colors?.[6] || "#a1a1aa");

const containerStyle = computed(() => ({
  backgroundColor: surfaceBg.value,
  borderColor: primaryAccent.value + "50",
}));

const alertStyle = computed(() => ({
  backgroundColor: mainBg.value,
  borderColor: primaryAccent.value + "40",
}));

const badgeStyle = computed(() => ({
  backgroundColor: primaryAccent.value,
  color: "#ffffff",
}));

const markStyle = computed(() => ({
  backgroundColor: primaryAccent.value + "18",
  borderColor: primaryAccent.value + "55",
  color: primaryAccent.value,
}));

const textPrimaryStyle = computed(() => ({
  color: textColor.value,
}));

const textMutedStyle = computed(() => ({
  color: mutedText.value,
}));

const textAccentStyle = computed(() => ({
  color: primaryAccent.value,
}));

const trackStyle = computed(() => ({
  backgroundColor: mainBg.value,
  borderColor: primaryAccent.value + "30",
}));

const progressFillStyle = computed(() => ({
  background: `linear-gradient(90deg, ${primaryAccent.value}, ${secondaryAccent.value})`,
}));

const chatBoxStyle = computed(() => ({
  backgroundColor: mainBg.value,
  borderColor: primaryAccent.value + "30",
}));
</script>
