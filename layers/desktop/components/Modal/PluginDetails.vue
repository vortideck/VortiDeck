<template lang="pug">
BaseModal(:is-open="isOpen" shell-class="scrollbar-visible" @close="handleClose")
  div(class="vd-modal-card relative w-full max-w-[36rem] max-h-[88vh] flex flex-col rounded-3xl overflow-hidden bg-themeBackground border border-themeBorder shadow-2xl")
    // Header
    div(class="p-6 pb-4 flex items-start justify-between gap-4 border-b border-themeBorder/40 bg-themeBackground2/40")
      div(class="flex items-center gap-4 min-w-0")
        img(
          :src="plugin?.icon"
          :alt="plugin?.name"
          class="w-14 h-14 rounded-2xl object-cover border border-themeBorder shrink-0 shadow-xs"
          @error="handleIconError"
        )
        div(class="min-w-0 flex flex-col gap-y-1")
          h3(class="text-lg font-bold text-themeText tracking-tight truncate") {{ plugin?.name }}
          div(class="flex flex-wrap items-center gap-2 text-xs text-themeText3")
            span(class="px-2 py-0.5 rounded-full text-[10px] uppercase font-bold tracking-wider bg-themePrimary/12 text-themePrimary border border-themePrimary/20") {{ plugin?.category || 'Plugin' }}
            span(v-if="plugin?.version && plugin.version !== 'undefined'" class="font-mono text-[11px] text-themeText2") v{{ plugin?.version }}
            span(v-if="plugin?.developer || plugin?.author" class="truncate max-w-[140px]") by {{ plugin?.developer || plugin?.author }}

      button(@click="handleClose" class="p-2 rounded-xl text-themeText2 hover:text-themeText hover:bg-themeBackground border border-transparent hover:border-themeBorder transition-colors cursor-pointer shrink-0")
        span(class="sr-only") Close
        svg(class="h-4.5 w-4.5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
    
    // Body (Scrollable)
    div(class="p-6 overflow-y-auto flex-1 flex flex-col gap-y-5")
      // Action & Stats Ribbon
      div(class="bg-themeBackground2 p-4 rounded-2xl border border-themeBorder shadow-xs flex flex-col sm:flex-row items-center justify-between gap-4")
        div(class="flex items-center gap-3 w-full sm:w-auto")
          div(class="flex flex-col")
            span(class="text-[10px] uppercase font-bold tracking-wider text-themeText3") Price
            span(class="text-base font-extrabold text-themeText") {{ plugin?.price || 'Free' }}
          button(
            @click="handleAction"
            class="flex-1 sm:flex-initial px-5 py-2.5 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-bold transition-all shadow-xs ml-auto cursor-pointer"
          )
            span(v-if="isInstalled") ✓ Configured
            span(v-else-if="plugin?.price && plugin.price !== 'Free' && plugin.price !== 0") Purchase Plugin
            span(v-else) Install Plugin

        div(class="flex items-center gap-3 text-xs text-themeText2 w-full sm:w-auto justify-around sm:justify-end sm:border-l sm:border-themeBorder/60 sm:pl-4")
          div(class="flex flex-col items-center sm:items-start")
            span(class="text-[10px] text-themeText3 font-medium") Rating
            span(class="font-semibold text-amber-400 flex items-center gap-1")
              span ★
              span {{ plugin?.rating || 4.5 }}/5
          div(class="flex flex-col items-center sm:items-start")
            span(class="text-[10px] text-themeText3 font-medium") Downloads
            span(class="font-semibold text-themeText") {{ formatNumber(plugin?.downloads) }}
          div(class="flex flex-col items-center sm:items-start" v-if="plugin?.lastUpdated && plugin.lastUpdated !== 'undefined'")
            span(class="text-[10px] text-themeText3 font-medium") Updated
            span(class="font-semibold text-themeText") {{ plugin?.lastUpdated }}

      // Overview / Description
      div(class="flex flex-col gap-y-1.5")
        h4(class="text-[11px] font-bold uppercase tracking-wider text-themeText3") Description
        p(class="text-xs text-themeText2 leading-relaxed bg-themeBackground2/60 p-3.5 rounded-xl border border-themeBorder/60") {{ plugin?.description }}
          
      // Permissions & Capabilities
      div(v-if="pkg" class="flex flex-col gap-y-1.5")
        h4(class="text-[11px] font-bold uppercase tracking-wider text-themeText3") Permissions &amp; Capabilities
        div(class="bg-themeBackground2 p-3.5 rounded-xl border border-themeBorder/60")
          p(v-if="accessItems.length === 0" class="text-xs text-themeText2 italic") No special access requested — runs sandboxed.
          ul(v-else class="space-y-1.5")
            li(
              v-for="item in accessItems"
              :key="item.label"
              class="flex items-start gap-2 text-xs"
            )
              span(
                class="shrink-0 mt-0.5 text-xs font-bold"
                :class="item.risk === 'high' ? 'text-amber-400' : 'text-emerald-400'"
              ) {{ item.risk === 'high' ? '⚠' : '✓' }}
              span(
                :class="item.risk === 'high' ? 'text-amber-400 font-semibold' : 'text-themeText2'"
              ) {{ item.label }}

      // Key Features
      div(v-if="plugin?.features && plugin.features.length" class="flex flex-col gap-y-1.5")
        h4(class="text-[11px] font-bold uppercase tracking-wider text-themeText3") Features
        div(class="flex flex-wrap gap-1.5")
          span(
            v-for="feature in plugin.features"
            :key="feature"
            class="px-2.5 py-1 rounded-lg text-xs font-medium bg-themeBackground2 border border-themeBorder text-themeText2"
          ) {{ feature }}
      
      // Screenshots
      div(v-if="plugin?.screenshots?.length" class="flex flex-col gap-y-1.5")
        h4(class="text-[11px] font-bold uppercase tracking-wider text-themeText3") Screenshots
        div(class="grid grid-cols-2 gap-3")
          img(
            v-for="(screenshot, index) in plugin.screenshots"
            :key="index"
            :src="screenshot"
            :alt="`Screenshot ${index + 1}`"
            class="w-full h-28 object-cover rounded-xl border border-themeBorder"
          )
      
      // Changelog
      div(v-if="plugin?.changelog" class="flex flex-col gap-y-1.5")
        h4(class="text-[11px] font-bold uppercase tracking-wider text-themeText3") What's New
        div(class="bg-themeBackground2 p-3.5 rounded-xl border border-themeBorder")
          p(class="text-xs text-themeText2") {{ plugin.changelog }}

    // Footer
    div(class="px-6 py-3.5 bg-themeBackground2/70 border-t border-themeBorder flex justify-end items-center")
      button(@click="handleClose" class="px-4 py-1.5 bg-themeBackground border border-themeBorder hover:bg-themeBackground3 text-themeText rounded-xl text-xs font-semibold transition-colors cursor-pointer") Close
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { computed } from "vue";
import { flattenPackageAccess, type PackageInfo } from "../../../desktop-streaming/composables/overlay/packageManifest";

interface Plugin {
  id: string;
  name: string;
  description: string;
  icon: string;
  category: string;
  price?: string | number;
  originalPrice?: string;
  features?: string[];
  screenshots?: string[];
  changelog?: string;
  downloads?: number;
  rating?: number;
  lastUpdated?: string;
  version?: string;
  developer?: string;
  author?: string;
  supportUrl?: string;
  websiteUrl?: string;
  featured?: boolean;
  verified?: boolean;
  trialAvailable?: boolean;
  installed?: boolean;
}

const props = defineProps<{
  isOpen: boolean;
  plugin?: Plugin;
  pkg?: PackageInfo;
}>();

const accessItems = computed(() =>
  flattenPackageAccess(props.pkg ?? { contributions: [] })
);

const emit = defineEmits(["close", "install", "configure", "purchase"]);

const isInstalled = computed(() => {
  return !!props.plugin?.installed;
});

const handleClose = () => {
  emit("close");
};

const handleAction = () => {
  if (isInstalled.value) {
    emit("configure");
  } else if (props.plugin?.price && props.plugin.price !== "Free" && props.plugin.price !== 0) {
    emit("purchase");
  } else {
    emit("install");
  }
};

const handleIconError = (event: Event) => {
  const img = event.target as HTMLImageElement;
  const initials = encodeURIComponent((props.plugin?.name || "PLG").substring(0, 3).toUpperCase());
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><rect width="100" height="100" rx="20" fill="%232c333f"/><text x="50" y="58" font-family="system-ui, sans-serif" font-size="28" font-weight="bold" fill="%23e6edf3" text-anchor="middle">${initials}</text></svg>`;
  img.src = `data:image/svg+xml;utf8,${svg}`;
};

const formatNumber = (num?: number) => {
  if (!num) return "0";
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
};
</script>
