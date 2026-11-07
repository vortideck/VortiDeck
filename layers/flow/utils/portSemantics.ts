import type {
  FlowCanvasConnection,
  FlowConnection,
} from "../types/flow";

export const DEFAULT_FLOW_OUTPUT_ID = "output";
const FLOW_OUTPUT_PREFIX = "flow-output-";
const VALUE_OUTPUT_PREFIX = "value-output-";
const VALUE_INPUT_PREFIX = "value-input-";

interface FlowOutputLike {
  id: string;
  type: string;
}

/**
 * The default control output is represented by `flow-output`. Only explicit
 * branch outputs (approved, denied, true_branch, and so on) receive a suffix.
 */
export const getNamedFlowRouteOutputs = <T extends FlowOutputLike>(
  outputs: T[] | undefined,
): T[] =>
  (outputs ?? []).filter(
    (output) => output.type === "flow" && output.id !== DEFAULT_FLOW_OUTPUT_ID,
  );

/** Canonicalize graphs created while the default output was treated as a route. */
export const canonicalizeFlowOutputPort = (portType: string): string =>
  portType === `flow-output-${DEFAULT_FLOW_OUTPUT_ID}` ? "flow-output" : portType;

const requiredString = (
  value: unknown,
  property: string,
): string => {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Invalid flow connection: ${property} must be a non-empty string`);
  }
  return value;
};

const controlRouteFromPort = (portType: string): string | undefined => {
  const canonicalPort = canonicalizeFlowOutputPort(portType);
  if (canonicalPort === "flow-output") return undefined;
  if (!canonicalPort.startsWith(FLOW_OUTPUT_PREFIX)) {
    throw new Error(`Invalid control output port: ${portType}`);
  }

  const route = canonicalPort.slice(FLOW_OUTPUT_PREFIX.length);
  return requiredString(route, "route");
};

/**
 * Convert a validated canvas gesture into the semantic edge used by the editor.
 * Port-name parsing is deliberately confined to this UI boundary.
 */
export const createFlowCanvasConnection = (
  id: number,
  from: number,
  to: number,
  fromType: string,
  toType: string,
): FlowCanvasConnection => {
  const canonicalFromType = canonicalizeFlowOutputPort(fromType);

  if (canonicalFromType.startsWith("flow-output") && toType === "flow-input") {
    const route = controlRouteFromPort(canonicalFromType);
    return {
      kind: "control",
      id,
      from,
      to,
      fromType: route ? `${FLOW_OUTPUT_PREFIX}${route}` : "flow-output",
      toType: "flow-input",
      ...(route ? { route } : {}),
    };
  }

  if (
    canonicalFromType.startsWith(VALUE_OUTPUT_PREFIX)
    && toType.startsWith(VALUE_INPUT_PREFIX)
  ) {
    const fromOutput = requiredString(
      canonicalFromType.slice(VALUE_OUTPUT_PREFIX.length),
      "fromOutput",
    );
    const toInput = requiredString(toType.slice(VALUE_INPUT_PREFIX.length), "toInput");
    return {
      kind: "data",
      id,
      from,
      to,
      fromType: `${VALUE_OUTPUT_PREFIX}${fromOutput}`,
      toType: `${VALUE_INPUT_PREFIX}${toInput}`,
      fromOutput,
      toInput,
    };
  }

  throw new Error(`Incompatible flow connection ports: ${fromType} -> ${toType}`);
};

export const canvasConnectionToFlowConnection = (
  connection: FlowCanvasConnection,
  resolveNodeId: (canvasId: number) => string = (canvasId) => canvasId.toString(),
): FlowConnection => {
  if (connection.kind === "control") {
    return {
      kind: "control",
      fromNode: resolveNodeId(connection.from),
      toNode: resolveNodeId(connection.to),
      ...(connection.route ? { route: connection.route } : {}),
    };
  }

  return {
    kind: "data",
    fromNode: resolveNodeId(connection.from),
    fromOutput: connection.fromOutput,
    toNode: resolveNodeId(connection.to),
    toInput: connection.toInput,
  };
};

export const flowConnectionToCanvasConnection = (
  connection: FlowConnection,
  id: number,
  resolveNodeId?: (persistedId: string) => number,
): FlowCanvasConnection => {
  const from = resolveNodeId
    ? resolveNodeId(connection.fromNode)
    : Number.parseInt(connection.fromNode, 10);
  const to = resolveNodeId
    ? resolveNodeId(connection.toNode)
    : Number.parseInt(connection.toNode, 10);
  if (!Number.isSafeInteger(from) || !Number.isSafeInteger(to)) {
    throw new Error(
      `Flow editor requires numeric node IDs; received ${connection.fromNode} -> ${connection.toNode}`,
    );
  }

  if (connection.kind === "control") {
    return {
      kind: "control",
      id,
      from,
      to,
      fromType: connection.route
        ? `${FLOW_OUTPUT_PREFIX}${connection.route}`
        : "flow-output",
      toType: "flow-input",
      ...(connection.route ? { route: connection.route } : {}),
    };
  }

  return {
    kind: "data",
    id,
    from,
    to,
    fromType: `${VALUE_OUTPUT_PREFIX}${connection.fromOutput}`,
    toType: `${VALUE_INPUT_PREFIX}${connection.toInput}`,
    fromOutput: connection.fromOutput,
    toInput: connection.toInput,
  };
};

type UnknownRecord = Record<string, unknown>;

const isRecord = (value: unknown): value is UnknownRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/**
 * Decode the current edge format and migrate the former four-port format.
 * Callers always receive the current typed representation.
 */
export const normalizeFlowConnection = (value: unknown): FlowConnection => {
  if (!isRecord(value)) {
    throw new Error("Invalid flow connection: expected an object");
  }

  if (value.kind === "control") {
    const route = value.route == null
      ? undefined
      : requiredString(value.route, "route");
    return {
      kind: "control",
      fromNode: requiredString(value.fromNode, "fromNode"),
      toNode: requiredString(value.toNode, "toNode"),
      ...(route && route !== DEFAULT_FLOW_OUTPUT_ID ? { route } : {}),
    };
  }

  if (value.kind === "data") {
    return {
      kind: "data",
      fromNode: requiredString(value.fromNode, "fromNode"),
      fromOutput: requiredString(value.fromOutput, "fromOutput"),
      toNode: requiredString(value.toNode, "toNode"),
      toInput: requiredString(value.toInput, "toInput"),
    };
  }

  if ("kind" in value) {
    throw new Error(`Invalid flow connection kind: ${String(value.kind)}`);
  }

  const fromNode = requiredString(value.fromNode, "fromNode");
  const toNode = requiredString(value.toNode, "toNode");
  const fromPort = requiredString(value.fromPort, "fromPort");
  const toPort = requiredString(value.toPort, "toPort");

  if (fromPort.startsWith("flow-output") && toPort === "flow-input") {
    const route = controlRouteFromPort(fromPort);
    return {
      kind: "control",
      fromNode,
      toNode,
      ...(route ? { route } : {}),
    };
  }
  if (fromPort === "flow-input" && toPort.startsWith("flow-output")) {
    const route = controlRouteFromPort(toPort);
    return {
      kind: "control",
      fromNode: toNode,
      toNode: fromNode,
      ...(route ? { route } : {}),
    };
  }
  if (fromPort.startsWith(VALUE_OUTPUT_PREFIX) && toPort.startsWith(VALUE_INPUT_PREFIX)) {
    return {
      kind: "data",
      fromNode,
      fromOutput: requiredString(fromPort.slice(VALUE_OUTPUT_PREFIX.length), "fromOutput"),
      toNode,
      toInput: requiredString(toPort.slice(VALUE_INPUT_PREFIX.length), "toInput"),
    };
  }
  if (fromPort.startsWith(VALUE_INPUT_PREFIX) && toPort.startsWith(VALUE_OUTPUT_PREFIX)) {
    return {
      kind: "data",
      fromNode: toNode,
      fromOutput: requiredString(toPort.slice(VALUE_OUTPUT_PREFIX.length), "fromOutput"),
      toNode: fromNode,
      toInput: requiredString(fromPort.slice(VALUE_INPUT_PREFIX.length), "toInput"),
    };
  }
  if (!fromPort.includes("flow") && !toPort.includes("flow")) {
    return {
      kind: "data",
      fromNode,
      fromOutput: fromPort,
      toNode,
      toInput: toPort,
    };
  }

  throw new Error(`Invalid legacy flow connection: ${fromPort} -> ${toPort}`);
};

export const normalizeFlowConnections = (values: unknown[]): FlowConnection[] =>
  values.map(normalizeFlowConnection);
