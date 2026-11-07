<template lang="pug">
BaseModal(
  :is-open="show"
  :z-index="12020"
  @close="emit('close')"
)
  section(class="vd-modal-card flex h-[min(86vh,920px)] w-[min(1200px,96vw)] flex-col overflow-hidden select-text")
    header(class="flex items-center justify-between gap-4 border-b border-themeBorder bg-themeBackground2 px-5 py-4")
      div(class="flex min-w-0 items-center gap-3")
        div(class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-themePrimary/30 bg-themePrimary/10 text-themePrimary")
          MdiHistory(class="h-5 w-5" aria-hidden="true")
        div(class="min-w-0")
          h3(class="text-base font-bold") Execution History
          p(class="truncate text-xs text-themeText2") Archived terminal runs are separate from interrupted recovery work.
      button(
        type="button"
        class="vd-modal-close-btn"
        aria-label="Close execution history"
        title="Close history"
        @click="emit('close')"
      )
        MdiClose(class="h-5 w-5" aria-hidden="true")

    div(class="flex min-h-0 flex-1 flex-col")
      div(
        v-if="recoveryRun"
        class="m-4 mb-0 rounded-xl border border-amber-500/45 bg-amber-500/10 p-4"
      )
        div(class="flex flex-wrap items-start justify-between gap-3")
          div
            p(class="font-semibold text-amber-600 dark:text-amber-300") Recovery decision required
            p(class="mt-1 text-sm text-themeText2")
              | {{ recoveryRun.flowDefinition.name }} was interrupted. It is not an archived failure and will not resume automatically.
          span(class="rounded-full border border-amber-500/40 px-2.5 py-1 text-xs font-semibold text-amber-600 dark:text-amber-300") Recovery required
        div(class="mt-3 flex flex-wrap gap-x-6 gap-y-1 text-xs text-themeText2")
          span Started {{ formatDateTime(recoveryRun.startedAt) }}
          span(v-if="recoveryRun.recoveryDetectedAt") Interruption detected {{ formatDateTime(recoveryRun.recoveryDetectedAt) }}
          span(v-else) Interruption time unavailable for this legacy snapshot

      div(class="grid min-h-0 flex-1 grid-cols-1 gap-0 overflow-hidden md:grid-cols-[19rem_1fr]")
        aside(class="min-h-0 overflow-y-auto border-b border-themeBorder bg-themeBackground2/60 p-4 md:border-b-0 md:border-r")
          div(class="mb-3 flex items-center justify-between")
            p(class="text-xs font-bold uppercase tracking-wider text-themeText2") Archived runs
            button(
              type="button"
              class="rounded-md border border-themeBorder px-2 py-1 text-xs text-themeText2 hover:bg-themeBackground hover:text-themeText disabled:opacity-50"
              :disabled="loading"
              @click="loadHistory"
            ) {{ loading ? 'Loading…' : 'Refresh' }}

          p(v-if="loadError" class="rounded-lg border border-red-500/35 bg-red-500/10 p-3 text-xs text-red-600 dark:text-red-300") {{ loadError }}
          p(v-else-if="!loading && history.length === 0" class="rounded-lg border border-dashed border-themeBorder p-4 text-center text-sm text-themeText2") No archived runs yet.

          button(
            v-for="entry in history"
            :key="entry.id"
            type="button"
            class="mb-2 w-full rounded-xl border p-3 text-left transition"
            :class="entry.id === selectedId ? 'border-themePrimary bg-themePrimary/10' : 'border-themeBorder bg-themeBackground hover:border-themePrimary/50'"
            @click="selectedId = entry.id"
          )
            div(class="flex items-center justify-between gap-2")
              span(class="truncate text-sm font-semibold") {{ entry.flowName }}
              span(class="shrink-0 text-[11px] font-semibold" :class="statusClass(entry.status)") {{ statusText(entry.status) }}
            p(class="mt-1 text-xs text-themeText2") {{ formatDateTime(entry.startedAt) }}
            p(class="mt-1 text-xs text-themeText2") {{ formatDuration(entry.durationMs) }} · {{ entry.completedNodes }}/{{ entry.reachableNodeCount }} nodes

        main(class="min-h-0 overflow-y-auto p-4 md:p-5")
          div(v-if="loading && history.length === 0" class="flex h-full items-center justify-center text-sm text-themeText2") Loading execution history…
          div(v-else-if="selectedEntry" class="space-y-4")
            div(class="grid grid-cols-2 gap-2 lg:grid-cols-4")
              div(class="rounded-xl border border-themeBorder bg-themeBackground2 p-3")
                p(class="text-[11px] uppercase tracking-wider text-themeText2") Status
                p(class="mt-1 text-sm font-semibold" :class="statusClass(selectedEntry.status)") {{ statusText(selectedEntry.status) }}
              div(class="rounded-xl border border-themeBorder bg-themeBackground2 p-3")
                p(class="text-[11px] uppercase tracking-wider text-themeText2") Duration
                p(class="mt-1 text-sm font-semibold") {{ formatDuration(selectedEntry.durationMs) }}
              div(class="rounded-xl border border-themeBorder bg-themeBackground2 p-3")
                p(class="text-[11px] uppercase tracking-wider text-themeText2") Completed
                p(class="mt-1 text-sm font-semibold") {{ selectedEntry.completedNodes }}/{{ selectedEntry.reachableNodeCount }} nodes
              div(class="rounded-xl border border-themeBorder bg-themeBackground2 p-3")
                p(class="text-[11px] uppercase tracking-wider text-themeText2") Finished
                p(class="mt-1 text-sm font-semibold") {{ formatDateTime(selectedEntry.completedAt) }}

            div(v-if="selectedEntry.errorMessage" class="rounded-xl border border-red-500/35 bg-red-500/10 p-4")
              p(class="text-xs font-bold uppercase tracking-wider text-red-600 dark:text-red-300") Execution error
              p(class="mt-1 whitespace-pre-wrap text-sm") {{ selectedEntry.errorMessage }}

            section(class="overflow-hidden rounded-xl border border-themeBorder")
              header(class="flex items-center justify-between bg-themeBackground2 px-4 py-3")
                div
                  h4(class="text-sm font-semibold") Node results
                  p(class="text-xs text-themeText2") Persisted inputs, outputs, timing, and errors for this run.
                button(
                  type="button"
                  class="rounded-md border border-themeBorder px-2.5 py-1.5 text-xs text-themeText2 hover:bg-themeBackground hover:text-themeText"
                  @click="copySelectedRun"
                ) {{ copyFeedback || 'Copy JSON' }}

              div(v-if="selectedEntry.nodeExecutionDetails.length" class="divide-y divide-themeBorder")
                details(v-for="node in selectedEntry.nodeExecutionDetails" :key="node.nodeId" class="group bg-themeBackground")
                  summary(class="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 hover:bg-themeBackground2")
                    div(class="min-w-0")
                      p(class="truncate text-sm font-semibold") {{ node.nodeName || node.nodeId }}
                      p(class="truncate text-xs text-themeText2") {{ node.nodeType }} · {{ node.nodeId }}
                    div(class="flex shrink-0 items-center gap-3 text-xs")
                      span(class="text-themeText2") {{ formatDuration(node.executionTimeMs) }}
                      span(:class="nodeStatusClass(node.status)") {{ node.status }}
                  div(class="grid gap-3 border-t border-themeBorder bg-themeBackground2/45 p-4 lg:grid-cols-2")
                    div
                      p(class="mb-1 text-[11px] font-bold uppercase tracking-wider text-themeText2") Inputs
                      pre(class="max-h-72 overflow-auto rounded-lg border border-themeBorder bg-themeBackground p-3 text-xs") {{ formatJson(node.inputs) }}
                    div
                      p(class="mb-1 text-[11px] font-bold uppercase tracking-wider text-themeText2") Outputs
                      pre(class="max-h-72 overflow-auto rounded-lg border border-themeBorder bg-themeBackground p-3 text-xs") {{ formatJson(node.outputs) }}
                    p(v-if="node.error" class="lg:col-span-2 rounded-lg border border-red-500/35 bg-red-500/10 p-3 text-sm text-red-600 dark:text-red-300") {{ node.error }}
              p(v-else class="p-8 text-center text-sm text-themeText2") This archived run has no node details.
          div(v-else class="flex h-full items-center justify-center text-center")
            div
              MdiHistory(class="mx-auto h-8 w-8 text-themeText3" aria-hidden="true")
              p(class="mt-2 font-semibold") No archived execution selected
              p(class="mt-1 text-sm text-themeText2") Run a flow to create a terminal history record.
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onBeforeUnmount, ref, watch } from "vue";
import MdiClose from "~icons/mdi/close";
import MdiHistory from "~icons/mdi/history";
import BaseModal from "../../../../desktop/components/Modal/BaseModal.vue";
import type { ExecutionHistoryEntry, FlowExecutionState, NodeStatus } from "../../../types/flow";
import { FlowStatus } from "../../../types/flow";

