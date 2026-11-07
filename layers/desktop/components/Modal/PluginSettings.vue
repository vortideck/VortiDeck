<template lang="pug">
BaseModal(:is-open="isOpen" shell-class="scrollbar-visible" @close="handleClose")
  div(class="vd-modal-card relative w-full max-w-[50rem] max-h-[88vh] flex flex-col rounded-3xl overflow-hidden bg-themeBackground border border-themeBorder shadow-2xl")
    // Header
    div(class="p-6 pb-4 flex items-center justify-between border-b border-themeBorder bg-themeBackground2/40")
      div(class="flex items-center gap-4 min-w-0")
        img(
          :src="plugin?.icon"
          :alt="plugin?.name"
          class="w-12 h-12 rounded-2xl object-cover border border-themeBorder shrink-0 shadow-xs"
          @error="handleIconError"
        )
        div(class="min-w-0")
          h3(class="text-lg font-bold text-themeText tracking-tight truncate") {{ plugin?.name }} Package
          div(class="flex items-center gap-2 mt-1")
            span(class="text-[10px] font-bold px-2 py-0.5 rounded-full uppercase tracking-wider bg-themePrimary/12 text-themePrimary border border-themePrimary/20") Installed
            span(v-if="plugin?.version && plugin.version !== 'undefined'" class="text-[11px] font-mono font-medium text-themeText3 bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder") v{{ plugin?.version }}
      button(@click="handleClose" class="p-2 rounded-xl text-themeText2 hover:text-themeText hover:bg-themeBackground border border-transparent hover:border-themeBorder transition-colors cursor-pointer shrink-0")
        span(class="sr-only") Close
        svg(class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
    
    // Body
    div(class="p-6 overflow-y-auto flex-1")
      div(class="grid grid-cols-1 lg:grid-cols-3 gap-6")
        // Left column - Settings
        div(class="lg:col-span-2 space-y-5")
          // Plugin Status
          div
            h4(class="text-xs font-bold uppercase tracking-wider text-themeText3 mb-2") Plugin Status
            div(class="bg-themeBackground2 p-4 rounded-2xl border border-themeBorder shadow-xs")
              div(class="flex items-center justify-between")
                div
                  h5(class="font-bold text-sm text-themeText") Runtime State
                  p(class="text-xs text-themeText3 mt-0.5") {{ runtimeDescription }}
                div(class="flex items-center gap-2 bg-themeBackground px-2.5 py-1 rounded-full border border-themeBorder text-xs")
                  div(class="w-2 h-2 rounded-full" :class="runtimeStatusClass")
                  span(class="font-bold text-[11px]" :class="runtimeTextClass") {{ runtimeStatusLabel }}
          
          // Permissions & Capabilities
          div(v-if="pkg")
            h4(class="text-xs font-bold uppercase tracking-wider text-themeText3 mb-2") Permissions &amp; Capabilities
            div(class="bg-themeBackground2 p-4 rounded-2xl border border-themeBorder")
              p(v-if="accessItems.length === 0" class="text-xs text-themeText3 italic") No special access requested.
              ul(v-else class="space-y-1.5")
                li(
                  v-for="item in accessItems"
                  :key="item.label"
                  class="flex items-start gap-2 text-xs"
                )
                  span(
                    class="shrink-0 mt-0.5 font-bold"
                    :class="item.risk === 'high' ? 'text-amber-400' : 'text-emerald-400'"
                  ) {{ item.risk === 'high' ? '⚠' : '✓' }}
                  span(
                    :class="item.risk === 'high' ? 'text-amber-400 font-semibold' : 'text-themeText2'"
                  ) {{ item.label }}

        // Right column - Plugin info & actions
        div(class="space-y-4")
          // Plugin Information Card
          div(class="bg-themeBackground2 p-4 rounded-2xl border border-themeBorder space-y-2.5 text-xs shadow-xs")
            h5(class="font-bold text-xs text-themeText mb-2") Package Info
            div(class="flex justify-between")
              span(class="text-themeText3") Version
              span(class="text-themeText font-semibold") {{ plugin?.version || '1.0.0' }}
            
            div(class="flex justify-between")
              span(class="text-themeText3") Developer
              span(class="text-themeText font-semibold truncate max-w-[120px]") {{ plugin?.developer || plugin?.author || 'VortiDeck' }}
            
            div(class="flex justify-between")
              span(class="text-themeText3") Category
              span(class="text-themeText font-semibold capitalize") {{ plugin?.category || 'Utility' }}
            
            div(class="flex justify-between")
              span(class="text-themeText3") Type
              span(class="text-themeText font-semibold") {{ packageTypeLabel }}

          // Action Buttons Card
          div(class="bg-themeBackground2 p-4 rounded-2xl border border-themeBorder space-y-2 shadow-xs")
            button(
              v-if="!isBuiltin"
              @click="uninstallPlugin"
              class="w-full py-2 px-3 bg-red-500/10 hover:bg-red-500/20 text-red-500 border border-red-500/20 rounded-xl text-xs font-bold transition-colors cursor-pointer"
            )
              span Uninstall Package

    // Footer
    div(class="px-6 py-3.5 bg-themeBackground2/70 border-t border-themeBorder flex justify-end items-center")
      button(@click="handleClose" class="px-4 py-1.5 bg-themeBackground border border-themeBorder hover:bg-themeBackground3 text-themeText rounded-xl text-xs font-semibold transition-colors cursor-pointer") Close
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { computed } from "vue";
import {
  flattenPackageAccess,
  type PackageInfo,
} from "../../../desktop-streaming/composables/overlay/packageManifest";

interface Plugin {
  id: string;
  name: string;
  icon: string;
  version: string;
  developer?: string;
  author?: string;
  category: string;
  lastUpdated: string;
  installDate?: string;
}

const props = defineProps<{
  isOpen: boolean;
  plugin?: Plugin;
  /** Unified package info — drives the Permissions & Capabilities section. */
  pkg?: PackageInfo;
}>();

const emit = defineEmits(["close", "uninstall"]);

// Permissions & capabilities flattened from the package (legacy runtime perms +
// widget capabilities). Same list the CapabilityDialog shows at install, so the
// dialog and per-plugin settings never drift.
const accessItems = computed(() =>
  flattenPackageAccess(props.pkg ?? { contributions: [] }),
);
const isGranted = computed(() => !!props.pkg?.is_granted);
const hasRuntime = computed(() => !!props.pkg?.has_runtime);
const isBuiltin = computed(() => props.pkg?.trust === "builtin");
const runtimeStatusLabel = computed(() => {
  if (!hasRuntime.value) return "No runtime";
  if (props.pkg?.runtime_active) return "Active";
  if (isGranted.value) return "Inactive";
  return "Not installed";
});
const runtimeDescription = computed(() => {
  if (!hasRuntime.value)
    return "This package contributes UI or widgets without an executable runtime.";
  if (props.pkg?.runtime_active)
    return "The package runtime is loaded in the current VortiDeck process.";
  if (isGranted.value)
    return "Access is granted, but the runtime is not currently loaded.";
  return "Review and grant package access before its runtime can start.";
});
const runtimeStatusClass = computed(() =>
  props.pkg?.runtime_active ? "bg-themeSuccess" : "bg-themeText3",
);
const runtimeTextClass = computed(() =>
  props.pkg?.runtime_active ? "text-themeSuccess" : "text-themeText2",
);
const packageTypeLabel = computed(() => {
  const kinds = new Set(
    (props.pkg?.contributions ?? []).map((item) => item.type),
  );
  if (hasRuntime.value) kinds.add("runtime");
  return kinds.size ? [...kinds].join(" · ") : "Package";
});

const handleClose = () => {
  emit("close");
};

const uninstallPlugin = () => {
  if (isBuiltin.value) return;
  // Confirmation and transactional progress belong to the package-store
  // owner. Keeping one confirmation surface avoids native dialog capability
  // dependencies and duplicate uninstall state machines.
  emit("uninstall", props.plugin);
};

const handleIconError = (event: Event) => {
  const img = event.target as HTMLImageElement;
  const initials = encodeURIComponent(
    (props.plugin?.name || "PLG").substring(0, 3).toUpperCase(),
  );
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><rect width="100" height="100" rx="20" fill="%232c333f"/><text x="50" y="58" font-family="system-ui, sans-serif" font-size="28" font-weight="bold" fill="%23e6edf3" text-anchor="middle">${initials}</text></svg>`;
  img.src = `data:image/svg+xml;utf8,${svg}`;
};
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;
</style>
