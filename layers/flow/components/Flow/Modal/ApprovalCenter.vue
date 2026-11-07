<template lang="pug">
div
  button(
    v-if="pendingApprovals.length > 0 && !isOpen && !hasVisibleApprovalNotification"
    type="button"
    class="fixed right-6 top-20 z-[12040] flex items-center gap-2 rounded-xl border border-amber-400/40 bg-themeBackground2 px-4 py-3 text-sm font-semibold text-themeText shadow-xl hover:bg-themeBackground3"
    @click="openInbox()"
  )
    span(class="h-2.5 w-2.5 animate-pulse rounded-full bg-amber-400")
    span {{ pendingApprovals.length }} {{ pendingApprovals.length === 1 ? 'approval' : 'approvals' }} waiting

  BaseModal(
    :is-open="isOpen"
    :close-on-backdrop="!resolving"
    :z-index="12100"
    @close="closeInbox"
  )
    section(
      class="vd-modal-card flex max-h-[88vh] w-full max-w-2xl flex-col overflow-hidden bg-themeBackground2"
      role="dialog"
      aria-modal="true"
      aria-labelledby="workflow-approval-title"
    )
      header(class="flex items-start justify-between gap-4 border-b border-themeBorder px-5 py-4")
        div(class="min-w-0")
          div(class="flex items-center gap-2")
            span(class="h-2.5 w-2.5 rounded-full bg-amber-400")
            h2(id="workflow-approval-title" class="text-base font-bold text-themeText") Workflow approval
          p(class="mt-1 text-xs text-themeText3") The workflow is paused. Nothing after this node runs until an authorized person decides.
        button(
          ref="closeButton"
          type="button"
          class="vd-modal-close-btn shrink-0 disabled:opacity-50"
          :disabled="resolving"
          aria-label="Close approval inbox"
          @click="closeInbox"
        ) ✕

      div(class="overflow-y-auto p-5")
        div(v-if="error" role="alert" class="mb-4 rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-300") {{ error }}
        div(v-if="loading && !currentApproval" class="py-12 text-center text-sm text-themeText3") Loading pending approvals…
        div(v-else-if="!currentApproval" class="py-12 text-center")
          p(class="text-sm font-semibold text-themeText") No approval is waiting
          p(class="mt-1 text-xs text-themeText3") The request was resolved, timed out, or cancelled.
        article(v-else)
          div(class="flex flex-wrap items-center justify-between gap-3")
            span(
              class="rounded-full px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide"
              :class="severityClass"
            ) {{ currentApproval.payload?.severity || 'normal' }} priority
            span(v-if="pendingApprovals.length > 1" class="text-xs text-themeText3") {{ currentIndex + 1 }} of {{ pendingApprovals.length }}

          h3(class="mt-4 text-lg font-bold text-themeText") {{ currentApproval.payload?.title || 'Workflow approval required' }}
          p(class="mt-2 whitespace-pre-wrap text-sm leading-6 text-themeText2") {{ currentApproval.payload?.message || 'A workflow is waiting for your decision.' }}

          div(class="mt-4 grid gap-2 rounded-xl border border-themeBorder bg-themeBackground p-3 text-xs text-themeText3 sm:grid-cols-2")
            p(class="break-all")
              span(class="font-semibold text-themeText2") Run:
              |  {{ currentApproval.runId }}
            p(class="break-all")
              span(class="font-semibold text-themeText2") Node:
              |  {{ currentApproval.nodeId }}
            p(v-if="currentApproval.dueAt" class="sm:col-span-2")
              span(class="font-semibold text-themeText2") Decision deadline:
              |  {{ formatDate(currentApproval.dueAt) }}
            p(v-else class="sm:col-span-2") No automatic deadline is configured; this run waits until it is decided or cancelled.

          details(v-if="currentApproval.payload?.requestSnapshot" class="mt-4 rounded-xl border border-themeBorder bg-themeBackground")
            summary(class="cursor-pointer px-3 py-2 text-xs font-semibold text-themeText2") Review decision context
            pre(class="max-h-48 overflow-auto border-t border-themeBorder p-3 text-[11px] text-themeText2") {{ formatSnapshot(currentApproval.payload.requestSnapshot) }}

          label(class="mt-4 block")
            span(class="text-xs font-semibold text-themeText2") Optional audit comment
            textarea(
              v-model="comment"
              maxlength="2000"
              rows="3"
              placeholder="Why are you approving or denying this request?"
              class="vd-modal-textarea mt-2 w-full px-3 py-2 text-sm"
            )

      footer(v-if="currentApproval" class="vd-modal-footer flex flex-wrap items-center justify-between gap-3 px-5 py-4")
        div(class="flex gap-2")
          button(
            v-if="pendingApprovals.length > 1"
            type="button"
            class="rounded-lg border border-themeBorder px-3 py-2 text-xs font-semibold text-themeText2 disabled:opacity-50"
            :disabled="resolving"
            @click="selectRelative(-1)"
          ) Previous
          button(
            v-if="pendingApprovals.length > 1"
            type="button"
            class="rounded-lg border border-themeBorder px-3 py-2 text-xs font-semibold text-themeText2 disabled:opacity-50"
            :disabled="resolving"
            @click="selectRelative(1)"
          ) Next
        div(class="ml-auto flex gap-2")
          button(
            type="button"
            class="rounded-lg border border-red-500/40 bg-red-500/10 px-4 py-2 text-sm font-semibold text-red-300 hover:bg-red-500/20 disabled:opacity-50"
            :disabled="resolving"
            @click="resolve('denied')"
          ) {{ resolving === 'denied' ? 'Denying…' : 'Deny' }}
          button(
            type="button"
            class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-bold text-white hover:bg-emerald-500 disabled:opacity-50"
            :disabled="resolving"
            @click="resolve('approved')"
          ) {{ resolving === 'approved' ? 'Approving…' : 'Approve' }}
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import BaseModal from "../../../../desktop/components/Modal/BaseModal.vue";
import { useMainStore } from "../../../../base/stores/Main";
import {
  canReviewApprovalOnDesktop,
  type WorkflowApprovalFlowEvent,
  type WorkflowApprovalWake,
} from "../../../types/workflowApproval";

