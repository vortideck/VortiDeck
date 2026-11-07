import type { FlowDefinition, FlowNodeDefinition } from "../types/flow";

export type FlowReviewClassification =
  | "saved_action"
  | "external_effect"
  | "ai"
  | "plugin"
  | "unknown";

export interface FlowReviewNode {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  classification: FlowReviewClassification;
  reason: string;
}

// These nodes only route, wait, inspect or mutate run-local state. Everything
// else is reviewed explicitly. That default is deliberate: an imported node
// introduced by a newer host must never become "safe" merely because this
// editor does not recognize its name.
const NON_EFFECTFUL_NODE_TYPES = new Set([
  "approval",
  "branch",
  "button-trigger",
  "delay",
  "get-variable",
  "set-variable",
  "webhook-trigger",
]);

const EXTERNAL_EFFECT_REASONS: Record<string, string> = {
  "audio-play": "Controls host audio output.",
  "audio-stop": "Controls host audio output.",
  "display-text": "Publishes display output outside run-local state.",
  "download-file": "Downloads and writes external data.",
  "http_request": "Makes an outbound network request.",
  "load-file": "Reads a local file.",
  "preview-file": "Opens or presents local file content.",
  "preview-image": "Opens or presents local image content.",
  "respond-to-webhook": "Sends a response to an external webhook caller.",
  "save-file": "Writes a local file.",
  "upload-file": "Uploads local data to an external destination.",
};

const AI_NODE_TYPES = new Set([
  "ai-orchestrator",
  "ai-text-generator",
  "hybrid-llm-free",
]);

function reviewNode(node: FlowNodeDefinition): FlowReviewNode | null {
  if (node.nodeType === "invoke-action") {
    return {
      nodeId: node.id,
      nodeName: node.name,
      nodeType: node.nodeType,
      classification: "saved_action",
      reason: "Invokes a provider-owned saved-action binding that must be repaired locally.",
    };
  }

  if (node.nodeType.startsWith("plugin.")) {
    return {
      nodeId: node.id,
      nodeName: node.name,
      nodeType: node.nodeType,
      classification: "plugin",
      reason: "Runs code supplied by an installed package under that package's grants.",
    };
  }

  if (node.aiBehavior || AI_NODE_TYPES.has(node.nodeType)) {
    return {
      nodeId: node.id,
      nodeName: node.name,
      nodeType: node.nodeType,
      classification: "ai",
      reason: "Uses an AI model; connected tools and provider data require review.",
    };
  }

  const effectReason = EXTERNAL_EFFECT_REASONS[node.nodeType];
  if (effectReason) {
    return {
      nodeId: node.id,
      nodeName: node.name,
      nodeType: node.nodeType,
      classification: "external_effect",
      reason: effectReason,
    };
  }

  if (NON_EFFECTFUL_NODE_TYPES.has(node.nodeType)) return null;

  return {
    nodeId: node.id,
    nodeName: node.name,
    nodeType: node.nodeType,
    classification: "unknown",
    reason: "This editor does not recognize the node's effect contract; review it before enabling the flow.",
  };
}

/** Derive review from the validated graph. Imported review metadata is never authoritative. */
export function deriveFlowGraphReview(flow: FlowDefinition): FlowReviewNode[] {
  return flow.nodes
    .map(reviewNode)
    .filter((node): node is FlowReviewNode => node !== null);
}