interface Props {
  show: boolean;
  flowId?: string;
  activeFlowExecution?: FlowExecutionState | null;
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  flowId: undefined,
  activeFlowExecution: null,
});
const emit = defineEmits<{ close: [] }>();

const history = ref<ExecutionHistoryEntry[]>([]);
const selectedId = ref<string | null>(null);
const loading = ref(false);
const loadError = ref("");
const copyFeedback = ref("");
let requestRevision = 0;
let copyTimer: ReturnType<typeof setTimeout> | undefined;

const recoveryRun = computed(() =>
  props.activeFlowExecution?.status === FlowStatus.RecoveryRequired
    ? props.activeFlowExecution
    : null
);
const selectedEntry = computed(() =>
  history.value.find((entry) => entry.id === selectedId.value) ?? history.value[0] ?? null
);

const loadHistory = async () => {
  const revision = ++requestRevision;
  loading.value = true;
  loadError.value = "";
  try {
    const entries = await invoke<ExecutionHistoryEntry[]>("list_flow_execution_history", {
      flowId: props.flowId || props.activeFlowExecution?.flowId || null,
      limit: 100,
    });
    if (revision !== requestRevision) return;
    history.value = entries;
    if (!entries.some((entry) => entry.id === selectedId.value)) {
      selectedId.value = entries[0]?.id ?? null;
    }
  } catch (error) {
    if (revision === requestRevision) {
      loadError.value = `Could not load execution history: ${String(error)}`;
    }
  } finally {
    if (revision === requestRevision) loading.value = false;
  }
};

