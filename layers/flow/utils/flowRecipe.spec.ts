import { describe, expect, it } from "vitest";
import type { FlowDefinition } from "../types/flow";
import { buildFlowRecipe, parseFlowRecipe } from "./flowRecipe";

const flow = (): FlowDefinition => ({
  id: "local-break-scene",
  name: "Switch to break scene",
  version: "1.0.0",
  description: "Switch OBS to the configured break scene.",
  createdAt: "2026-09-05T10:00:00.000Z",
  updatedAt: "2026-09-05T10:00:00.000Z",
  tags: [],
  startNodeId: "1",
  isActive: true,
  nodes: [
    {
      id: "1", nodeType: "button-trigger", name: "Button", inputs: {}, outputs: [],
      position: [0, 0], uiData: { collapsed: false, selected: false, zIndex: 0, customProperties: {} },
      executionConfig: { retryCount: 0, retryDelayMs: 1000, skipOnError: false, parallelExecution: false },
    },
    {
      id: "2", nodeType: "invoke-action", name: "Break scene", inputs: {
        actionReference: { kind: "saved_action", id: "local-obs-action" },
      }, outputs: [], position: [240, 0],
      uiData: { collapsed: false, selected: false, zIndex: 0, customProperties: {
        actionReference: { kind: "saved_action", id: "local-obs-action" },
      } },
      executionConfig: { retryCount: 0, retryDelayMs: 1000, skipOnError: false, parallelExecution: false },
    },
  ],
  connections: [{ kind: "control", fromNode: "1", toNode: "2" }],
  metadata: {
    canvasSettings: { zoom: 1, panX: 0, panY: 0, gridEnabled: true, snapToGrid: false },
    variables: {},
    permissions: { public: true, collaborators: ["local-user"], executionRoles: ["owner"] },
    executionSettings: { maxConcurrentNodes: 5, autoSaveIntervalMs: 30_000 },
  },
});

const health = [{
  nodeId: "2",
  nodeName: "Break scene",
  stableActionId: "local-obs-action",
  preflight: {
    integrationId: "obs",
    actionId: "set_current_program_scene",
    status: "ready" as const,
    capability: {
      integrationId: "obs",
      capabilityId: "set_current_program_scene",
      version: 1,
      kind: "action" as const,
      presentation: { name: "Activate scene", description: "", category: "OBS" },
      inputs: [], outputs: [], allowAdditionalInputs: false,
      contract: { requiredPermission: "integration:obs:set_scene", receipt: "applied" },
    },
  },
}];