const pendingApprovals = ref<WorkflowApprovalWake[]>([]);
const mainStore = useMainStore();
const APPROVAL_NOTIFICATION_PREFIX = "workflow-approval:";
const selectedWakeId = ref<string | null>(null);
const isOpen = ref(false);
const loading = ref(false);
const resolving = ref<"approved" | "denied" | null>(null);
const comment = ref("");
const error = ref("");
const closeButton = ref<HTMLButtonElement | null>(null);
let refreshSequence = 0;
let unlistenFlowEvent: (() => void) | null = null;
let unlistenReviewRequest: (() => void) | null = null;

const currentApproval = computed(() =>
  pendingApprovals.value.find(({ id }) => id === selectedWakeId.value)
    ?? pendingApprovals.value[0]
    ?? null,
);
const hasVisibleApprovalNotification = computed(() => {
  const pendingKeys = new Set(
    pendingApprovals.value.map(({ id }) => `${APPROVAL_NOTIFICATION_PREFIX}${id}`),
  );
  return mainStore
    .getOpenNotificationsForWindow("flow_editor")
    .some(({ key }) => Boolean(key && pendingKeys.has(key)));
});
const currentIndex = computed(() => currentApproval.value
  ? pendingApprovals.value.findIndex(({ id }) => id === currentApproval.value?.id)
  : -1,
);
const severityClass = computed(() => {
  switch (currentApproval.value?.payload?.severity) {
    case "critical":
      return "border border-red-500/30 bg-red-500/10 text-red-300";
    case "high":
      return "border border-orange-500/30 bg-orange-500/10 text-orange-300";
    default:
      return "border border-amber-500/30 bg-amber-500/10 text-amber-300";
  }
});
const formatDate = (value: string) => new Date(value).toLocaleString();
const formatSnapshot = (snapshot: unknown) => JSON.stringify(snapshot, null, 2);

const selectWake = (wakeId?: string | null) => {
  const requested = pendingApprovals.value.find(({ id }) => id === wakeId);
  selectedWakeId.value = requested?.id ?? pendingApprovals.value[0]?.id ?? null;
  comment.value = "";
};

const openInbox = async (wakeId?: string | null) => {
  selectWake(wakeId);
  isOpen.value = true;
  await nextTick();
  closeButton.value?.focus();
};

const closeInbox = () => {
  if (!resolving.value) isOpen.value = false;
};

const isCurrentWindowFocused = async () => {
  try {
    return await getCurrentWindow().isFocused();
  } catch {
    return true;
  }
};

const showApprovalNotification = (wake: WorkflowApprovalWake) => {
  mainStore.addNotification(
    {
      key: `${APPROVAL_NOTIFICATION_PREFIX}${wake.id}`,
      title: wake.payload?.title || "Workflow approval required",
      message: wake.payload?.message || "A workflow is waiting for your decision.",
      buttonText: "Review",
      buttonFunction: () => { void openInbox(wake.id); },
      autoClose: false,
      duration: 0,
      type: "warning",
      color: "bg-amber-500",
    },
    "flow_editor",
  );
};

