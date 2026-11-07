export type ActionReferenceKind =
  | "saved_action"
  | "macro"
  | "program"
  | "flow"
  | "system_control"
  | "http_request"
  | "focus_window";

export interface ActionReference {
  kind: ActionReferenceKind;
  id: string;
}

export interface ActionReferenceDragPayload {
  event: "action-reference";
  data: {
    reference: ActionReference;
    label?: string;
  };
}

export type ActionPreflightStatus =
  | "ready"
  | "syncing"
  | "disconnected"
  | "missing_integration"
  | "missing_target"
  | "permission_denied"
  | "invalid_action"
  | "contract_drift"
  | "receipt_degraded";

export type ActionRepairKind =
  | "retry_preflight"
  | "open_integration_settings"
  | "rebind_action"
  | "replace_references"
  | "disable_affected_flows"
  | "review_receipt_semantics";

export interface ActionPreflightIssue {
  code: string;
  message: string;
  recoverable: boolean;
  repairs?: ActionRepairKind[];
}

export interface ActionPreflight {
  integrationId: string;
  actionId: string;
  status: ActionPreflightStatus;
  capability?: import("./integrationCapability").CapabilityDefinition | null;
  issues?: ActionPreflightIssue[];
}

export interface SavedActionPreflight {
  stableId: string;
  name: string;
  preflight: ActionPreflight;
}
