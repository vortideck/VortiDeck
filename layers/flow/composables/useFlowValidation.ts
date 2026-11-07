import { type Ref } from "vue";
import type { FlowCanvasConnection } from "../types/flow";

// Local UI types for validation
interface Node {
  id: number;
  type: string;
  text: string;
  x: number;
  y: number;
  inputs?: NodeIO[];
  outputs?: NodeIO[];
  isTool?: boolean;
  toolName?: string;
  parameters?: any;
  aiBehavior?: any;
  isAiNode?: boolean;
  aiModel?: string;
  promptTemplate?: string;
  executionState?: any;
  actionPreflight?: import("@base/types/actionReference").ActionPreflight;
}

type Connection = FlowCanvasConnection;

interface NodeIO {
  id: string;
  name: string;
  type: string;
  value?: any;
  attached?: boolean;
  enum?: string[];
}

interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

interface FlowValidationState {
  nodes: Ref<Node[]>;
  connections: Ref<Connection[]>;
  nodeTypes: Ref<any[]>;
}

export const useFlowValidation = (state: FlowValidationState) => {
  // Value-driven nodes are resolved from data dependencies and do not require
  // explicit flow-order incoming edges.
  const isValueDrivenNode = (nodeType: string): boolean => {
    return nodeType === "get-variable" || nodeType === "set-variable";
  };
  
  // Check if a node is a trigger node
  const isTriggerNode = (nodeType: string): boolean => {
    return state.nodeTypes.value.some(
      (type) => type.name === nodeType && type.category === "TRIGGER"
    );
  };

  // Find all trigger nodes in the flow
  const findTriggerNodes = (): Node[] => {
    return state.nodes.value.filter((node) => isTriggerNode(node.type));
  };

  // Find nodes that have no incoming connections (potential starting points)
  const findStartingNodes = (): Node[] => {
    const nodesWithIncomingConnections = new Set(
      state.connections.value
        .filter((conn) => conn.kind === "control")
        .map((conn) => conn.to)
    );
    
    return state.nodes.value.filter(
      (node) => !nodesWithIncomingConnections.has(node.id)
    );
  };

  // Find disconnected nodes (nodes with no connections at all)
  const findDisconnectedNodes = (): Node[] => {
    const connectedNodes = new Set([
      ...state.connections.value.map((conn) => conn.from),
      ...state.connections.value.map((conn) => conn.to),
    ]);

    return state.nodes.value.filter(
      (node) => !connectedNodes.has(node.id)
    );
  };

  // Validate flow structure for execution
  const validateFlowForExecution = (): ValidationResult => {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Check if flow has any nodes
    if (state.nodes.value.length === 0) {
      errors.push("Flow is empty. Add at least one node to execute.");
      return { isValid: false, errors, warnings };
    }

    const unhealthyActions = state.nodes.value.filter((node) =>
      node.type === "invoke-action"
      && node.actionPreflight
      && !["ready", "receipt_degraded"].includes(node.actionPreflight.status),
    );
    if (unhealthyActions.length > 0) {
      errors.push(
        `${unhealthyActions.length} action node(s) need repair before activation: ${unhealthyActions
          .map((node) => `${node.text} (${node.actionPreflight?.status.replace(/_/g, " ")})`)
          .join(", ")}.`,
      );
    }
    const degradedActions = state.nodes.value.filter((node) =>
      node.type === "invoke-action" && node.actionPreflight?.status === "receipt_degraded",
    );
    if (degradedActions.length > 0) {
      warnings.push(
        `${degradedActions.length} action node(s) only confirm transport acceptance; final provider application cannot be proven.`,
      );
    }

    // Find trigger nodes
    const triggerNodes = findTriggerNodes();
    
    // Check for proper entry point (only trigger nodes should be entry points)
    if (triggerNodes.length === 0) {
      errors.push("Flow must have at least one trigger node to start execution.");
    } else if (triggerNodes.length > 1) {
      warnings.push(`Flow has ${triggerNodes.length} trigger nodes. Only the first trigger will be used as entry point.`);
    } else {
      console.log(`✅ Found trigger node "${triggerNodes[0]!.text}" as entry point`);
    }

    // Find completely disconnected nodes (INFORMATIONAL ONLY)
    const disconnectedNodes = findDisconnectedNodes();
    if (disconnectedNodes.length > 0) {
      const disconnectedNonTriggerNodes = disconnectedNodes.filter(node => !isTriggerNode(node.type));
      
      if (disconnectedNonTriggerNodes.length > 0) {
        warnings.push(
          `${disconnectedNonTriggerNodes.length} node(s) are disconnected from the main flow: ${disconnectedNonTriggerNodes
            .map((node) => node.text)
            .join(", ")}. They will be ignored during execution.`
        );
      }
    }

    // The control scheduler executes a DAG. Data dependencies are validated
    // separately and do not turn into control-flow cycles.
    const hasCycles = detectSimpleCycles();
    if (hasCycles) {
      errors.push("Control flow contains a cycle. Remove the cycle before execution.");
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
    };
  };

  // Validate flow structure for saving
  const validateFlowForSaving = (): ValidationResult => {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Less strict validation for saving (allow empty flows, etc.)
    if (state.nodes.value.length === 0) {
      warnings.push("Flow is empty.");
      return { isValid: true, errors, warnings };
    }

    // Find trigger nodes
    const triggerNodes = findTriggerNodes();
    
    // Allow multiple entry points for saving (more flexible)
    const startingNodes = findStartingNodes();
    
    if (startingNodes.length > 0) {
      const triggers = triggerNodes.length;
      const disconnected = startingNodes.filter(
        (node) => !isTriggerNode(node.type) && !isValueDrivenNode(node.type)
      ).length;
      
      if (triggers > 1) {
        warnings.push(`Flow has ${triggers} trigger nodes. Multiple triggers will be executed in parallel.`);
      }
      if (disconnected > 0) {
        warnings.push(`Flow has ${disconnected} disconnected starting nodes that will be executed as separate entry points.`);
      }
    }

    // Find completely disconnected nodes (INFORMATIONAL ONLY)
    const disconnectedNodes = findDisconnectedNodes();
    if (disconnectedNodes.length > 0) {
      const disconnectedNonTriggerNodes = disconnectedNodes.filter(node => !isTriggerNode(node.type));
      
      if (disconnectedNonTriggerNodes.length > 0) {
        warnings.push(
          `${disconnectedNonTriggerNodes.length} node(s) are disconnected from the main flow: ${disconnectedNonTriggerNodes
            .map((node) => node.text)
            .join(", ")}. They will be ignored during execution.`
        );
      }
    }

    if (detectSimpleCycles()) {
      warnings.push("Control flow contains a cycle and cannot be activated or executed yet.");
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
    };
  };

  // Cycle detection over control edges only.
  const detectSimpleCycles = (): boolean => {
    const visited = new Set<number>();
    const recursionStack = new Set<number>();

    const hasCycleDFS = (nodeId: number): boolean => {
      if (recursionStack.has(nodeId)) {
        return true; // Cycle detected
      }
      if (visited.has(nodeId)) {
        return false; // Already processed
      }

      visited.add(nodeId);
      recursionStack.add(nodeId);

      // Check all connected nodes
      const outgoingConnections = state.connections.value.filter(
        (conn) => conn.kind === "control" && conn.from === nodeId
      );

      for (const connection of outgoingConnections) {
        if (hasCycleDFS(connection.to)) {
          return true;
        }
      }

      recursionStack.delete(nodeId);
      return false;
    };

    // Check from all trigger nodes
    const triggerNodes = findTriggerNodes();
    for (const triggerNode of triggerNodes) {
      if (hasCycleDFS(triggerNode.id)) {
        return true;
      }
    }

    return false;
  };

  // Get a summary of flow structure
  const getFlowSummary = () => {
    const triggerNodes = findTriggerNodes();
    const startingNodes = findStartingNodes();
    const disconnectedNodes = findDisconnectedNodes();

    return {
      totalNodes: state.nodes.value.length,
      totalConnections: state.connections.value.length,
      triggerNodes: triggerNodes.length,
      startingPoints: startingNodes.length,
      disconnectedNodes: disconnectedNodes.length,
      triggerNodeNames: triggerNodes.map((node) => node.text),
      startingNodeNames: startingNodes.map((node) => node.text),
      disconnectedNodeNames: disconnectedNodes.map((node) => node.text),
    };
  };

  return {
    validateFlowForExecution,
    validateFlowForSaving,
    findTriggerNodes,
    findStartingNodes,
    findDisconnectedNodes,
    getFlowSummary,
    isTriggerNode,
  };
}; 