const refresh = async (options: { open?: boolean; wakeId?: string | null } = {}) => {
  const sequence = ++refreshSequence;
  loading.value = true;
  error.value = "";
  try {
    const wakes = await invoke<WorkflowApprovalWake[]>("list_workflow_wakes");
    if (sequence !== refreshSequence) return;
    pendingApprovals.value = wakes.filter(canReviewApprovalOnDesktop);
    mainStore.removeNotificationsByKeyPrefix(
      APPROVAL_NOTIFICATION_PREFIX,
      pendingApprovals.value.map(({ id }) => `${APPROVAL_NOTIFICATION_PREFIX}${id}`),
    );
    selectWake(options.wakeId ?? selectedWakeId.value);
    if (pendingApprovals.value.length === 0) {
      isOpen.value = false;
    } else if (options.open) {
      await openInbox(options.wakeId);
    }
  } catch (cause) {
    if (sequence === refreshSequence) {
      error.value = `Could not load pending approvals: ${String(cause)}`;
    }
  } finally {
    if (sequence === refreshSequence) loading.value = false;
  }
};

const selectRelative = (offset: number) => {
  if (pendingApprovals.value.length < 2) return;
  const next = (currentIndex.value + offset + pendingApprovals.value.length)
    % pendingApprovals.value.length;
  selectWake(pendingApprovals.value[next]?.id);
};

const resolve = async (decision: "approved" | "denied") => {
  const wake = currentApproval.value;
  if (!wake || resolving.value) return;

  resolving.value = decision;
  error.value = "";
  try {
    await invoke("resolve_workflow_approval", {
      wakeId: wake.id,
      decision,
      comment: comment.value.trim() || null,
    });
    mainStore.removeNotificationByKey(`${APPROVAL_NOTIFICATION_PREFIX}${wake.id}`);
    pendingApprovals.value = pendingApprovals.value.filter(({ id }) => id !== wake.id);
    selectWake();
    if (pendingApprovals.value.length === 0) isOpen.value = false;
    await refresh();
  } catch (cause) {
    error.value = `Approval could not be resolved: ${String(cause)}`;
  } finally {
    resolving.value = null;
  }
};

const handleFlowEvent = async (payload: WorkflowApprovalFlowEvent) => {
  if (payload.type === "approvalRequested" && payload.request) {
    if (!canReviewApprovalOnDesktop(payload.request)) return;
    const existing = pendingApprovals.value.findIndex(({ id }) => id === payload.request?.id);
    if (existing >= 0) pendingApprovals.value[existing] = payload.request;
    else pendingApprovals.value.unshift(payload.request);
    if (await isCurrentWindowFocused()) {
      if (mainStore.getWorkflowApprovalPresentation === "dialog") {
        await openInbox(payload.request.id);
      } else {
        showApprovalNotification(payload.request);
      }
    }
    return;
  }

  if (payload.type === "approvalResolved" && payload.wakeId) {
    mainStore.removeNotificationByKey(
      `${APPROVAL_NOTIFICATION_PREFIX}${payload.wakeId}`,
    );
    pendingApprovals.value = pendingApprovals.value.filter(({ id }) => id !== payload.wakeId);
    selectWake();
    if (pendingApprovals.value.length === 0) isOpen.value = false;
    return;
  }

  if (payload.type === "flowCancelled" && payload.runId) {
    const cancelledWakeIds = pendingApprovals.value
      .filter(({ runId }) => runId === payload.runId)
      .map(({ id }) => id);
    for (const wakeId of cancelledWakeIds) {
      mainStore.removeNotificationByKey(`${APPROVAL_NOTIFICATION_PREFIX}${wakeId}`);
    }
    pendingApprovals.value = pendingApprovals.value.filter(
      ({ runId }) => runId !== payload.runId,
    );
    selectWake();
    if (pendingApprovals.value.length === 0) isOpen.value = false;
    void refresh();
  }
};

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === "Escape" && isOpen.value && !resolving.value) closeInbox();
};

onMounted(async () => {
  unlistenFlowEvent = await listen<WorkflowApprovalFlowEvent>("flow_event", ({ payload }) => {
    void handleFlowEvent(payload);
  });
  unlistenReviewRequest = await listen<{ wakeId?: string }>(
    "review-workflow-approval",
    ({ payload }) => { void refresh({ open: true, wakeId: payload?.wakeId }); },
  );
  document.addEventListener("keydown", handleKeydown);
  const initialWakeId = new URL(window.location.href).searchParams.get("approvalWakeId");
  await refresh({
    open: Boolean(initialWakeId)
      || (mainStore.getWorkflowApprovalPresentation === "dialog"
        && await isCurrentWindowFocused()),
    wakeId: initialWakeId,
  });
});

onBeforeUnmount(() => {
  unlistenFlowEvent?.();
  unlistenReviewRequest?.();
  document.removeEventListener("keydown", handleKeydown);
});
</script>
