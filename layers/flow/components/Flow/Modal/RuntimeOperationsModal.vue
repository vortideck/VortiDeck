<template lang="pug">
BaseModal(
  :is-open="show"
  :close-on-backdrop="!operationPending"
  :z-index="12070"
  @close="emit('close')"
)
  section(class="vd-modal-card flex max-h-[88vh] w-full max-w-6xl flex-col overflow-hidden bg-themeBackground2" role="dialog" aria-modal="true" aria-labelledby="runtime-operations-title")
    header(class="flex items-center justify-between border-b border-themeBorder px-5 py-4")
      div
        h2(id="runtime-operations-title" class="text-base font-bold text-themeText") Workflow runtime diagnostics
        p(class="mt-1 text-xs text-themeText3") Durable state from flows.db. External side-effect replay requires an explicit recovery policy.
      div(class="flex items-center gap-2")
        button(class="rounded-xl border border-themeBorder px-3 py-1.5 text-xs font-semibold text-themeText2 hover:bg-themeBackground3 disabled:opacity-50" :disabled="loading" @click="refresh") {{ loading ? 'Refreshing…' : 'Refresh' }}
        button(ref="closeButton" class="rounded-lg p-2 text-themeText3 hover:bg-themeBackground3 hover:text-themeText disabled:opacity-50" :disabled="operationPending" aria-label="Close diagnostics" @click="emit('close')") ✕

    div(class="overflow-y-auto p-5")
      div(v-if="error" role="alert" class="mb-4 rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-300") {{ error }}
      div(v-if="notice" role="status" class="mb-4 rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-3 text-xs text-emerald-300") {{ notice }}

      div(v-if="inspection" class="space-y-5")
        div(class="grid grid-cols-2 gap-3 md:grid-cols-4")
          article(v-for="card in metricCards" :key="card.label" class="rounded-xl border border-themeBorder bg-themeBackground p-3")
            p(class="text-[11px] font-semibold uppercase tracking-wide text-themeText3") {{ card.label }}
            p(class="mt-1 text-2xl font-bold" :class="card.warning ? 'text-amber-300' : 'text-themeText'") {{ card.value }}

        section(class="rounded-xl border border-themeBorder bg-themeBackground")
          div(class="flex flex-wrap items-center justify-between gap-3 px-4 py-3")
            div
              h3(class="text-sm font-bold text-themeText") Prometheus export
              p(class="mt-1 break-all text-[11px] text-themeText3") {{ metricsEndpoint || 'Metrics endpoint unavailable' }}
            button(class="rounded-lg border border-themeBorder px-3 py-1.5 text-xs font-semibold text-themeText2 hover:bg-themeBackground3 disabled:opacity-50" :disabled="!metricsEndpoint || !metricsAccessToken" @click="copyMetricsCurl") Copy authenticated curl

        section(class="rounded-xl border border-themeBorder bg-themeBackground")
          div(class="flex items-center justify-between border-b border-themeBorder px-4 py-3")
            h3(class="text-sm font-bold text-themeText") Workflow approvals
            span(class="text-xs text-themeText3") {{ approvalWakes.length }} waiting
          div(v-if="approvalWakes.length === 0" class="p-4 text-xs text-themeText3") No workflow nodes are waiting for human approval.
          div(v-else class="divide-y divide-themeBorder")
            article(v-for="wake in approvalWakes" :key="wake.id" class="p-4")
              div(class="flex flex-wrap items-start justify-between gap-3")
                div(class="min-w-0")
                  div(class="flex items-center gap-2")
                    p(class="text-sm font-semibold text-themeText") {{ wake.payload?.title || 'Workflow approval' }}
                    span(class="rounded-full bg-amber-500/10 px-2 py-0.5 text-[10px] font-bold uppercase text-amber-300") {{ wake.payload?.severity || 'normal' }}
                  p(class="mt-1 text-xs text-themeText2") {{ wake.payload?.message || 'A workflow is waiting for a decision.' }}
                  p(class="mt-1 text-[11px] text-themeText3") Run {{ wake.runId }} · node {{ wake.nodeId }}{{ wake.dueAt ? ` · expires ${formatDate(wake.dueAt)}` : '' }}
              pre(v-if="wake.payload?.requestSnapshot" class="mt-3 max-h-32 overflow-auto rounded-lg border border-themeBorder bg-themeBackground3 p-2 text-[11px] text-themeText2") {{ formatSnapshot(wake.payload.requestSnapshot) }}
              textarea(v-model="approvalComments[wake.id]" maxlength="2000" rows="2" placeholder="Optional audit comment" class="mt-3 w-full rounded-lg border border-themeBorder bg-themeBackground3 px-3 py-2 text-xs text-themeText")
              div(class="mt-3 flex justify-end gap-2")
                button(class="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-xs font-semibold text-red-300 disabled:opacity-50" :disabled="resolvingWake === wake.id" @click="resolveApproval(wake.id, 'denied')") Deny
                button(class="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50" :disabled="resolvingWake === wake.id" @click="resolveApproval(wake.id, 'approved')") Approve

        section(class="rounded-xl border border-themeBorder bg-themeBackground")
          div(class="flex items-center justify-between border-b border-themeBorder px-4 py-3")
            h3(class="text-sm font-bold text-themeText") Dead letters
            span(class="text-xs text-themeText3") {{ inspection.deadLetters.length }} unresolved
          div(v-if="inspection.deadLetters.length === 0" class="p-4 text-xs text-themeText3") No failed ingress routes are awaiting review.
          div(v-else class="divide-y divide-themeBorder")
            article(v-for="record in inspection.deadLetters" :key="record.id" class="p-4")
              div(class="flex flex-wrap items-start justify-between gap-3")
                div(class="min-w-0")
                  p(class="truncate text-sm font-semibold text-themeText") {{ record.flowId }} · {{ record.nodeId }}
                  p(class="mt-1 break-words text-xs text-red-300") Route failure details are redacted from exported diagnostics.
                  p(class="mt-1 text-[11px] text-themeText3") Attempts {{ record.attempts }} · {{ formatDate(record.updatedAt) }}
                button(v-if="replayCandidate !== record.id" class="rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-1.5 text-xs font-semibold text-amber-300 hover:bg-amber-500/20" @click="replayCandidate = record.id") Review replay
              div(v-if="replayCandidate === record.id" class="mt-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3")
                p(class="text-xs text-amber-200") This starts a new workflow run from the recorded event and may repeat external side effects.
                div(class="mt-3 flex gap-2")
                  button(class="rounded-lg bg-amber-500 px-3 py-1.5 text-xs font-bold text-black disabled:opacity-50" :disabled="replaying" @click="confirmReplay(record.id)") {{ replaying ? 'Starting…' : 'Confirm replay with side effects' }}
                  button(class="rounded-lg border border-themeBorder px-3 py-1.5 text-xs text-themeText2" :disabled="replaying" @click="replayCandidate = null") Cancel

        section(class="rounded-xl border border-themeBorder bg-themeBackground")
          div(class="flex items-center justify-between border-b border-themeBorder px-4 py-3")
            h3(class="text-sm font-bold text-themeText") Recovery queue
            span(class="text-xs text-themeText3") {{ recoveryRuns.length }} run(s)
          div(v-if="recoveryRuns.length === 0" class="p-4 text-xs text-themeText3") No interrupted workflows require a decision.
          div(v-else class="divide-y divide-themeBorder")
            article(v-for="run in recoveryRuns" :key="run.runId || run.flowId" class="p-4")
              div(class="flex flex-wrap items-start justify-between gap-3")
                div(class="min-w-0")
                  p(class="text-sm font-semibold text-themeText") {{ run.flowDefinition?.name || run.flowId }}
                  p(class="mt-1 text-xs text-themeText2") {{ inFlightNodes(run).length }} in-flight node(s) · recovery is derived from durable per-node ledgers
                  p(class="mt-1 text-[11px] text-themeText3") Run {{ recoveryRunId(run) }} · {{ inFlightNodes(run).join(', ') || 'graph frontier ready' }}
                div(v-if="recoveryCandidate?.runId !== recoveryRunId(run)" class="flex gap-2")
                  button(class="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-xs font-semibold text-emerald-300 hover:bg-emerald-500/20 disabled:opacity-50" :disabled="operationPending" @click="reviewRecovery(run, 'resume')") Review safe resume
                  button(class="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-xs font-semibold text-red-300 hover:bg-red-500/20 disabled:opacity-50" :disabled="operationPending" @click="reviewRecovery(run, 'fail')") Fail safely
              div(v-if="recoveryCandidate?.runId === recoveryRunId(run)" class="mt-3 rounded-xl border p-3" :class="recoveryCandidate.action === 'resume' ? 'border-emerald-500/30 bg-emerald-500/10' : 'border-red-500/30 bg-red-500/10'")
                p(v-if="recoveryCandidate.action === 'resume'" class="text-xs text-emerald-200") Resume is accepted only when the persisted node ledger proves that continuing will not repeat an external side effect. The runtime rejects an unsafe checkpoint.
                p(v-else class="text-xs text-red-200") This permanently marks the interrupted run as failed without repeating its external side effects.
                div(class="mt-3 flex gap-2")
                  button(class="rounded-lg px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50" :class="recoveryCandidate.action === 'resume' ? 'bg-emerald-600' : 'bg-red-600'" :disabled="operationPending" @click="confirmRecovery") {{ recoveryPending ? 'Working…' : recoveryCandidate.action === 'resume' ? 'Confirm safe resume' : 'Confirm fail safely' }}
                  button(class="rounded-lg border border-themeBorder px-3 py-1.5 text-xs text-themeText2 disabled:opacity-50" :disabled="operationPending" @click="recoveryCandidate = null") Cancel

        div(class="grid gap-5 lg:grid-cols-2")
          RuntimeList(title="Pending overlay commands" :items="overlayRows" empty="No overlay commands await ACK or reconnect delivery.")
          RuntimeList(title="Recent side-effect attempts" :items="sideEffectRows" empty="No side-effect attempts have been recorded.")
      div(v-else-if="loading" class="py-16 text-center text-sm text-themeText3") Loading durable runtime state…
