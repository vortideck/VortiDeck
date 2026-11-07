import { describe, expect, it } from "vitest";
import {
  AiNodeBehavior,
  VariableScope,
  type FlowDefinition,
} from "../types/flow";
import {
  FLOW_DOCUMENT_VERSION,
  assertSupportedFlowDocument,
  buildFlowDocument,
  createEditorNodeIdProjection,
  flowRevisionPayload,
  persistedEditorNodeId,
  reidentifyImportedFlow,
  serializeEditorNode,
} from "./flowSerialization";
import {
  canvasConnectionToFlowConnection,
  flowConnectionToCanvasConnection,
} from "./portSemantics";

const baseFlow = (): FlowDefinition => ({
  id: "flow-existing",
  name: "Studio flow",
  version: FLOW_DOCUMENT_VERSION,
  description: "Representative graph",
  createdAt: "2026-09-01T10:00:00.000Z",
  updatedAt: "2026-09-01T10:00:00.000Z",
  createdBy: "user-1",
  tags: ["live"],
  isActive: true,
  nodes: [],
  connections: [],
  startNodeId: "1",
  metadata: {
    canvasSettings: {
      zoom: 0.8,
      panX: 5,
      panY: 9,
      gridEnabled: false,
      snapToGrid: true,
    },
    variables: {
      channel: {
        name: "channel",
        value: "main",
        variableType: "string",
        scope: VariableScope.Global,
      },
    },
    permissions: {
      public: false,
      collaborators: ["operator-1"],
      executionRoles: ["studio"],
    },
    executionSettings: {
      maxExecutionTimeMs: 20_000,
      maxConcurrentNodes: 2,
      autoSaveIntervalMs: 15_000,
    },
  },
});

