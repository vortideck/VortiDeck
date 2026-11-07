import type {
  AiNodeBehavior,
  CanvasSettings,
  FlowConnection,
  FlowDefinition,
  FlowNodeDefinition,
  FlowNodeOutput,
  NodeExecutionConfig,
} from "../types/flow";

export const FLOW_DOCUMENT_VERSION = "1.0.0";
const LEGACY_FLOW_DOCUMENT_VERSIONS = new Set(["1"]);

const DEFAULT_EXECUTION_CONFIG: NodeExecutionConfig = {
  timeoutMs: undefined,
  retryCount: 0,
  retryDelayMs: 1000,
  skipOnError: false,
  parallelExecution: false,
};

export interface SerializableEditorPort {
  id: string;
  name: string;
  type: string;
  value?: unknown;
}

export interface SerializableEditorNode {
  id: string | number;
  type: string;
  text?: string;
  x: number;
  y: number;
  inputs?: SerializableEditorPort[] | Record<string, unknown>;
  outputs?: SerializableEditorPort[];
  isTool?: boolean;
  toolName?: string;
  isAiNode?: boolean;
  aiBehavior?: AiNodeBehavior;
  aiModel?: string;
  promptTemplate?: string;
  parameters?: {
    backendId?: string;
    actionReference?: unknown;
    [key: string]: unknown;
  };
  invocationMode?: boolean;
  minimized?: boolean;
  executionConfig?: Partial<NodeExecutionConfig>;
  /** Original persisted node. It preserves same-version fields the editor does not expose. */
  persistedDefinition?: FlowNodeDefinition;
}

export interface BuildFlowDocumentOptions {
  id: string;
  name: string;
  description?: string;
  isActive: boolean;
  nodes: SerializableEditorNode[];
  connections: FlowConnection[];
  startNodeId: string;
  canvasSettings: CanvasSettings;
  now: string;
  baseFlow?: FlowDefinition | null;
  /** Save As creates a new document identity while retaining graph metadata. */
  newIdentity?: boolean;
}

export interface EditorNodeIdProjection {
  canvasIdFor(persistedId: string): number;
  persistedIdFor(canvasId: number): string;
}

/**
 * The canvas currently uses numeric DOM/interaction IDs while persisted flow
 * IDs are opaque strings. Project every persisted ID to one deterministic,
 * collision-free canvas ID and retain the inverse for connections. Never use
 * parseInt here: "0", "01", "1", UUIDs and names are distinct identities.
 */
export function createEditorNodeIdProjection(
  nodes: Array<Pick<FlowNodeDefinition, "id">>,
): EditorNodeIdProjection {
  const canvasByPersisted = new Map<string, number>();
  const persistedByCanvas = new Map<number, string>();

  nodes.forEach((node, index) => {
    if (!node.id || canvasByPersisted.has(node.id)) {
      throw new Error(`Flow contains a missing or duplicate node ID "${node.id}".`);
    }
    const canvasId = index + 1;
    canvasByPersisted.set(node.id, canvasId);
    persistedByCanvas.set(canvasId, node.id);
  });

  return {
    canvasIdFor(persistedId: string): number {
      const canvasId = canvasByPersisted.get(persistedId);
      if (canvasId === undefined) {
        throw new Error(`Flow connection references missing node "${persistedId}".`);
      }
      return canvasId;
    },
    persistedIdFor(canvasId: number): string {
      const persistedId = persistedByCanvas.get(canvasId);
      if (persistedId === undefined) {
        throw new Error(`Flow editor references missing canvas node "${canvasId}".`);
      }
      return persistedId;
    },
  };
}

export function persistedEditorNodeId(
  node: Pick<SerializableEditorNode, "id" | "persistedDefinition">,
): string {
  return node.persistedDefinition?.id ?? node.id.toString();
}

export function assertSupportedFlowDocument(
  flow: Pick<FlowDefinition, "version">,
): void {
  if (
    flow.version !== FLOW_DOCUMENT_VERSION &&
    !LEGACY_FLOW_DOCUMENT_VERSIONS.has(flow.version)
  ) {
    throw new Error(
      `Unsupported flow document version "${flow.version || "missing"}". ` +
        `This VortiDeck build supports ${FLOW_DOCUMENT_VERSION}.`,
    );
  }
}

export function serializeEditorInputs(
  inputs: SerializableEditorNode["inputs"],
): Record<string, unknown> {
  if (!inputs) return {};
  if (!Array.isArray(inputs)) return { ...inputs };

  return Object.fromEntries(
    inputs.map((input, index) => [
      input.id || input.name || `input_${index}`,
      input.value !== undefined ? input.value : input,
    ]),
  );
}

function serializeOutput(
  output: SerializableEditorPort,
  persisted?: FlowNodeOutput,
): FlowNodeOutput {
  return {
    ...persisted,
    id: output.id || output.name,
    name: output.name || output.id,
    outputType: output.type || "string",
    description: persisted?.description,
  };
}

