import { defineStore } from "pinia";
import { computed, ref } from "vue";

export interface WorkflowApproval {
  id: string;
  runId: string;
  nodeId: string;
  status: "waiting" | "ready" | "consumed" | "cancelled";
  dueAt?: string | null;
  payload: {
    title?: string;
    message?: string;
    severity?: "normal" | "high" | "critical";
    requestedChannels?: string[];
    decisionPolicy?: { allowedChannels?: string[] };
    requestSnapshot?: unknown;
    resolution?: unknown;
  };
  createdAt: string;
  updatedAt: string;
}

export const useApprovalsStore = defineStore("workflow-approvals", () => {
  const approvals = ref<WorkflowApproval[]>([]);
  const pending = computed(() => approvals.value.filter(({ status }) => status === "waiting"));

  const replace = (items: WorkflowApproval[]) => {
    approvals.value = items.filter(({ status }) => status === "waiting");
  };

  const upsert = (approval: WorkflowApproval) => {
    const index = approvals.value.findIndex(({ id }) => id === approval.id);
    if (approval.status !== "waiting") {
      if (index >= 0) approvals.value.splice(index, 1);
      return;
    }
    if (index >= 0) approvals.value[index] = approval;
    else approvals.value.unshift(approval);
  };

  const remove = (wakeId: string) => {
    approvals.value = approvals.value.filter(({ id }) => id !== wakeId);
  };

  return { approvals, pending, replace, upsert, remove };
});
