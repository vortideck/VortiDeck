import { flushPromises } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FlowStatus, NodeStatus } from "../../flow/types/flow";

const tauri = vi.hoisted(() => ({
  invoke: vi.fn(),
  flowEventHandler: undefined as
    | ((event: { payload: unknown }) => void)
    | undefined,
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauri.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    (_event: string, handler: (event: { payload: unknown }) => void) => {
      tauri.flowEventHandler = handler;
      return Promise.resolve(vi.fn());
    },
  ),
}));

import { useFlowManager } from "./useFlowManager";

const flow = {
  id: "flow-1",
  name: "Display",
  version: "1.0.0",
  createdAt: "2026-08-29T17:04:00Z",
  updatedAt: "2026-08-29T17:04:00Z",
  tags: [],
  nodes: [
    {
      id: "delay",
      nodeType: "delay",
      name: "Delay",
      inputs: {},
      outputs: [],
      position: [0, 0] as [number, number],
      uiData: {
        collapsed: false,
        selected: false,
        zIndex: 0,
        customProperties: {},
      },
      executionConfig: {
        retryCount: 0,
        retryDelayMs: 0,
        skipOnError: false,
        parallelExecution: false,
      },
    },
  ],
  connections: [],
  startNodeId: "delay",
  isActive: false,
  metadata: {
    canvasSettings: {
      zoom: 1,
      panX: 0,
      panY: 0,
      gridEnabled: true,
      snapToGrid: false,
    },
    variables: {},
    permissions: { public: false, collaborators: [], executionRoles: [] },
    executionSettings: { maxConcurrentNodes: 5, autoSaveIntervalMs: 30_000 },
  },
};

const runningState = {
  runId: "run-1",
  flowId: flow.id,
  flowDefinition: flow,
  status: FlowStatus.Running,
  currentNode: "delay",
  completedNodes: [],
  failedNodes: [],
  nodeResults: {
    delay: {
      nodeId: "delay",
      nodeName: "Delay",
      nodeType: "delay",
      status: NodeStatus.Running,
      inputs: {},
      outputs: {},
      executionTimeMs: 0,
      retryCount: 0,
    },
  },
  executionLog: [],
  startedAt: "2026-08-29T17:04:00Z",
  pendingNodes: [],
  sortedNodeResults: [],
};

const completedNode = {
  ...runningState.nodeResults.delay,
  status: NodeStatus.Completed,
  executionTimeMs: 1_000,
};

const completedState = {
  ...runningState,
  status: FlowStatus.Completed,
  currentNode: undefined,
  completedNodes: ["delay"],
  nodeResults: { delay: completedNode },
  sortedNodeResults: [completedNode],
  completedAt: "2026-08-29T17:04:01Z",
};