export function serializeEditorNode(
  node: SerializableEditorNode,
): FlowNodeDefinition {
  const persisted = node.persistedDefinition;
  const editorInputs = serializeEditorInputs(node.inputs);
  const persistedOutputs = new Map(
    (persisted?.outputs || []).map((output) => [output.id, output]),
  );
  const customProperties = {
    ...(persisted?.uiData?.customProperties || {}),
  };

  if (node.parameters?.actionReference !== undefined) {
    customProperties.actionReference = node.parameters.actionReference;
  } else {
    delete customProperties.actionReference;
  }
  if (node.invocationMode) customProperties.invocationMode = true;
  else delete customProperties.invocationMode;
  if (node.minimized) customProperties.minimized = true;
  else delete customProperties.minimized;

  return {
    ...persisted,
    id: persistedEditorNodeId(node),
    nodeType: node.parameters?.backendId || node.type,
    name: node.text || node.type,
    // Schema hydration exposes only inputs understood by this editor build.
    // Retain same-version inputs it cannot render while letting visible edits
    // replace their persisted values.
    inputs: {
      ...(persisted?.inputs || {}),
      ...editorInputs,
    },
    outputs: (node.outputs || []).map((output) =>
      serializeOutput(output, persistedOutputs.get(output.id || output.name)),
    ),
    aiBehavior: node.isAiNode && node.aiBehavior ? node.aiBehavior : undefined,
    aiModel: node.aiModel,
    promptTemplate: node.promptTemplate,
    toolName: node.isTool ? node.toolName || node.type : undefined,
    position: [node.x, node.y],
    uiData: {
      ...(persisted?.uiData || {}),
      collapsed: false,
      selected: false,
      zIndex: persisted?.uiData?.zIndex ?? 0,
      customProperties,
    },
    executionConfig: {
      ...DEFAULT_EXECUTION_CONFIG,
      ...(persisted?.executionConfig || {}),
      ...(node.executionConfig || {}),
    },
  };
}

export function buildFlowDocument(
  options: BuildFlowDocumentOptions,
): FlowDefinition {
  const { baseFlow } = options;
  if (baseFlow) assertSupportedFlowDocument(baseFlow);

  return {
    ...baseFlow,
    id: options.id,
    name: options.name,
    version: FLOW_DOCUMENT_VERSION,
    description: options.description,
    createdAt: options.newIdentity
      ? options.now
      : baseFlow?.createdAt || options.now,
    updatedAt: options.now,
    createdBy: baseFlow?.createdBy,
    tags: baseFlow?.tags ? [...baseFlow.tags] : [],
    nodes: options.nodes.map(serializeEditorNode),
    connections: options.connections.map((connection) => ({ ...connection })),
    startNodeId: options.startNodeId,
    metadata: {
      ...(baseFlow?.metadata || {}),
      canvasSettings: { ...options.canvasSettings },
      variables: { ...(baseFlow?.metadata?.variables || {}) },
      permissions: {
        public: false,
        collaborators: [],
        executionRoles: [],
        ...(baseFlow?.metadata?.permissions || {}),
      },
      executionSettings: {
        maxExecutionTimeMs: undefined,
        maxConcurrentNodes: 5,
        autoSaveIntervalMs: 30000,
        ...(baseFlow?.metadata?.executionSettings || {}),
      },
    },
    isActive: options.newIdentity ? false : options.isActive,
  };
}

/**
 * Import creates a new local draft identity without rebuilding the graph.
 * Keeping the graph object intact preserves forward-compatible node fields;
 * identity, activation authority, and document-version migration are explicit.
 */
export function reidentifyImportedFlow(
  flow: FlowDefinition,
  identity: { id: string; now: string },
): FlowDefinition {
  assertSupportedFlowDocument(flow);
  return {
    ...flow,
    id: identity.id,
    version: FLOW_DOCUMENT_VERSION,
    createdAt: identity.now,
    updatedAt: identity.now,
    isActive: false,
    nodes: flow.nodes.map((node) => ({ ...node })),
    connections: flow.connections.map((connection) => ({ ...connection })),
    tags: [...(flow.tags || [])],
    metadata: {
      ...flow.metadata,
      canvasSettings: { ...flow.metadata.canvasSettings },
      variables: { ...(flow.metadata.variables || {}) },
      permissions: {
        ...flow.metadata.permissions,
        collaborators: [...(flow.metadata.permissions?.collaborators || [])],
        executionRoles: [...(flow.metadata.permissions?.executionRoles || [])],
      },
      executionSettings: { ...flow.metadata.executionSettings },
    },
  };
}

/** Stable semantic payload for optimistic edit/revision checks. */
export function flowRevisionPayload(flow: FlowDefinition): string {
  const { updatedAt: _updatedAt, ...semanticDocument } = flow;
  return stableStringify(semanticDocument);
}

export async function flowRevision(flow: FlowDefinition): Promise<string> {
  const bytes = new TextEncoder().encode(flowRevisionPayload(flow));
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}

function stableStringify(value: unknown): string {
  if (value === undefined) return "null";
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;

  const record = value as Record<string, unknown>;
  return `{${Object.keys(record)
    .filter((key) => record[key] !== undefined)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(",")}}`;
}
