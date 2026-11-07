import { describe, expect, it } from "vitest";

import {
  canvasConnectionToFlowConnection,
  canonicalizeFlowOutputPort,
  createFlowCanvasConnection,
  flowConnectionToCanvasConnection,
  getNamedFlowRouteOutputs,
  normalizeFlowConnection,
} from "./portSemantics";

describe("flow port semantics", () => {
  it("keeps the default output separate from named branches", () => {
    expect(
      getNamedFlowRouteOutputs([
        { id: "output", type: "flow" },
        { id: "approved", type: "flow" },
        { id: "decision", type: "string" },
      ]),
    ).toEqual([{ id: "approved", type: "flow" }]);
  });

  it("canonicalizes the malformed default-output route", () => {
    expect(canonicalizeFlowOutputPort("flow-output-output")).toBe("flow-output");
    expect(canonicalizeFlowOutputPort("flow-output-approved")).toBe(
      "flow-output-approved",
    );
  });

  it("creates and persists an explicit named control edge", () => {
    const canvasEdge = createFlowCanvasConnection(
      7,
      1,
      2,
      "flow-output-approved",
      "flow-input",
    );

    expect(canvasEdge).toEqual({
      kind: "control",
      id: 7,
      from: 1,
      to: 2,
      fromType: "flow-output-approved",
      toType: "flow-input",
      route: "approved",
    });
    expect(canvasConnectionToFlowConnection(canvasEdge)).toEqual({
      kind: "control",
      fromNode: "1",
      toNode: "2",
      route: "approved",
    });
  });

  it("round-trips a typed data edge without parsing IDs in runtime code", () => {
    const edge = {
      kind: "data" as const,
      fromNode: "10",
      fromOutput: "display-text",
      toNode: "11",
      toInput: "message-body",
    };

    expect(canvasConnectionToFlowConnection(flowConnectionToCanvasConnection(edge, 3)))
      .toEqual(edge);
  });

  it("round-trips opaque node identities through a numeric canvas projection", () => {
    const persistedByCanvas = new Map([[1, "01"], [2, "named-node"]]);
    const canvasByPersisted = new Map([["01", 1], ["named-node", 2]]);
    const edge = {
      kind: "control" as const,
      fromNode: "01",
      toNode: "named-node",
      route: "approved",
    };

    const canvas = flowConnectionToCanvasConnection(
      edge,
      9,
      (id) => canvasByPersisted.get(id)!,
    );
    expect(canvasConnectionToFlowConnection(
      canvas,
      (id) => persistedByCanvas.get(id)!,
    )).toEqual(edge);
  });

  it("migrates old control, reverse data, and default-output edges", () => {
    expect(normalizeFlowConnection({
      fromNode: "1",
      fromPort: "flow-output-output",
      toNode: "2",
      toPort: "flow-input",
    })).toEqual({ kind: "control", fromNode: "1", toNode: "2" });

    expect(normalizeFlowConnection({
      fromNode: "2",
      fromPort: "value-input-message-body",
      toNode: "1",
      toPort: "value-output-display-text",
    })).toEqual({
      kind: "data",
      fromNode: "1",
      fromOutput: "display-text",
      toNode: "2",
      toInput: "message-body",
    });
  });

  it("rejects mixed or malformed legacy edges", () => {
    expect(() => normalizeFlowConnection({
      fromNode: "1",
      fromPort: "flow-output",
      toNode: "2",
      toPort: "value-input-text",
    })).toThrow("Invalid legacy flow connection");
    expect(() => normalizeFlowConnection({
      kind: "unknown",
      fromNode: "1",
      fromPort: "flow-output",
      toNode: "2",
      toPort: "flow-input",
    })).toThrow("Invalid flow connection kind");
  });
});