describe("portable flow recipes", () => {
  it("exports no local binding, activation authority, or collaborators", () => {
    const recipe = buildFlowRecipe(flow(), health, {
      recipeId: "break-scene",
      recipeVersion: "1.0.0",
      now: "2026-09-05T12:00:00.000Z",
    });

    expect(JSON.stringify(recipe)).not.toContain("local-obs-action");
    expect(recipe.flow.isActive).toBe(false);
    expect(recipe.flow.metadata.permissions).toEqual({
      public: false, collaborators: [], executionRoles: [],
    });
    expect(recipe.flow.nodes[1]?.inputs.actionReference).toEqual({
      kind: "saved_action", id: "recipe-binding:action-1",
    });
    expect(recipe.dependencies[0]).toMatchObject({
      integrationId: "obs", actionId: "set_current_program_scene", nodeId: "2",
    });
  });

  it("imports as a fresh inactive flow and reports the binding to repair", () => {
    const recipe = buildFlowRecipe(flow(), health, {
      recipeId: "break-scene", recipeVersion: "1.0.0", now: "2026-09-05T12:00:00.000Z",
    });
    const preview = parseFlowRecipe(JSON.stringify(recipe), {
      id: "imported-flow", now: "2026-09-05T13:00:00.000Z",
    });

    expect(preview.flow.id).toBe("imported-flow");
    expect(preview.flow.isActive).toBe(false);
    expect(preview.missingBindings).toHaveLength(1);
    expect(preview.permissionsReset).toBe(true);
  });

  it("blocks embedded credentials instead of silently sharing them", () => {
    const unsafe = flow();
    unsafe.nodes[0]!.inputs = { apiKey: "private" };
    expect(() => buildFlowRecipe(unsafe, health, {
      recipeId: "unsafe", recipeVersion: "1.0.0", now: "2026-09-05T12:00:00.000Z",
    })).toThrow(/sensitive values/);
  });

  it("rejects a recipe whose declared binding does not match its graph placeholder", () => {
    const recipe = buildFlowRecipe(flow(), health, {
      recipeId: "break-scene", recipeVersion: "1.0.0", now: "2026-09-05T12:00:00.000Z",
    });
    recipe.flow.nodes[1]!.inputs.actionReference = { kind: "saved_action", id: "local-origin-id" };
    recipe.flow.nodes[1]!.uiData.customProperties.actionReference = {
      kind: "saved_action", id: "local-origin-id",
    };

    expect(() => parseFlowRecipe(JSON.stringify(recipe), {
      id: "imported-flow", now: "2026-09-05T13:00:00.000Z",
    })).toThrow(/invalid action dependency/);
  });

  it("blocks credentials stored as HTTP header rows", () => {
    const unsafe = flow();
    unsafe.nodes[0]!.inputs = {
      headers: [{ name: "Authorization", value: "Bearer private" }],
    };

    expect(() => buildFlowRecipe(unsafe, health, {
      recipeId: "unsafe-header", recipeVersion: "1.0.0", now: "2026-09-05T12:00:00.000Z",
    })).toThrow(/sensitive values/);
  });

  it("derives executable review from the graph even when imported review is forged empty", () => {
    const recipe = buildFlowRecipe(flow(), health, {
      recipeId: "break-scene", recipeVersion: "1.0.0", now: "2026-09-05T12:00:00.000Z",
    });
    recipe.flow.nodes.push({
      id: "http-node", nodeType: "http_request", name: "Post status", inputs: {}, outputs: [],
      position: [400, 0], uiData: { collapsed: false, selected: false, zIndex: 0, customProperties: {} },
      executionConfig: { retryCount: 0, retryDelayMs: 0, skipOnError: false, parallelExecution: false },
    });
    recipe.flow.nodes.push({
      id: "future-node", nodeType: "future-provider-operation", name: "Future operation", inputs: {}, outputs: [],
      position: [600, 0], uiData: { collapsed: false, selected: false, zIndex: 0, customProperties: {} },
      executionConfig: { retryCount: 0, retryDelayMs: 0, skipOnError: false, parallelExecution: false },
    });
    recipe.review.executableNodes = [];

    const preview = parseFlowRecipe(JSON.stringify(recipe), {
      id: "imported-flow", now: "2026-09-05T13:00:00.000Z",
    });

    expect(preview.executableNodes).toEqual(expect.arrayContaining([
      expect.objectContaining({ nodeId: "http-node", classification: "external_effect" }),
      expect.objectContaining({ nodeId: "future-node", classification: "unknown" }),
    ]));
  });

  it("rejects two binding slots that claim the same action node", () => {
    const recipe = buildFlowRecipe(flow(), health, {
      recipeId: "break-scene", recipeVersion: "1.0.0", now: "2026-09-05T12:00:00.000Z",
    });
    recipe.dependencies.push({ ...recipe.dependencies[0]!, slotId: "action-duplicate" });
    const actionNode = recipe.flow.nodes[1]!;
    actionNode.inputs.actionReference = { kind: "saved_action", id: "recipe-binding:action-duplicate" };
    actionNode.uiData.customProperties.actionReference = actionNode.inputs.actionReference;

    expect(() => parseFlowRecipe(JSON.stringify(recipe), {
      id: "imported-flow", now: "2026-09-05T13:00:00.000Z",
    })).toThrow(/duplicate dependencies/);
  });
});
