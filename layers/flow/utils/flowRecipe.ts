import type { ActionPreflight } from "@base/types/actionReference";
import type { FlowDefinition } from "../types/flow";
import {
  FLOW_DOCUMENT_VERSION,
  assertSupportedFlowDocument,
  reidentifyImportedFlow,
} from "./flowSerialization";
import { deriveFlowGraphReview, type FlowReviewNode } from "./flowReview";

export const FLOW_RECIPE_FORMAT = "vortideck.flow-recipe";
export const FLOW_RECIPE_VERSION = 1;

export interface FlowActionHealth {
  nodeId: string;
  nodeName: string;
  stableActionId: string;
  preflight: ActionPreflight;
}

export interface FlowRecipeDependency {
  slotId: string;
  nodeId: string;
  nodeName: string;
  kind: "action_binding";
  integrationId: string;
  actionId: string;
  capabilityVersion?: number;
  receiptSemantics?: unknown;
  requiredPermission?: unknown;
}

export interface FlowRecipeEnvelope {
  format: typeof FLOW_RECIPE_FORMAT;
  version: typeof FLOW_RECIPE_VERSION;
  recipe: {
    id: string;
    version: string;
    name: string;
    description?: string;
    createdAt: string;
  };
  flow: FlowDefinition;
  dependencies: FlowRecipeDependency[];
  review: {
    executableNodes: FlowReviewNode[];
    permissionsResetOnImport: true;
  };
}

export interface FlowRecipeImportPreview {
  envelope: FlowRecipeEnvelope;
  flow: FlowDefinition;
  sourceFlowId: string;
  importedFlowId: string;
  missingBindings: FlowRecipeDependency[];
  executableNodes: FlowRecipeEnvelope["review"]["executableNodes"];
  permissionsReset: boolean;
}

const SENSITIVE_QUERY = /[?&](authorization|password|secret|token|api_?key|access_?token|refresh_?token|client_?secret|overlay_?key)=/i;
const SENSITIVE_AUTH_VALUE = /^(basic|bearer)\s+\S+/i;
const URL_USER_INFO = /^[a-z][a-z0-9+.-]*:\/\/[^/\s:@]+:[^/\s@]+@/i;
const SENSITIVE_KEYS = new Set([
  "authorization",
  "proxyauthorization",
  "xauthtoken",
  "cookie",
  "password",
  "secret",
  "token",
  "apikey",
  "privatekey",
  "accesstoken",
  "refreshtoken",
  "clientsecret",
]);

function isSensitiveKey(key: string): boolean {
  const normalized = key.replace(/[-_]/g, "").toLowerCase();
  return SENSITIVE_KEYS.has(normalized)
    || normalized.endsWith("apikey")
    || normalized.endsWith("accesstoken")
    || normalized.endsWith("refreshtoken")
    || normalized.endsWith("clientsecret");
}

function cloneFlow(flow: FlowDefinition): FlowDefinition {
  return structuredClone(flow);
}

function sensitivePaths(value: unknown, path = "flow"): string[] {
  if (typeof value === "string") {
    return SENSITIVE_QUERY.test(value)
      || SENSITIVE_AUTH_VALUE.test(value)
      || URL_USER_INFO.test(value)
      || value.startsWith('vortideck-credential://')
      ? [path]
      : [];
  }
  if (!value || typeof value !== "object") return [];
  if (Array.isArray(value)) {
    return value.flatMap((item, index) => sensitivePaths(item, `${path}[${index}]`));
  }

  const record = value as Record<string, unknown>;
  if (record.type === 'custom_headers' && record.headers && typeof record.headers === 'object'
    && Object.values(record.headers).some(value => value !== null && value !== undefined && value !== '')) {
    return [`${path}.headers`];
  }
  const headerName = typeof record.name === "string"
    ? record.name
    : typeof record.key === "string"
      ? record.key
      : undefined;
  const headerValue = record.value;
  const rowSecret = headerName && isSensitiveKey(headerName)
    && headerValue !== null && headerValue !== undefined && headerValue !== ""
    ? [`${path}.value`]
    : [];

  return [...rowSecret, ...Object.entries(record).flatMap(([key, item]) => {
    const itemPath = `${path}.${key}`;
    if (isSensitiveKey(key) && item !== null && item !== undefined && item !== "") {
      return [itemPath];
    }
    return sensitivePaths(item, itemPath);
  })];
}

function replaceActionReference(
  flow: FlowDefinition,
  nodeId: string,
  referenceId: string,
): void {
  const node = flow.nodes.find((candidate) => candidate.id === nodeId);
  if (!node) throw new Error(`Recipe dependency references missing node "${nodeId}".`);
  const reference = { kind: "saved_action", id: referenceId };
  node.inputs = { ...node.inputs, actionReference: reference };
  node.uiData = {
    ...node.uiData,
    customProperties: { ...node.uiData.customProperties, actionReference: reference },
  };
}

function actionReferenceId(flow: FlowDefinition, nodeId: string): string | undefined {
  const node = flow.nodes.find((candidate) => candidate.id === nodeId);
  const reference = node?.inputs?.actionReference
    || node?.uiData?.customProperties?.actionReference;
  return typeof reference?.id === "string" ? reference.id : undefined;
}

export function assertSafeFlowExport(source: FlowDefinition): void {
  const unsafePaths = sensitivePaths(source);
  if (unsafePaths.length > 0) {
    throw new Error(`Flow export blocked because sensitive values or local credential references may be embedded at: ${unsafePaths.join(', ')}. Remove them from the export copy and configure credentials after import.`);
  }
}

