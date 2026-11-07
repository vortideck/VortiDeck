import type { FlowDefinition } from "../types/flow";
import type { FlowActionHealth } from "./flowRecipe";
import { deriveFlowGraphReview, type FlowReviewNode } from "./flowReview";

export interface FlowGraphValidation {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

export interface FlowRehearsalReport {
  mode: "read_only";
  externalEffects: false;
  flowRevision: string;
  generatedAt: string;
  ready: boolean;
  graph: FlowGraphValidation;
  actionHealth: FlowActionHealth[];
  blockers: string[];
  warnings: string[];
  executableNodes: FlowReviewNode[];
}

const READY_ACTION_STATUSES = new Set(["ready", "receipt_degraded"]);

export function buildReadOnlyRehearsal(
  flow: FlowDefinition,
  graph: FlowGraphValidation,
  actionHealth: FlowActionHealth[],
  flowRevision: string,
  generatedAt = new Date().toISOString(),
): FlowRehearsalReport {
  const actionBlockers = actionHealth
    .filter(({ preflight }) => !READY_ACTION_STATUSES.has(preflight.status))
    .map(({ nodeName, preflight }) => {
      const evidence = preflight.issues?.map(({ message }) => message).join("; ");
      return `${nodeName}: ${evidence || preflight.status.replace(/_/g, " ")}`;
    });
  const degraded = actionHealth
    .filter(({ preflight }) => preflight.status === "receipt_degraded")
    .map(({ nodeName }) => `${nodeName}: the provider can confirm acceptance, not final application.`);

  return {
    mode: "read_only",
    externalEffects: false,
    flowRevision,
    generatedAt,
    ready: graph.isValid && actionBlockers.length === 0,
    graph,
    actionHealth,
    blockers: [...graph.errors, ...actionBlockers],
    warnings: [...graph.warnings, ...degraded],
    executableNodes: deriveFlowGraphReview(flow)
      .filter((node) => node.classification !== "saved_action"),
  };
}
