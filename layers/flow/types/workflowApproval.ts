export interface WorkflowApprovalWake {
  id: string;
  runId: string;
  nodeId: string;
  kind: "approval" | string;
  status: "waiting" | "ready" | "consumed" | "cancelled" | string;
  dueAt?: string | null;
  payload?: {
    title?: string;
    message?: string;
    severity?: "normal" | "high" | "critical";
    requestedChannels?: string[];
    decisionPolicy?: { allowedChannels?: string[] };
    requestSnapshot?: unknown;
    resolution?: unknown;
  };
  createdAt?: string;
  updatedAt?: string;
}

export interface WorkflowApprovalFlowEvent {
  type:
    | "approvalRequested"
    | "approvalResolved"
    | "flowCancelled"
    | string;
  flowId?: string;
  runId?: string;
  wakeId?: string;
  request?: WorkflowApprovalWake;
}

export const canReviewApprovalOnDesktop = (wake: WorkflowApprovalWake): boolean => {
  if (wake.kind !== "approval" || wake.status !== "waiting") return false;

  const requestedChannels = wake.payload?.requestedChannels ?? [];
  if (requestedChannels.length > 0 && !requestedChannels.includes("desktop")) {
    return false;
  }

  const allowedChannels = wake.payload?.decisionPolicy?.allowedChannels ?? [];
  return allowedChannels.length === 0 || allowedChannels.includes("desktop");
};