export function buildFlowRecipe(
  source: FlowDefinition,
  actionHealth: FlowActionHealth[],
  options: { recipeId: string; recipeVersion: string; now: string },
): FlowRecipeEnvelope {
  assertSupportedFlowDocument(source);
  assertSafeFlowExport(source);

  const actionNodeIds = source.nodes
    .filter((node) => node.nodeType === "invoke-action")
    .map((node) => node.id)
    .sort();
  const healthNodeIds = actionHealth.map(({ nodeId }) => nodeId).sort();
  if (
    new Set(healthNodeIds).size !== healthNodeIds.length
    || JSON.stringify(actionNodeIds) !== JSON.stringify(healthNodeIds)
  ) {
    throw new Error("Recipe export requires exactly one resolved preflight result per action node.");
  }

  const flow = cloneFlow(source);
  flow.id = "recipe-flow";
  flow.version = FLOW_DOCUMENT_VERSION;
  flow.createdAt = options.now;
  flow.updatedAt = options.now;
  flow.createdBy = undefined;
  flow.isActive = false;
  flow.metadata = {
    ...flow.metadata,
    permissions: { public: false, collaborators: [], executionRoles: [] },
  };

  const dependencies = actionHealth.map((health, index): FlowRecipeDependency => {
    const slotId = `action-${index + 1}`;
    replaceActionReference(flow, health.nodeId, `recipe-binding:${slotId}`);
    const contract = health.preflight.capability?.contract || {};
    return {
      slotId,
      nodeId: health.nodeId,
      nodeName: health.nodeName,
      kind: "action_binding",
      integrationId: health.preflight.integrationId,
      actionId: health.preflight.actionId,
      capabilityVersion: health.preflight.capability?.version,
      receiptSemantics: contract.receipt,
      requiredPermission: contract.requiredPermission,
    };
  });

  const executableNodes = deriveFlowGraphReview(flow)
    .filter((node) => node.classification !== "saved_action");

  return {
    format: FLOW_RECIPE_FORMAT,
    version: FLOW_RECIPE_VERSION,
    recipe: {
      id: options.recipeId,
      version: options.recipeVersion,
      name: source.name,
      description: source.description,
      createdAt: options.now,
    },
    flow,
    dependencies,
    review: { executableNodes, permissionsResetOnImport: true },
  };
}

export function parseFlowRecipe(
  serialized: string,
  identity: { id: string; now: string },
): FlowRecipeImportPreview {
  const envelope = JSON.parse(serialized) as FlowRecipeEnvelope;
  if (envelope?.format !== FLOW_RECIPE_FORMAT || envelope.version !== FLOW_RECIPE_VERSION) {
    throw new Error("Unsupported VortiDeck recipe format or version.");
  }
  if (!envelope.recipe?.id || !envelope.recipe?.version || !envelope.flow) {
    throw new Error("Recipe metadata is incomplete.");
  }
  if (!Array.isArray(envelope.dependencies) || !Array.isArray(envelope.review?.executableNodes)) {
    throw new Error("Recipe dependency review is incomplete.");
  }
  assertSupportedFlowDocument(envelope.flow);

  const dependencySlots = new Set(envelope.dependencies.map((dependency) => dependency.slotId));
  if (dependencySlots.size !== envelope.dependencies.length) {
    throw new Error("Recipe contains duplicate dependency slots.");
  }
  const dependencyNodes = new Set(envelope.dependencies.map(({ nodeId }) => nodeId));
  if (dependencyNodes.size !== envelope.dependencies.length) {
    throw new Error("Recipe contains duplicate dependencies for one action node.");
  }
  for (const dependency of envelope.dependencies) {
    if (
      !dependency.slotId
      || !dependency.nodeId
      || !dependency.integrationId
      || !dependency.actionId
      || dependency.kind !== "action_binding"
      || !envelope.flow.nodes.some(
        (node) => node.id === dependency.nodeId && node.nodeType === "invoke-action",
      )
      || actionReferenceId(envelope.flow, dependency.nodeId) !== `recipe-binding:${dependency.slotId}`
    ) {
      throw new Error("Recipe contains an invalid action dependency.");
    }
  }
  if (envelope.flow.nodes.some(
    (node) => node.nodeType === "invoke-action" && !dependencyNodes.has(node.id),
  )) {
    throw new Error("Recipe contains an action node without a portable binding requirement.");
  }
  const unsafePaths = sensitivePaths(envelope.flow);
  if (unsafePaths.length > 0) {
    throw new Error("Recipe import blocked because the graph contains embedded sensitive values.");
  }

  const flow = reidentifyImportedFlow(cloneFlow(envelope.flow), identity);
  flow.metadata.permissions = { public: false, collaborators: [], executionRoles: [] };
  flow.isActive = false;

  return {
    envelope,
    flow,
    sourceFlowId: envelope.flow.id,
    importedFlowId: flow.id,
    missingBindings: envelope.dependencies,
    executableNodes: deriveFlowGraphReview(envelope.flow)
      .filter((node) => node.classification !== "saved_action"),
    permissionsReset: true,
  };
}

export function isFlowRecipe(serialized: string): boolean {
  try {
    return JSON.parse(serialized)?.format === FLOW_RECIPE_FORMAT;
  } catch {
    return false;
  }
}