</template>

<script setup lang="ts">
import {
  computed,
  defineComponent,
  h,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import BaseModal from "../../../../desktop/components/Modal/BaseModal.vue";
import type { FlowExecutionState } from "../../../types/flow";
import type { WorkflowApprovalWake } from "../../../types/workflowApproval";

interface DeadLetter {
  id: string;
  flowId: string;
  nodeId: string;
  attempts: number;
  hasReason: boolean;
  updatedAt: string;
}
interface RuntimeInspection {
  recoveryRuns: number;
  pendingOverlayCommands: Array<{
    command: { commandId: string; command: string; deliveryRevision: number };
    runId: string;
    status: string;
    updatedAt: string;
  }>;
  sideEffects: Array<{
    idempotencyKey: string;
    nodeId: string;
    status: string;
    updatedAt: string;
    hasError: boolean;
  }>;
  deadLetters: DeadLetter[];
  metrics: Record<string, number | string | null>;
}
interface ServerInfo {
  ip: string;
  port: number;
}

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();
const inspection = ref<RuntimeInspection | null>(null);
const recoveryRuns = ref<FlowExecutionState[]>([]);
const workflowWakes = ref<WorkflowApprovalWake[]>([]);
const closeButton = ref<HTMLButtonElement | null>(null);
const loading = ref(false);
const replaying = ref(false);
const replayCandidate = ref<string | null>(null);
const recoveryCandidate = ref<{
  runId: string;
  action: "resume" | "fail";
} | null>(null);
const recoveryPending = ref(false);
const resolvingWake = ref<string | null>(null);
const approvalComments = ref<Record<string, string>>({});
const error = ref("");
const notice = ref("");
const metricsAccessToken = ref("");
const metricsEndpoint = ref("");
const operationPending = computed(
  () =>
    replaying.value || recoveryPending.value || resolvingWake.value !== null,
);

const RuntimeList = defineComponent({
  props: {
    title: { type: String, required: true },
    items: { type: Array as () => string[], required: true },
    empty: { type: String, required: true },
  },
  setup(listProps) {
    return () =>
      h(
        "section",
        { class: "rounded-xl border border-themeBorder bg-themeBackground" },
        [
          h(
            "h3",
            {
              class:
                "border-b border-themeBorder px-4 py-3 text-sm font-bold text-themeText",
            },
            listProps.title,
          ),
          listProps.items.length
            ? h(
                "ul",
                {
                  class: "max-h-64 divide-y divide-themeBorder overflow-y-auto",
                },
                listProps.items.map((item) =>
                  h(
                    "li",
                    { class: "break-words px-4 py-3 text-xs text-themeText2" },
                    item,
                  ),
                ),
              )
            : h("p", { class: "p-4 text-xs text-themeText3" }, listProps.empty),
        ],
      );
  },
});

const metricCards = computed(() => {
  const metrics = inspection.value?.metrics ?? {};
  return [
    {
      label: "Recovery runs",
      value: inspection.value?.recoveryRuns ?? 0,
      warning: Boolean(inspection.value?.recoveryRuns),
    },
    {
      label: "Approvals",
      value: approvalWakes.value.length,
      warning: approvalWakes.value.length > 0,
    },
    {
      label: "Pending overlay",
      value: metrics.pendingOverlayCommands ?? 0,
      warning: Boolean(metrics.pendingOverlayCommands),
    },
    {
      label: "Outbox dispatching",
      value: metrics.dispatchingSideEffects ?? 0,
      warning: Boolean(metrics.dispatchingSideEffects),
    },
    {
      label: "Open wakes",
      value: metrics.openWorkflowWakes ?? 0,
      warning: false,
    },
    {
      label: "Resource leases",
      value: metrics.activeResourceLeases ?? 0,
      warning: false,
    },
    {
      label: "Dead letters",
      value: metrics.unresolvedDeadLetters ?? 0,
      warning: Boolean(metrics.unresolvedDeadLetters),
    },
    {
      label: "Canonical events",
      value: metrics.canonicalEvents ?? 0,
      warning: false,
    },
    {
      label: "Overlay ACKed",
      value: metrics.acknowledgedOverlayCommands ?? 0,
      warning: false,
    },
    {
      label: "Overlay expired",
      value: metrics.expiredOverlayCommands ?? 0,
      warning: Boolean(metrics.expiredOverlayCommands),
    },
    {
      label: "Side effects failed",
      value: metrics.failedSideEffects ?? 0,
      warning: Boolean(metrics.failedSideEffects),
    },
    {
      label: "Side effects uncertain",
      value: metrics.uncertainSideEffects ?? 0,
      warning: Boolean(metrics.uncertainSideEffects),
    },
    {
      label: "Quarantined records",
      value: metrics.incompatibleRuntimeRecords ?? 0,
      warning: Boolean(metrics.incompatibleRuntimeRecords),
    },
  ];
});
const approvalWakes = computed(() =>
  workflowWakes.value.filter(
    (wake) => wake.kind === "approval" && wake.status === "waiting",
  ),
);

const overlayRows = computed(() =>
  (inspection.value?.pendingOverlayCommands ?? []).map(
    (delivery) =>
      `${delivery.command.command} · ${delivery.command.commandId} · revision ${delivery.command.deliveryRevision} · run ${delivery.runId}`,
  ),
);
const sideEffectRows = computed(() =>
  (inspection.value?.sideEffects ?? []).map(
    (entry) =>
      `${entry.status} · ${entry.nodeId} · ${entry.idempotencyKey}${entry.hasError ? " · error details redacted" : ""}`,
  ),
);

const formatDate = (value: string) => new Date(value).toLocaleString();
const inFlightNodes = (run: FlowExecutionState): string[] =>
  Object.values(run.nodeResults ?? {})
    .filter((result) => result?.status === "Running")
    .map((result) => result.nodeId);
const recoveryRunId = (run: FlowExecutionState): string =>
  run.runId || run.flowId;

const refresh = async () => {
  loading.value = true;
  error.value = "";
  try {
    const [runtime, recoveries, wakes] = await Promise.all([
      invoke<RuntimeInspection>("inspect_workflow_runtime"),
      invoke<FlowExecutionState[]>("list_flow_recovery_runs"),
      invoke<WorkflowApprovalWake[]>("list_workflow_wakes"),
    ]);
    inspection.value = runtime;
    recoveryRuns.value = recoveries;
    workflowWakes.value = wakes;

    const [tokenResult, serverInfoResult] = await Promise.allSettled([
      invoke<string>("get_metrics_access_token"),
      invoke<ServerInfo | null>("get_server_info_new"),
    ]);
    metricsAccessToken.value =
      tokenResult.status === "fulfilled" ? tokenResult.value : "";
    const serverInfo =
      serverInfoResult.status === "fulfilled" ? serverInfoResult.value : null;
    metricsEndpoint.value = serverInfo
      ? `http://${serverInfo.ip}:${serverInfo.port}/metrics`
      : "";
  } catch (cause) {
    error.value = `Could not load runtime diagnostics: ${String(cause)}`;
  } finally {
    loading.value = false;
  }
};

const copyMetricsCurl = async () => {
  if (!metricsEndpoint.value || !metricsAccessToken.value) return;
  try {
    await navigator.clipboard.writeText(
      `curl -H "Authorization: Bearer ${metricsAccessToken.value}" "${metricsEndpoint.value}"`,
    );
    notice.value =
      "Authenticated metrics command copied. Treat it as a process-scoped secret.";
  } catch (cause) {
    error.value = `Metrics command could not be copied: ${String(cause)}`;
  }
};

const formatSnapshot = (snapshot: unknown) => JSON.stringify(snapshot, null, 2);

const resolveApproval = async (
  wakeId: string,
  decision: "approved" | "denied",
) => {
  resolvingWake.value = wakeId;
  error.value = "";
  notice.value = "";
  try {
    await invoke("resolve_workflow_approval", {
      wakeId,
      decision,
      comment: approvalComments.value[wakeId]?.trim() || null,
    });
    delete approvalComments.value[wakeId];
    notice.value =
      decision === "approved"
        ? "Workflow approval accepted."
        : "Workflow approval denied.";
    await refresh();
  } catch (cause) {
    error.value = `Approval could not be resolved: ${String(cause)}`;
  } finally {
    resolvingWake.value = null;
  }
};

const confirmReplay = async (deadLetterId: string) => {
  replaying.value = true;
  error.value = "";
  notice.value = "";
  try {
    const runId = await invoke<string>("replay_workflow_dead_letter", {
      deadLetterId,
      confirmSideEffects: true,
    });
    notice.value = `Replay started as workflow run ${runId}.`;
    replayCandidate.value = null;
    await refresh();
  } catch (cause) {
    error.value = `Replay was not started: ${String(cause)}`;
  } finally {
    replaying.value = false;
  }
};

const reviewRecovery = (run: FlowExecutionState, action: "resume" | "fail") => {
  recoveryCandidate.value = { runId: recoveryRunId(run), action };
};

const confirmRecovery = async () => {
  const candidate = recoveryCandidate.value;
  if (!candidate) return;

  recoveryPending.value = true;
  error.value = "";
  notice.value = "";
  try {
    if (candidate.action === "resume") {
      await invoke("resume_flow_recovery_run", { runId: candidate.runId });
      notice.value = `Workflow run ${candidate.runId} resumed from its proven-safe checkpoint.`;
    } else {
      await invoke("fail_flow_recovery_run", {
        runId: candidate.runId,
        reason: "Operator selected fail safely from runtime diagnostics",
      });
      notice.value = `Workflow run ${candidate.runId} was marked failed without replaying side effects.`;
    }
    recoveryCandidate.value = null;
    await refresh();
  } catch (cause) {
    error.value =
      candidate.action === "resume"
        ? `Safe resume was rejected: ${String(cause)}`
        : `Recovery run could not be failed safely: ${String(cause)}`;
  } finally {
    recoveryPending.value = false;
  }
};

const handleKeydown = (event: KeyboardEvent) => {
  if (props.show && event.key === "Escape" && !operationPending.value)
    emit("close");
};

watch(
  () => props.show,
  async (visible) => {
    replayCandidate.value = null;
    recoveryCandidate.value = null;
    if (visible) {
      void refresh();
      await nextTick();
      closeButton.value?.focus();
    }
  },
);
onMounted(() => {
  document.addEventListener("keydown", handleKeydown);
  if (props.show) void refresh();
});
onBeforeUnmount(() => document.removeEventListener("keydown", handleKeydown));
</script>