watch(
  () => [props.flowId, props.activeFlowExecution?.flowId] as const,
  () => void loadHistory(),
  { immediate: true },
);

watch(
  () => props.show,
  (show) => {
    if (show && history.value.length === 0 && !loading.value) {
      void loadHistory();
    }
  },
);

const statusText = (status: FlowStatus) => ({
  [FlowStatus.Pending]: "Pending",
  [FlowStatus.Running]: "Running",
  [FlowStatus.RecoveryRequired]: "Recovery required",
  [FlowStatus.Completed]: "Completed",
  [FlowStatus.CompletedWithErrors]: "Completed with errors",
  [FlowStatus.Failed]: "Failed",
  [FlowStatus.Cancelled]: "Cancelled",
})[status] ?? "Unknown";

const statusClass = (status: FlowStatus) => {
  if (status === FlowStatus.Completed) return "text-green-600 dark:text-green-300";
  if (status === FlowStatus.CompletedWithErrors) return "text-amber-600 dark:text-amber-300";
  if (status === FlowStatus.Failed) return "text-red-600 dark:text-red-300";
  if (status === FlowStatus.Cancelled) return "text-themeText2";
  return "text-themePrimary";
};

const nodeStatusClass = (status: NodeStatus) => {
  if (status === "Completed") return "text-green-600 dark:text-green-300";
  if (status === "Failed") return "text-red-600 dark:text-red-300";
  return "text-themeText2";
};

const formatDateTime = (value?: string) => {
  if (!value) return "Not recorded";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Invalid timestamp" : date.toLocaleString();
};

const formatDuration = (durationMs?: number) => {
  if (durationMs === undefined || durationMs === null) return "Not recorded";
  if (durationMs < 1000) return `${durationMs} ms`;
  const seconds = Math.floor(durationMs / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  return `${minutes}m ${seconds % 60}s`;
};

const formatJson = (value: unknown) => JSON.stringify(value ?? null, null, 2);

const copySelectedRun = async () => {
  if (!selectedEntry.value) return;
  try {
    await navigator.clipboard.writeText(formatJson(selectedEntry.value));
    copyFeedback.value = "Copied";
  } catch {
    copyFeedback.value = "Copy failed";
  }
  if (copyTimer) clearTimeout(copyTimer);
  copyTimer = setTimeout(() => (copyFeedback.value = ""), 1600);
};

onBeforeUnmount(() => {
  requestRevision++;
  if (copyTimer) clearTimeout(copyTimer);
});
</script>
