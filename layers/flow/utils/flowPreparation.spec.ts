import { describe, expect, it } from "vitest";
import type { FlowDefinition } from "../types/flow";
import { buildReadOnlyRehearsal } from "./flowPreparation";

const flow = (): FlowDefinition => ({
  id: "flow-1",
  name: "Break scene",
  version: "1.0.0",
  createdAt: "2026-09-05T12:00:00.000Z",
  updatedAt: "2026-09-05T12:00:00.000Z",
  tags: [],
  startNodeId: "1",
  isActive: false,
  connections: [],
  nodes: [{
    id: "1", nodeType: "invoke-action", name: "Switch scene", inputs: {}, outputs: [],
    position: [0, 0], uiData: { collapsed: false, selected: false, zIndex: 0, customProperties: {} },
    executionConfig: { retryCount: 0, retryDelayMs: 0, skipOnError: false, parallelExecution: false },
  }],
  metadata: {
    canvasSettings: { zoom: 1, panX: 0, panY: 0, gridEnabled: true, snapToGrid: false },
    variables: {}, permissions: { public: false, collaborators: [], executionRoles: [] },
    executionSettings: { maxConcurrentNodes: 1, autoSaveIntervalMs: 0 },
  },
});

describe("buildReadOnlyRehearsal", () => {
  it("records a zero-effect report without invoking a provider", () => {
    const report = buildReadOnlyRehearsal(
      flow(), { isValid: true, errors: [], warnings: [] }, [{
        nodeId: "1", nodeName: "Switch scene", stableActionId: "obs-action",
        preflight: { integrationId: "obs", actionId: "set_scene", status: "ready", issues: [] },
      }], "revision-1", "2026-09-05T13:00:00.000Z",
    );

    expect(report).toMatchObject({ mode: "read_only", externalEffects: false, ready: true });
    expect(report.flowRevision).toBe("revision-1");
  });

  it("blocks a missing local action binding with provider evidence", () => {
    const report = buildReadOnlyRehearsal(
      flow(), { isValid: true, errors: [], warnings: [] }, [{
        nodeId: "1", nodeName: "Switch scene", stableActionId: "missing",
        preflight: {
          integrationId: "", actionId: "missing", status: "missing_target",
          issues: [{ code: "missing_saved_action", message: "Binding is not installed", recoverable: true }],
        },
      }], "revision-2",
    );

    expect(report.ready).toBe(false);
    expect(report.blockers).toContain("Switch scene: Binding is not installed");
  });

  it("allows a degraded receipt while explaining its evidence limit", () => {
    const report = buildReadOnlyRehearsal(
      flow(), { isValid: true, errors: [], warnings: [] }, [{
        nodeId: "1", nodeName: "Switch scene", stableActionId: "obs-action",
        preflight: { integrationId: "obs", actionId: "set_scene", status: "receipt_degraded" },
      }], "revision-3",
    );

    expect(report.ready).toBe(true);
    expect(report.warnings[0]).toMatch(/acceptance, not final application/);
  });
});
