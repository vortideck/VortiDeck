

import type {
  ActionReference,
  ActionReferenceDragPayload,
  ActionReferenceKind,
} from "@base/types/actionReference";

const LOCAL_ACTION_REFERENCE_KINDS = new Set<ActionReferenceKind>([
  "macro",
  "program",
  "flow",
  "system_control",
  "http_request",
  "focus_window",
]);

export const toActionReferenceKind = (type: string): ActionReferenceKind =>
  LOCAL_ACTION_REFERENCE_KINDS.has(type as ActionReferenceKind)
    ? type as ActionReferenceKind
    : "saved_action";

export const useHandleDragAction = (
  event: DragEvent,
  actionId: string | number,
  type: string,
  label?: string,
) => {
  if (!event.dataTransfer) return;

  const dragIcon = document.querySelector<HTMLElement>("#drag-icon");
  if (dragIcon) {
    const rect = dragIcon.getBoundingClientRect();
    event.dataTransfer.setDragImage(dragIcon, rect.width, rect.height);
  }

  const payload: ActionReferenceDragPayload = {
    event: "action-reference",
    data: {
      reference: {
        kind: toActionReferenceKind(type),
        id: String(actionId),
      } satisfies ActionReference,
      label,
    },
  };
  const payloadStr = JSON.stringify(payload);
  event.dataTransfer.setData("application/json", payloadStr);
  event.dataTransfer.setData("text/plain", payloadStr);
};

