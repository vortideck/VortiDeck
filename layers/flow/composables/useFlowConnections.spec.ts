import { describe, expect, it, vi } from "vitest";
import { ref } from "vue";

import { useFlowConnections } from "./useFlowConnections";

const node = (id: number, overrides: Record<string, unknown> = {}) => ({
  id,
  type: "action",
  text: `node-${id}`,
  x: 0,
  y: 0,
  inputs: [],
  outputs: [],
  ...overrides,
});

const setup = (nodes: any[]) => {
  const connections = ref<any[]>([]);
  const api = useFlowConnections(
    {
      nodes: ref(nodes),
      connections,
      camera: ref({ x: 0, y: 0, scale: 1 }),
    },
    {
      forceConnectionUpdate: vi.fn(),
      findNodeById: (id) => nodes.find((candidate) => candidate.id === id),
      findNodeInputById: (candidate, inputId) =>
        candidate.inputs?.find((input: any) => input.id === inputId),
      findNodeOutputById: (candidate, outputId) =>
        candidate.outputs?.find((output: any) => output.id === outputId),
      isTriggerNode: () => false,
    },
  );
  return { api, connections };
};

describe("flow connection direction", () => {
  it("canonicalizes a drag from an input to an implicit flow output", () => {
    const upstream = node(1);
    const downstream = node(2);
    const { api, connections } = setup([upstream, downstream]);

    expect(
      api.addConnectionWithValidation(
        downstream,
        upstream,
        "flow-input",
        "flow-output",
      ),
    ).toBe(true);
    expect(connections.value).toMatchObject([
      {
        kind: "control",
        from: 1,
        to: 2,
        fromType: "flow-output",
        toType: "flow-input",
      },
    ]);
  });

  it("accepts and preserves a named control route", () => {
    const approval = node(1, {
      outputs: [{ id: "approved", name: "Approved", type: "flow" }],
    });
    const action = node(2);
    const { api, connections } = setup([approval, action]);

    expect(
      api.addConnectionWithValidation(
        approval,
        action,
        "flow-output-approved",
        "flow-input",
      ),
    ).toBe(true);
    expect(connections.value[0]?.fromType).toBe("flow-output-approved");
    expect(connections.value[0]).toMatchObject({
      kind: "control",
      route: "approved",
    });
  });

  it("normalizes the reserved default output instead of treating it as a branch", () => {
    const upstream = node(1, {
      outputs: [{ id: "output", name: "Output", type: "flow" }],
    });
    const downstream = node(2);
    const { api, connections } = setup([upstream, downstream]);

    expect(
      api.addConnectionWithValidation(
        upstream,
        downstream,
        "flow-output-output",
        "flow-input",
      ),
    ).toBe(true);
    expect(connections.value[0]?.fromType).toBe("flow-output");
    expect(connections.value[0]).not.toHaveProperty("route");
  });

  it("canonicalizes value connections without truncating hyphenated port IDs", () => {
    const producer = node(1, {
      outputs: [{ id: "display-text", name: "Text", type: "string" }],
    });
    const consumer = node(2, {
      inputs: [{ id: "tools-to-execute", name: "Tools", type: "string" }],
    });
    const { api, connections } = setup([producer, consumer]);

    expect(
      api.addConnectionWithValidation(
        consumer,
        producer,
        "value-input-tools-to-execute",
        "value-output-display-text",
      ),
    ).toBe(true);
    expect(connections.value[0]).toMatchObject({
      kind: "data",
      from: 1,
      to: 2,
      fromType: "value-output-display-text",
      toType: "value-input-tools-to-execute",
      fromOutput: "display-text",
      toInput: "tools-to-execute",
    });
    expect((consumer.inputs as any[])[0]?.attached).toBe(true);
  });

  it("allows multiple control predecessors to form a join", () => {
    const left = node(1);
    const right = node(2);
    const join = node(3);
    const { api, connections } = setup([left, right, join]);

    expect(api.addConnectionWithValidation(left, join, "flow-output", "flow-input"))
      .toBe(true);
    expect(api.addConnectionWithValidation(right, join, "flow-output", "flow-input"))
      .toBe(true);
    expect(connections.value).toHaveLength(2);
    expect(connections.value.every((edge) => edge.kind === "control")).toBe(true);
  });
});
