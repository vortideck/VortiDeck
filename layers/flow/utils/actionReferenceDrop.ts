import type {
  ActionReference,
  ActionReferenceDragPayload,
  ActionReferenceKind,
} from "@base/types/actionReference";

export interface DeckModeDropPayload {
  event: "mode";
  data?: { mode?: string };
}

export type FlowDropPayload = ActionReferenceDragPayload | DeckModeDropPayload;

const ACTION_REFERENCE_KINDS = new Set<ActionReferenceKind>([
  "saved_action",
  "macro",
  "program",
  "flow",
  "system_control",
  "http_request",
  "focus_window",
]);

const ACTION_KIND_LABELS: Record<ActionReferenceKind, string> = {
  saved_action: "Saved Action",
  macro: "Macro",
  program: "Program",
  flow: "Flow",
  system_control: "Value Action",
  http_request: "HTTP Request",
  focus_window: "Focus Window",
};

export const parseFlowDropPayload = (raw: string): FlowDropPayload | null => {
  if (!raw) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return null;
    const candidate = parsed as Record<string, any>;
    if (candidate.event === "action-reference") {
      const reference = candidate.data?.reference as Partial<ActionReference> | undefined;
      if (
        !reference ||
        !ACTION_REFERENCE_KINDS.has(reference.kind as ActionReferenceKind) ||
        typeof reference.id !== "string" ||
        reference.id.trim().length === 0
      ) {
        return null;
      }
      return candidate as unknown as ActionReferenceDragPayload;
    }
    return candidate.event === "mode"
      ? candidate as unknown as DeckModeDropPayload
      : null;
  } catch {
    return null;
  }
};

export const buildActionReferenceNodeType = (
  payload: ActionReferenceDragPayload,
  defaultFlowOutputId: string,
) => {
  const { reference } = payload.data;
  const providedLabel = payload.data.label?.trim();
  const readableType = ACTION_KIND_LABELS[reference.kind];

  return {
    name: "invoke-action",
    label: providedLabel ? `${readableType}: ${providedLabel}` : `${readableType} Action`,
    category: "ACTION",
    inputs: [{ id: 'inputs', name: 'Action inputs', type: 'object', required: false, value: {} }],
    outputs: [
      { id: defaultFlowOutputId, name: "Output", type: "flow" },
      { id: "status", name: "status", type: "boolean" },
      { id: "outcome", name: "outcome", type: "string" },
      { id: "partial", name: "partial", type: "boolean" },
      { id: "targetResults", name: "target results", type: "array" },
      { id: "result", name: "result", type: "object" },
    ],
    color: "#d946ef",
    parameters: { actionReference: reference },
  };
};
