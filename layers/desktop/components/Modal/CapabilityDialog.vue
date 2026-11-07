<template lang="pug">
BaseModal(:is-open="show" @close="!busy && $emit('cancel')")
  div(class="vd-modal-card relative w-full max-w-md max-h-[85vh] flex flex-col")

    //- Header
    div(class="flex items-start justify-between gap-4 p-6 border-b border-themeBorder2")
      div(class="min-w-0")
        h3(class="text-xl font-bold text-themeText tracking-tight truncate") {{ manifest.name }}
        p(class="text-sm text-themeText2 mt-0.5 truncate") by {{ manifest.author || 'unknown' }}
      button(@click="$emit('cancel')" :disabled="busy" class="vd-modal-close-btn shrink-0 disabled:opacity-40 disabled:cursor-not-allowed" aria-label="Close")
        svg(class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

    //- Body
    div(class="p-6 overflow-y-auto flex-1")
      //- Trust badge + install count
      div(v-if="trustBadge || installs != null" class="flex items-center gap-2 flex-wrap mb-4")
        span(
          v-if="trustBadge"
          class="inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium"
          :class="trustBadge.class"
        ) {{ trustBadge.label }}
        span(v-if="installs != null" class="inline-flex items-center gap-1 px-2 py-1 text-xs text-themeText2")
          | ★ {{ installs.toLocaleString() }} installs

      p(class="text-sm font-medium text-themeText mb-3") This package wants to:
      ul(class="space-y-2")
        li(
          v-for="cap in requestedCapabilities"
          :key="cap.label"
          class="flex items-start gap-2.5 text-sm"
        )
          span(class="shrink-0 mt-0.5" :class="cap.risk === 'high' ? 'text-themeWarningIcon' : 'text-themeSuccess'")
            | {{ cap.risk === 'high' ? '⚠' : '✓' }}
          span(:class="cap.risk === 'high' ? 'text-themeWarningIcon font-medium' : 'text-themeText2'") {{ cap.label }}
      p(
        v-if="!requestedCapabilities.length"
        class="text-sm text-themeText2 italic"
      ) No special access requested — it just renders.

      //- Reassurance
      div(class="mt-5 p-3 rounded-lg border border-themeBorder2 flex items-center gap-2" style="background: hsl(var(--background2) / 0.6);")
        svg(class="w-4 h-4 shrink-0 text-themeText2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z")
        span(class="text-xs text-themeText2") Runs sandboxed — it can't see the editor, other widgets, or your accounts.

      div(v-if="busy" class="mt-4 p-3 rounded-lg border border-themeButton/25 bg-themeButton/10 flex items-center gap-3")
        span(class="h-4 w-4 shrink-0 rounded-full border-2 border-themeButton border-t-transparent animate-spin")
        div(class="min-w-0 flex-1")
          p(class="text-sm font-medium text-themeText") Installing and compiling…
          div(class="flex items-center justify-between gap-3 mt-0.5")
            p(class="text-xs text-themeText2 truncate") {{ status || 'Preparing package runtime…' }}
            span(v-if="progress === 100" class="text-xs text-themeButton font-semibold") Ready
            span(v-else class="text-xs text-themeButton font-semibold") Working
          div(class="mt-2 h-1.5 overflow-hidden rounded-full bg-themeBackground3 border border-themeBorder2")
            div(
              class="h-full w-full rounded-full bg-themeButton animate-pulse"
            )

      div(v-else-if="error" class="mt-4 p-3 rounded-lg border border-themeError/25 bg-themeError/10")
        p(class="text-sm font-medium text-themeError") Installation failed
        p(class="text-xs text-themeText2 mt-1 break-words") {{ error }}

    //- Footer
    div(class="vd-modal-footer flex items-center justify-end gap-3 p-6")
      button(
        @click="$emit('cancel')"
        :disabled="busy"
        class="px-4 py-2 rounded-lg text-sm font-medium text-themeText2 hover:text-themeText transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
      ) Cancel
      button(
        @click="$emit('grant')"
        :disabled="busy"
        class="vd-modal-primary-btn px-4 py-2 text-sm font-medium disabled:opacity-70 disabled:cursor-wait inline-flex items-center gap-2"
      )
        span(v-if="busy" class="h-3.5 w-3.5 rounded-full border-2 border-white border-t-transparent animate-spin")
        span {{ busy ? 'Installing…' : (hasHighRisk ? 'Install (high access)' : 'Install') }}
</template>

<script setup lang="ts">
import {
  flattenPackageAccess,
  type PackageInfo,
} from "../../../desktop-streaming/composables/overlay/packageManifest";
import BaseModal from "./BaseModal.vue";

const props = defineProps<{
  /** Whether the dialog is open. */
  show: boolean;
  /** The package being reviewed (widget is one contribution type; runtime
   *  plugins carry legacy `permissions`). */
  manifest: PackageInfo;
  /** Optional marketplace install count (social proof). */
  installs?: number;
  /** Runtime grant/activation is in progress. */
  busy?: boolean;
  /** Installation error retained while the dialog stays open for retry. */
  error?: string | null;
  /** Backend phase milestone. It orders lifecycle states; it is not measured
   * byte/compiler completion and therefore is not displayed as a percentage. */
  progress?: number;
  /** Current backend lifecycle operation. */
  status?: string;
}>();

defineEmits<{
  /** User approved — caller should install + grant capabilities. */
  (e: "grant"): void;
  /** User cancelled. */
  (e: "cancel"): void;
}>();

// Shared flattening (legacy permissions + widget capabilities) — same list the
// per-plugin Settings modal shows, so the dialog and settings never drift.
const requestedCapabilities = computed(() =>
  flattenPackageAccess(props.manifest),
);

const hasHighRisk = computed(() =>
  requestedCapabilities.value.some((c) => c.risk === "high"),
);

/** Trust badge config; null for builtin (dialog wouldn't normally show then). */
const trustBadge = computed(() => {
  switch (props.manifest.trust) {
    case "verified":
      return {
        label: "✓ Verified creator",
        class: "bg-themeSuccess/20 text-themeSuccess",
      };
    case "community":
      return {
        label: "Community package",
        class: "bg-themeWarningBg/20 text-themeWarningIcon",
      };
    default:
      return null;
  }
});
</script>
