<template lang="pug">
BaseModal(:is-open="show" :close-on-backdrop="!loading" :z-index="12060" @close="emit('close')")
  section(class="vd-modal-card flex max-h-[86vh] w-full max-w-4xl flex-col overflow-hidden bg-themeBackground2" role="dialog" aria-modal="true" aria-labelledby="flow-preparation-title")
    header(class="flex items-start justify-between gap-4 border-b border-themeBorder px-5 py-4")
      div(class="flex min-w-0 items-start gap-3")
        div(class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-themePrimary/25 bg-themePrimary/10 text-themePrimary")
          HugeiconsTestTube01(class="h-5 w-5")
        div
          h2(id="flow-preparation-title" class="text-base font-bold text-themeText") {{ mode === 'import' ? 'Review recipe import' : 'Safe flow rehearsal' }}
          p(class="mt-1 text-xs text-themeText3") {{ mode === 'import' ? 'Inspect dependencies and security changes before adding this recipe.' : 'Read-only resolution and preflight. No provider is called and no external effect is performed.' }}
      button(class="rounded-lg p-2 text-themeText3 hover:bg-themeBackground3 hover:text-themeText disabled:opacity-50" :disabled="loading" aria-label="Close" @click="emit('close')")
        EvaCloseOutline(class="h-4 w-4")

    div(class="custom-scrollbar-always overflow-y-auto p-5")
      div(v-if="loading" class="flex min-h-48 items-center justify-center gap-2 text-sm text-themeText2")
        EvaLoaderOutline(class="h-5 w-5 animate-spin text-themePrimary")
        span Running read-only checks…
      div(v-else-if="error" role="alert" class="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-300") {{ error }}

      template(v-else-if="mode === 'rehearsal' && report")
        div(class="grid gap-3 sm:grid-cols-3")
          article(class="rounded-xl border border-themeBorder bg-themeBackground p-3")
            p(class="text-[10px] font-bold uppercase tracking-wider text-themeText3") Outcome
            p(class="mt-1 text-sm font-bold" :class="report.ready ? 'text-emerald-400' : 'text-red-400'") {{ report.ready ? 'Ready for a real run' : 'Repair required' }}
          article(class="rounded-xl border border-themeBorder bg-themeBackground p-3")
            p(class="text-[10px] font-bold uppercase tracking-wider text-themeText3") Effect mode
            p(class="mt-1 text-sm font-bold text-themeText") Read-only · zero effects
          article(class="rounded-xl border border-themeBorder bg-themeBackground p-3")
            p(class="text-[10px] font-bold uppercase tracking-wider text-themeText3") Graph revision
            p(class="mt-1 truncate font-mono text-xs text-themeText2" :title="report.flowRevision") {{ shortRevision }}

        section(v-if="report.blockers.length" class="mt-4 rounded-xl border border-red-500/30 bg-red-500/10")
          h3(class="border-b border-red-500/20 px-4 py-3 text-sm font-bold text-red-300") Blocking repairs
          ul(class="space-y-2 p-4")
            li(v-for="blocker in report.blockers" :key="blocker" class="flex gap-2 text-xs text-red-200")
              EvaAlertTriangleOutline(class="mt-0.5 h-4 w-4 shrink-0")
              span {{ blocker }}

        section(v-if="report.warnings.length" class="mt-4 rounded-xl border border-amber-500/30 bg-amber-500/10")
          h3(class="border-b border-amber-500/20 px-4 py-3 text-sm font-bold text-amber-300") Evidence limits
          ul(class="space-y-2 p-4")
            li(v-for="warning in report.warnings" :key="warning" class="text-xs text-amber-200") {{ warning }}

        section(class="mt-4 overflow-hidden rounded-xl border border-themeBorder bg-themeBackground")
          div(class="flex items-center justify-between border-b border-themeBorder px-4 py-3")
            div
              h3(class="text-sm font-bold text-themeText") Action checks
              p(class="mt-0.5 text-[11px] text-themeText3") Provider-owned capability and connection evidence at this revision.
            span(class="text-xs text-themeText3") {{ report.actionHealth.length }} action(s)
          p(v-if="report.actionHealth.length === 0" class="p-4 text-xs text-themeText3") This graph has no saved-action bindings to resolve.
          div(v-else class="divide-y divide-themeBorder")
            article(v-for="health in report.actionHealth" :key="`${health.nodeId}:${health.stableActionId}`" class="flex items-start justify-between gap-4 px-4 py-3")
              div(class="min-w-0")
                p(class="truncate text-sm font-semibold text-themeText") {{ health.nodeName }}
                p(class="mt-1 text-[11px] text-themeText3") {{ actionEvidence(health) }}
              span(class="shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-bold uppercase" :class="actionStatusClass(health.preflight.status)") {{ formatStatus(health.preflight.status) }}

        section(v-if="report.executableNodes.length" class="mt-4 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4")
          h3(class="text-sm font-bold text-amber-300") Executable nodes require review
          p(class="mt-1 text-xs text-amber-200") Rehearsal identifies these nodes but does not execute them.
          ul(class="mt-2 space-y-1 text-xs text-amber-100")
            li(v-for="node in report.executableNodes" :key="node.nodeId" class="space-y-0.5")
              p {{ node.nodeName }} · {{ formatStatus(node.classification) }}
              p(class="text-[11px] text-amber-200/80") {{ node.reason }}

      template(v-else-if="mode === 'import' && importPreview")
        div(class="grid gap-3 sm:grid-cols-3")
          article(class="rounded-xl border border-themeBorder bg-themeBackground p-3")
            p(class="text-[10px] font-bold uppercase tracking-wider text-themeText3") Recipe
            p(class="mt-1 truncate text-sm font-bold text-themeText") {{ importPreview.envelope.recipe.name }}
            p(class="mt-0.5 text-[11px] text-themeText3") v{{ importPreview.envelope.recipe.version }}
          article(class="rounded-xl border border-themeBorder bg-themeBackground p-3")
            p(class="text-[10px] font-bold uppercase tracking-wider text-themeText3") New identity
            p(class="mt-1 text-sm font-bold text-themeText") Fresh inactive flow
            p(class="mt-0.5 truncate font-mono text-[10px] text-themeText3" :title="importPreview.importedFlowId") {{ importPreview.importedFlowId }}
          article(class="rounded-xl border border-themeBorder bg-themeBackground p-3")
            p(class="text-[10px] font-bold uppercase tracking-wider text-themeText3") Permissions
            p(class="mt-1 text-sm font-bold text-emerald-400") Reset to private
            p(class="mt-0.5 text-[11px] text-themeText3") No collaborators or execution roles copied

        section(class="mt-4 overflow-hidden rounded-xl border border-themeBorder bg-themeBackground")
          div(class="flex items-center justify-between border-b border-themeBorder px-4 py-3")
            div
              h3(class="text-sm font-bold text-themeText") Bindings to repair
              p(class="mt-0.5 text-[11px] text-themeText3") Choose local saved actions after import; credentials and parameters were not copied.
            span(class="text-xs text-themeText3") {{ importPreview.missingBindings.length }} required
          p(v-if="importPreview.missingBindings.length === 0" class="p-4 text-xs text-themeText3") No local action bindings are required.
          div(v-else class="divide-y divide-themeBorder")
            article(v-for="dependency in importPreview.missingBindings" :key="dependency.slotId" class="px-4 py-3")
              p(class="text-sm font-semibold text-themeText") {{ dependency.nodeName }}
              p(class="mt-1 text-[11px] text-themeText3") {{ dependency.integrationId }} · {{ dependency.actionId }} · slot {{ dependency.slotId }}

        section(v-if="importPreview.executableNodes.length" class="mt-4 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4")
          h3(class="text-sm font-bold text-amber-300") Executable content
          p(class="mt-1 text-xs text-amber-200") Review these nodes before enabling or running the imported flow.
          ul(class="mt-2 space-y-1 text-xs text-amber-100")
            li(v-for="node in importPreview.executableNodes" :key="node.nodeId" class="space-y-0.5")
              p {{ node.nodeName }} · {{ formatStatus(node.classification) }}
              p(class="text-[11px] text-amber-200/80") {{ node.reason }}

        section(class="mt-4 rounded-xl border border-themeBorder bg-themeBackground p-4")
          h3(class="text-sm font-bold text-themeText") Import inspection boundary
          p(class="mt-1 text-xs text-themeText2") Known credential-shaped fields and URL/header values were screened, and saved-action references remain provider-owned placeholders. This is a heuristic screen, not proof that arbitrary custom or package code contains no secret; review unknown and plugin nodes before enabling the flow.

    footer(v-if="!loading" class="flex items-center justify-between gap-3 border-t border-themeBorder px-5 py-4")
      p(class="text-[11px] text-themeText3") {{ mode === 'import' ? 'Import does not activate or execute the recipe.' : 'A real run is separate and will write authoritative execution history.' }}
      div(class="flex shrink-0 gap-2")
        button(class="rounded-xl border border-themeBorder px-4 py-2 text-xs font-semibold text-themeText2 hover:bg-themeBackground3" @click="emit('close')") Cancel
        button(v-if="mode === 'import' && importPreview" class="rounded-xl bg-themeButton px-4 py-2 text-xs font-bold text-white hover:bg-themeButtonHover" @click="emit('confirmImport')") Import inactive flow
        button(v-else-if="report?.ready" class="rounded-xl bg-themeButton px-4 py-2 text-xs font-bold text-white hover:bg-themeButtonHover" @click="emit('runRealFlow')") Run real flow
</template>

<script setup lang="ts">
import { computed } from "vue";
import EvaAlertTriangleOutline from "~icons/eva/alert-triangle-outline";
import EvaCloseOutline from "~icons/eva/close-outline";
import EvaLoaderOutline from "~icons/eva/loader-outline";
import HugeiconsTestTube01 from "~icons/hugeicons/test-tube-01";
import BaseModal from "../../../../desktop/components/Modal/BaseModal.vue";
import type { FlowRehearsalReport } from "../../../utils/flowPreparation";
import type { FlowActionHealth, FlowRecipeImportPreview } from "../../../utils/flowRecipe";

const props = defineProps<{
  show: boolean;
  mode: "rehearsal" | "import";
  report?: FlowRehearsalReport | null;
  importPreview?: FlowRecipeImportPreview | null;
  loading?: boolean;
  error?: string;
}>();
const emit = defineEmits<{ close: []; runRealFlow: []; confirmImport: [] }>();

const shortRevision = computed(() => props.report?.flowRevision.slice(0, 16) || "Unavailable");
const formatStatus = (value: string) => value.replace(/[_-]/g, " ");
const actionEvidence = (health: FlowActionHealth) => health.preflight.issues?.map((issue) => issue.message).join("; ")
  || `${health.preflight.integrationId || "local"} · ${health.preflight.actionId}`;
const actionStatusClass = (status: string) => status === "ready"
  ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
  : status === "receipt_degraded"
    ? "border-amber-500/30 bg-amber-500/10 text-amber-300"
    : "border-red-500/30 bg-red-500/10 text-red-300";
</script>
