import { describe, expect, it } from "vitest";
import {
  buildActionReferenceNodeType,
  parseFlowDropPayload,
} from "./actionReferenceDrop";

describe("Deck to Flow action-reference drop", () => {
  it("parses the canonical drag payload and creates an invoke-action node", () => {
    const payload = parseFlowDropPayload(JSON.stringify({
      event: "action-reference",
      data: {
        reference: { kind: "saved_action", id: "action-uuid" },
        label: "Switch scene",
      },
    }));

    expect(payload?.event).toBe("action-reference");
    if (!payload || payload.event !== "action-reference") return;

    const node = buildActionReferenceNodeType(payload, "output");
    expect(node.name).toBe("invoke-action");
    expect(node.parameters.actionReference).toEqual({
      kind: "saved_action",
      id: "action-uuid",
    });
    expect(node.outputs[0]).toEqual({ id: "output", name: "Output", type: "flow" });
  });

  it("rejects legacy and malformed action payloads", () => {
    expect(parseFlowDropPayload(JSON.stringify({ event: "action", data: {} }))).toBeNull();
    expect(parseFlowDropPayload(JSON.stringify({
      event: "action-reference",
      data: { reference: { kind: "unknown", id: "x" } },
    }))).toBeNull();
    expect(parseFlowDropPayload(JSON.stringify({
      event: "action-reference",
      data: { reference: { kind: "macro", id: "" } },
    }))).toBeNull();
  });
});