describe("canonical flow serialization", () => {
  it("preserves execution behavior while projecting editor state", () => {
    const node = serializeEditorNode({
      id: 7,
      type: "ai-card",
      text: "Generate response",
      x: 100,
      y: 200,
      inputs: [
        { id: "prompt", name: "Prompt", type: "string", value: "hello" },
        { id: "waitMs", name: "Wait", type: "number", value: 750 },
      ],
      outputs: [
        { id: "done", name: "Done", type: "flow" },
        { id: "text", name: "Text", type: "string" },
      ],
      isAiNode: true,
      aiBehavior: AiNodeBehavior.TextOnly,
      aiModel: "deterministic-test-model",
      promptTemplate: "template-support-reply",
      parameters: {
        backendId: "ai-text-generation",
        actionReference: { kind: "obs", id: "scene-main" },
      },
      invocationMode: true,
      minimized: true,
      executionConfig: {
        timeoutMs: 5_000,
        retryCount: 2,
        retryDelayMs: 250,
        skipOnError: true,
        parallelExecution: false,
      },
    });

    expect(node).toMatchObject({
      id: "7",
      nodeType: "ai-text-generation",
      inputs: { prompt: "hello", waitMs: 750 },
      promptTemplate: "template-support-reply",
      aiModel: "deterministic-test-model",
      executionConfig: {
        timeoutMs: 5_000,
        retryCount: 2,
        retryDelayMs: 250,
        skipOnError: true,
        parallelExecution: false,
      },
      uiData: {
        customProperties: {
          actionReference: { kind: "obs", id: "scene-main" },
          invocationMode: true,
          minimized: true,
        },
      },
    });
    expect(node.outputs).toEqual([
      { id: "done", name: "Done", outputType: "flow", description: undefined },
      {
        id: "text",
        name: "Text",
        outputType: "string",
        description: undefined,
      },
    ]);
  });

  it("retains editor-unknown inputs and output metadata on a visible edit", () => {
    const persisted: FlowDefinition["nodes"][number] = {
      id: "7",
      nodeType: "delay",
      name: "Delay",
      inputs: {
        duration: 10,
        futureBinding: { source: "fixture", path: "viewer.count" },
      },
      outputs: [
        {
          id: "done",
          name: "Done",
          outputType: "flow",
          description: "Persisted route description",
        },
      ],
      position: [10, 20],
      uiData: {
        collapsed: false,
        selected: false,
        zIndex: 0,
        customProperties: {},
      },
      executionConfig: {
        retryCount: 0,
        retryDelayMs: 1000,
        skipOnError: false,
        parallelExecution: false,
      },
    };

    const serialized = serializeEditorNode({
      id: 7,
      type: "delay",
      text: "Delay",
      x: 30,
      y: 40,
      inputs: [
        { id: "duration", name: "Duration", type: "number", value: 25 },
      ],
      outputs: [{ id: "done", name: "Done", type: "flow" }],
      persistedDefinition: persisted,
    });

    expect(serialized.inputs).toEqual({
      duration: 25,
      futureBinding: { source: "fixture", path: "viewer.count" },
    });
    expect(serialized.outputs[0]).toEqual({
      id: "done",
      name: "Done",
      outputType: "flow",
      description: "Persisted route description",
    });
    expect(serialized.id).toBe("7");
  });

  it("projects opaque persisted node IDs to unique canvas IDs without changing identity", () => {
    const opaqueIds = ["0", "01", "1", "1a", "550e8400-e29b-41d4-a716-446655440000", "named-node"];
    const projection = createEditorNodeIdProjection(
      opaqueIds.map((id) => ({ id })),
    );
    const canvasIds = opaqueIds.map(projection.canvasIdFor);

    expect(new Set(canvasIds).size).toBe(opaqueIds.length);
    expect(canvasIds.map(projection.persistedIdFor)).toEqual(opaqueIds);
    expect(opaqueIds.map((id) => persistedEditorNodeId({
      id: projection.canvasIdFor(id),
      persistedDefinition: { id } as FlowDefinition["nodes"][number],
    }))).toEqual(opaqueIds);

    const persisted = {
      id: "01",
      nodeType: "future-node",
      name: "Future node",
      inputs: { retained: true },
      outputs: [],
      position: [0, 0] as [number, number],
      uiData: { collapsed: false, selected: false, zIndex: 0, customProperties: {} },
      executionConfig: { retryCount: 0, retryDelayMs: 0, skipOnError: false, parallelExecution: false },
      futureNodeField: { retained: true },
    } as FlowDefinition["nodes"][number] & { futureNodeField: { retained: boolean } };
    const serialized = serializeEditorNode({
      id: projection.canvasIdFor(persisted.id),
      type: persisted.nodeType,
      text: persisted.name,
      x: 4,
      y: 8,
      inputs: [],
      outputs: [],
      persistedDefinition: persisted,
    }) as FlowDefinition["nodes"][number] & { futureNodeField: { retained: boolean } };

    expect(serialized.id).toBe("01");
    expect(serialized.futureNodeField).toEqual({ retained: true });
  });

  it("rejects duplicate opaque node IDs before the editor can alias them", () => {
    expect(() => createEditorNodeIdProjection([{ id: "1" }, { id: "1" }]))
      .toThrow(/duplicate node ID/);
  });

  it("retains opaque node and edge identities through the shared load/save/export contract", () => {
    const opaqueIds = ["0", "01", "1", "1a", "550e8400-e29b-41d4-a716-446655440000", "named-node"];
    const persistedNodes = opaqueIds.map((id, index): FlowDefinition["nodes"][number] => ({
      id,
      nodeType: index === 0 ? "button-trigger" : "delay",
      name: `Node ${id}`,
      inputs: { opaqueBinding: id },
      outputs: [{ id: "output", name: "Output", outputType: "flow" }],
      position: [index * 100, index * 20],
      uiData: { collapsed: false, selected: false, zIndex: index, customProperties: {} },
      executionConfig: { retryCount: 0, retryDelayMs: 0, skipOnError: false, parallelExecution: false },
    }));
    const persistedConnections = opaqueIds.slice(1).map((toNode, index) => ({
      kind: "control" as const,
      fromNode: opaqueIds[index]!,
      toNode,
    }));
    const projection = createEditorNodeIdProjection(persistedNodes);
    const editorNodes = persistedNodes.map((node) => ({
      id: projection.canvasIdFor(node.id),
      type: node.nodeType,
      text: node.name,
      x: node.position[0],
      y: node.position[1],
      inputs: node.inputs,
      outputs: node.outputs.map((output) => ({
        id: output.id,
        name: output.name,
        type: output.outputType,
      })),
      persistedDefinition: node,
    }));
    const canvasConnections = persistedConnections.map((connection, index) =>
      flowConnectionToCanvasConnection(connection, index + 1, projection.canvasIdFor)
    );
    const saved = buildFlowDocument({
      id: "roundtrip",
      name: "Opaque identity fixture",
      isActive: false,
      nodes: editorNodes,
      connections: canvasConnections.map((connection) =>
        canvasConnectionToFlowConnection(connection, projection.persistedIdFor)
      ),
      startNodeId: projection.persistedIdFor(editorNodes[0]!.id),
      canvasSettings: baseFlow().metadata.canvasSettings,
      now: "2026-09-06T05:00:00.000Z",
    });

    expect(saved.nodes.map((node) => node.id)).toEqual(opaqueIds);
    expect(saved.connections).toEqual(persistedConnections);
    expect(saved.startNodeId).toBe("0");
    expect(saved.nodes.map((node) => node.inputs.opaqueBinding)).toEqual(opaqueIds);
  });

  it("retains persisted metadata and gives Save As an explicit new identity", () => {
    const base = baseFlow();
    const common = {
      name: base.name,
      description: base.description,
      nodes: [],
      connections: [],
      startNodeId: "1",
      canvasSettings: base.metadata.canvasSettings,
      now: "2026-09-05T12:00:00.000Z",
      baseFlow: base,
    };

    const saved = buildFlowDocument({ ...common, id: base.id, isActive: true });
    const copied = buildFlowDocument({
      ...common,
      id: "flow-copy",
      isActive: true,
      newIdentity: true,
    });

    expect(saved.createdAt).toBe(base.createdAt);
    expect(saved.metadata.variables).toEqual(base.metadata.variables);
    expect(saved.metadata.permissions).toEqual(base.metadata.permissions);
    expect(saved.metadata.executionSettings).toEqual(
      base.metadata.executionSettings,
    );
    expect(copied.id).toBe("flow-copy");
    expect(copied.createdAt).toBe(common.now);
    expect(copied.isActive).toBe(false);
  });

  it("uses a stable semantic revision independent of updatedAt and key order", () => {
    const left = baseFlow();
    const right = JSON.parse(JSON.stringify(left)) as FlowDefinition;
    right.updatedAt = "2026-09-05T12:00:00.000Z";

    expect(flowRevisionPayload(right)).toBe(flowRevisionPayload(left));
    right.name = "Changed";
    expect(flowRevisionPayload(right)).not.toBe(flowRevisionPayload(left));
  });

  it("rejects documents whose contract version is not supported", () => {
    expect(() => assertSupportedFlowDocument({ version: "1" })).not.toThrow();
    expect(() => assertSupportedFlowDocument({ version: "2.0.0" })).toThrow(
      /Unsupported flow document version/,
    );
  });

  it("imports a graph as a current-version inactive draft without dropping graph fields", () => {
    const exported = baseFlow();
    exported.version = "1";
    exported.nodes = [
      {
        id: "ai",
        nodeType: "ai-text-generation",
        name: "AI",
        inputs: { prompt: "hello" },
        outputs: [],
        aiModel: "deterministic-test-model",
        promptTemplate: "template-support-reply",
        position: [10, 20],
        uiData: {
          collapsed: false,
          selected: false,
          zIndex: 0,
          customProperties: { futureField: { retained: true } },
        },
        executionConfig: {
          retryCount: 0,
          retryDelayMs: 1000,
          skipOnError: false,
          parallelExecution: false,
        },
      },
    ];

    const imported = reidentifyImportedFlow(exported, {
      id: "flow-imported",
      now: "2026-09-05T15:00:00.000Z",
    });

    expect(imported).toMatchObject({
      id: "flow-imported",
      version: FLOW_DOCUMENT_VERSION,
      createdAt: "2026-09-05T15:00:00.000Z",
      updatedAt: "2026-09-05T15:00:00.000Z",
      isActive: false,
    });
    expect(imported.nodes[0]).toMatchObject({
      aiModel: "deterministic-test-model",
      promptTemplate: "template-support-reply",
      uiData: { customProperties: { futureField: { retained: true } } },
    });
    expect(exported.id).toBe("flow-existing");
    expect(exported.isActive).toBe(true);
  });
});