describe("useFlowManager terminal reconciliation", () => {
  it('applies the same export privacy checks to ordinary JSON and recipes', () => {
    const manager = useFlowManager();
    for (const value of [
      { authentication: { type: 'bearer', token: 'private' } },
      { authentication: { type: 'custom_headers', headers: { 'X-Service': 'private' } } },
      { headers: { 'X-Service': 'vortideck-credential://00000000-0000-4000-8000-000000000000' } },
      { url: 'https://example.com/?access_token=private' },
    ]) {
      const copy = structuredClone(flow);
      copy.nodes[0]!.inputs = value;
      expect(() => manager.exportFlow(copy)).toThrow(/sensitive values/);
    }
    expect(flow.nodes[0]!.inputs).toEqual({});
  });
  beforeEach(() => {
    localStorage.clear();
    tauri.invoke.mockReset();
    tauri.flowEventHandler = undefined;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("applies the final execution-state event before the backend releases the live run", async () => {
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "start_flow_execution") return Promise.resolve("run-1");
      if (command === "get_flow_execution_status")
        return Promise.resolve(runningState);
      return Promise.resolve(undefined);
    });
    const manager = useFlowManager();
    manager.currentFlow.value = flow;

    await manager.executeFlow();
    await flushPromises();
    expect(manager.activeFlowExecution.value?.status).toBe(FlowStatus.Running);

    tauri.flowEventHandler?.({
      payload: {
        type: "executionStateChanged",
        flowId: flow.id,
        runId: "run-1",
        executionState: completedState,
      },
    });

    expect(manager.activeFlowExecution.value?.status).toBe(
      FlowStatus.Completed,
    );
    expect(manager.flowProgress.value).toBe(100);
  });

  it("reconciles archived history when the run finishes between polling intervals", async () => {
    vi.useFakeTimers();
    let statusChecks = 0;
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "start_flow_execution") return Promise.resolve("run-1");
      if (command === "get_flow_execution_status") {
        statusChecks += 1;
        return statusChecks === 1
          ? Promise.resolve(runningState)
          : Promise.reject(new Error("Flow run-1 not found"));
      }
      if (command === "list_flow_execution_history") {
        return Promise.resolve([
          {
            id: "history-run-1",
            runId: "run-1",
            flowId: flow.id,
            flowName: flow.name,
            status: FlowStatus.Completed,
            startedAt: runningState.startedAt,
            completedAt: completedState.completedAt,
            nodeExecutionDetails: [completedNode],
          },
        ]);
      }
      return Promise.resolve(undefined);
    });
    const manager = useFlowManager();
    manager.currentFlow.value = flow;

    await manager.executeFlow();
    await flushPromises();
    expect(manager.activeFlowExecution.value?.status).toBe(FlowStatus.Running);

    await vi.advanceTimersByTimeAsync(1_000);
    await flushPromises();

    expect(manager.activeFlowExecution.value?.status).toBe(
      FlowStatus.Completed,
    );
    expect(manager.activeFlowExecution.value?.currentNode).toBeUndefined();
    expect(manager.flowProgress.value).toBe(100);
    expect(manager.error.value).toBeNull();
    expect(statusChecks).toBe(2);
  });

  it("reconciles a flow archived before its first status poll", async () => {
    tauri.invoke.mockImplementation((command: string) => {
      if (command === "start_flow_execution") return Promise.resolve("run-1");
      if (command === "get_flow_execution_status") {
        return Promise.reject(new Error("Flow run-1 not found"));
      }
      if (command === "list_flow_execution_history") {
        return Promise.resolve([
          {
            id: "history-run-1",
            runId: "run-1",
            flowId: flow.id,
            flowName: flow.name,
            status: FlowStatus.Completed,
            startedAt: runningState.startedAt,
            completedAt: completedState.completedAt,
            nodeExecutionDetails: [completedNode],
          },
        ]);
      }
      return Promise.resolve(undefined);
    });
    const manager = useFlowManager();
    manager.currentFlow.value = flow;

    await manager.executeFlow();
    await flushPromises();

    expect(manager.activeFlowExecution.value?.status).toBe(
      FlowStatus.Completed,
    );
    expect(manager.flowProgress.value).toBe(100);
    expect(manager.error.value).toBeNull();
  });

  it("round-trips exported behavior into an explicitly new inactive import identity", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-09-05T15:00:00.000Z"));
    const manager = useFlowManager();
    const representative = structuredClone(flow);
    representative.version = "1";
    representative.isActive = true;
    representative.nodes[0] = {
      ...representative.nodes[0],
      nodeType: "ai-text-generation",
      aiModel: "deterministic-test-model",
      promptTemplate: "template-support-reply",
      uiData: {
        ...representative.nodes[0].uiData,
        customProperties: { futureField: { retained: true } },
      },
    };

    const exported = manager.exportFlow(representative);
    const imported = manager.importFlow(exported);

    expect(JSON.parse(exported)).toEqual(representative);
    expect(imported.id).toBe(`flow-${Date.parse("2026-09-05T15:00:00.000Z")}`);
    expect(imported.version).toBe("1.0.0");
    expect(imported.createdAt).toBe("2026-09-05T15:00:00.000Z");
    expect(imported.updatedAt).toBe("2026-09-05T15:00:00.000Z");
    expect(imported.isActive).toBe(false);
    expect(imported.nodes[0]).toMatchObject({
      nodeType: "ai-text-generation",
      aiModel: "deterministic-test-model",
      promptTemplate: "template-support-reply",
      uiData: { customProperties: { futureField: { retained: true } } },
    });
  });

  it("rejects unsupported imported document versions before changing editor state", () => {
    const manager = useFlowManager();
    const unsupported = { ...structuredClone(flow), version: "2.0.0" };

    expect(() => manager.importFlow(JSON.stringify(unsupported))).toThrow(
      /Unsupported flow document version/,
    );
    expect(manager.currentFlow.value).toBeNull();
  });
});
