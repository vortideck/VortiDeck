import { type Ref } from "vue";
import type {
  AiNodeBehavior,
  FlowCanvasConnection,
  NodeExecutionConfig,
} from "../types/flow";
import { useAIModelsStore } from "@/stores/AIModels";
import {
  convertJsonSchemaToNodeInputs,
  getDefaultValueForSchemaType,
} from "../utils/schemaInputs";

/**
 * Get the default AI model for new AI nodes
 * Uses the default_model from the first provider config in settings
 */
function getDefaultAiModel(): string | undefined {
  const aiModelsStore = useAIModelsStore();
  return aiModelsStore.defaultModel;
}



// Local UI types for the flow builder (not part of the unified flow format)
interface Node {
  id: number;
  type: string;
  text: string;
  x: number;
  y: number;
  width?: number;  // Cached for O(1) selection performance
  height?: number; // Cached for O(1) selection performance
  minWidth?: number;  // Minimum width constraint (set at creation, stable across resizes)
  minHeight?: number; // Minimum height constraint (set at creation, stable across resizes)
  inputs?: NodeIO[];
  outputs?: NodeIO[];
  isTool?: boolean;
  toolName?: string;
  parameters?: any;
  // AI Node specific properties
  aiBehavior?: AiNodeBehavior;
  isAiNode?: boolean;
  aiModel?: string; // Selected AI model ID
  promptTemplate?: string; // Selected prompt template ID for text-only nodes
  executionState?: ExecutionState;
  executionConfig: NodeExecutionConfig;
  invocationMode?: boolean; // Compact output-only view for referenced actions
}

type Connection = FlowCanvasConnection;

interface NodeIO {
  id: string;
  name: string;
  type: string;
  value?: any;
  attached?: boolean;
  enum?: string[];
  required?: boolean; // Whether this input is required (defaults to true for backwards compatibility)
  forceInput?: boolean; // If true, this input MUST be a connection (no widget shown)
  isDynamicType?: boolean; // If true, this output's type adapts at runtime (allows "any" to connect to specific types)
  displayOptions?: {
    show?: Record<string, any[]>; // Show this field when other fields match these values
    hide?: Record<string, any[]>; // Hide this field when other fields match these values
  };
  // Widget metadata for UI controls
  widget?: string; // 'slider', 'color', 'toggle', 'file', 'textarea', 'select', 'multiselect', 'text', 'number'
  widgetOptions?: {
    min?: number;
    max?: number;
    step?: number;
    accept?: string[]; // File types for file picker
    multiline?: boolean; // For text areas
    rows?: number; // Number of rows for textarea
    options?: string[]; // Options for multi-select dropdown
  };
}

// Local execution state interface (specific to flow.vue)
interface ExecutionState {
  status: "idle" | "running" | "completed" | "error";
  last_execution?: Date;
  last_inputs?: Record<string, any>; // Actual input values used during execution (after connection resolution)
  last_output?: any; // Generic output type
  error_message?: string;
}

// Define interfaces that match the main component
interface NodeState {
  nodes: Ref<Node[]>;
  connections: Ref<Connection[]>;
  nodeTypes: Ref<any[]>;
  selectedNodes: Ref<Set<number>>;
  selectedNode: Ref<Node | null>;
  hoveredFlowPort: Ref<any>;
  isCtrlPressed: Ref<boolean>;
}

interface NodeActions {
  forceConnectionUpdate: () => void;
  findConnectionsByNode: (nodeId: number) => Connection[];
  findNodeInputById: (node: Node, inputId: string) => NodeIO | undefined;
  findTriggerNodes: () => Array<Pick<Node, "id" | "type" | "text">>;
  showValidationError: (error: string) => void;
}

export const useFlowNodes = (state: NodeState, actions: NodeActions) => {
  // Helper function to get node dimensions based on type
  const getNodeDimensions = (nodeType: any, isTool: boolean = false) => {
    if (isTool) {
      // Tool nodes are typically smaller but use same base
      return { width: 200, height: 140 };
    }
    
    // Determine size based on node category/type using actual CSS values
    const category = nodeType.category || '';
    const nodeTypeName = nodeType.name || '';
    
    if (category === 'TRIGGER' || nodeTypeName.includes('trigger')) {
      // Trigger nodes use min-width: 320px from CSS
      return { width: 320, height: 180 };
    } else if (category === 'AI' || nodeTypeName.includes('ai')) {
      // AI nodes are medium sized, use default min-width: 200px
      return { width: 200, height: 160 };
    } else if (category === 'AUDIO' || nodeTypeName.includes('audio')) {
      // Audio nodes are medium sized
      return { width: 200, height: 150 };
    } else {
      // Default control nodes use min-width: 200px from CSS
      return { width: 200, height: 140 };
    }
  };

  const createNodeAtPosition = (
    nodeType: any,
    x: number,
    y: number,
    isTool: boolean = false,
    width?: number,
    height?: number
  ): Node => {
    // Create the node at the exact position (will be corrected after DOM rendering)
    const newNode: Node = {
      id: Date.now() + Math.floor(Math.random() * 1000),
      type: nodeType.name || nodeType.toolName || nodeType.type,
      text: nodeType.label || nodeType.name || nodeType.toolName,
      // Position as-is, will be centered after DOM measurement
      x: x,
      y: y,
      // Don't set width/height - let CSS control default sizing
      // Only set if explicitly provided (e.g., when loading from saved flow)
      width: width,
      height: height,
      // Don't set minWidth/minHeight here - they'll be captured from actual DOM size on first resize
      // This ensures min constraints match the real spawning size, not hardcoded values
      inputs: isTool
        ? convertToolInputsToNodeInputs(nodeType)
        : nodeType.inputs
        ? nodeType.inputs.map((input: NodeIO) => ({
            ...input,
            value: input.value !== undefined ?
              (typeof input.value === 'object' ? JSON.parse(JSON.stringify(input.value)) : input.value) :
              getDefaultValueForProperty(input.type, input.enum)
          }))
        : [],
      outputs: isTool
        ? createToolOutputs(nodeType)
        : nodeType.outputs
        ? nodeType.outputs.map((output: NodeIO) => ({ ...output }))
        : [],
      isTool,
      toolName: isTool ? nodeType.name || nodeType.toolName : undefined,
      isAiNode: nodeType.isAiNode || nodeType.aiBehavior,
      aiBehavior: nodeType.aiBehavior,
      aiModel: nodeType.isAiNode ? getDefaultAiModel() : undefined,
      executionState: {
        status: "idle",
        last_execution: undefined,
        last_output: undefined,
        error_message: undefined,
      },
      executionConfig: {
        timeoutMs: undefined,
        retryCount: 0,
        retryDelayMs: 1000,
        skipOnError: false,
        parallelExecution: false,
      },
      parameters: {
        ...(nodeType.parameters || {}),
        backendId: nodeType.backendId,
        displayName: nodeType.displayName,
        isBackendNode: nodeType.isBackendNode,
      },
    };

    return newNode;
  };

  // Get default value for property type
  const getDefaultValueForProperty = (type: string, enumOptions?: string[]): any => {
    return getDefaultValueForSchemaType(type, enumOptions);
  };

  // Helper function to convert tool inputs to node inputs
  const convertToolInputsToNodeInputs = (tool: any): NodeIO[] => {
    return convertJsonSchemaToNodeInputs(tool.parameters, {
      defaultValueResolver: getDefaultValueForProperty,
    }) as NodeIO[];
  };

  // Helper function to create tool outputs
  const createToolOutputs = (tool: any): NodeIO[] => {
    return [{ id: "value", name: "Value", type: "string" }];
  };

  // Helper function to center node on exact position using actual DOM dimensions
  const centerNodeOnPosition = (node: Node, targetX: number, targetY: number) => {
    try {
      const nodeElement = document.querySelector(`[data-node-id="${node.id}"]`) as HTMLElement;
      if (nodeElement) {
        const rect = nodeElement.getBoundingClientRect();

        // Try to get camera scale from the node's transform style
        let scale = 1;
        const transformStyle = nodeElement.style.transform;
        if (transformStyle && transformStyle.includes('scale(')) {
          const scaleMatch = transformStyle.match(/scale\(([^)]+)\)/);
          if (scaleMatch) {
            scale = parseFloat(scaleMatch[1] ?? "1") || 1;
          }
        }

        // Get actual dimensions in world coordinates
        const actualWidth = rect.width / scale;
        const actualHeight = rect.height / scale;

        // Calculate perfect center position
        const centeredX = targetX - actualWidth / 2;
        const centeredY = targetY - actualHeight / 2;

        // Update the node position
        node.x = centeredX;
        node.y = centeredY;

        // Force update to refresh visual positions
        actions.forceConnectionUpdate();
      }
    } catch (error) {
      console.warn("Could not center node:", error);
    }
  };

  const addNodeToCanvas = (node: Node): void => {
    state.nodes.value.push(node);
    actions.forceConnectionUpdate();
  };

  const deleteNode = (node: Node) => {
    // Clear any hover states for the node being deleted
    if (state.hoveredFlowPort.value && state.hoveredFlowPort.value.nodeId === node.id) {
      state.hoveredFlowPort.value = null;
    }

    // Remove the node
    state.nodes.value = state.nodes.value.filter((n) => n.id !== node.id);

    // Find all connections to/from this node using utility function
    const nodeConnections = actions.findConnectionsByNode(node.id);

    // For connections to this node, update the attached status of inputs
    nodeConnections.forEach((conn) => {
      if (conn.to === node.id && conn.kind === "data") {
        // Nothing to do here since the node is being deleted
      } else if (
        conn.from === node.id &&
        conn.kind === "data"
      ) {
        // Find the target node and update its input's attached status using utility functions
        const targetNode = findNodeById(conn.to);
        if (targetNode && targetNode.inputs) {
          const inputId = conn.toInput;
          if (inputId) {
            const input = actions.findNodeInputById(targetNode, inputId);
            if (input) {
              input.attached = false;
            }
          }
        }
      }
    });

    // Remove all connections involving this node
    state.connections.value = state.connections.value.filter(
      (conn) => conn.from !== node.id && conn.to !== node.id
    );

    // Force UI update to refresh port states
    actions.forceConnectionUpdate();
  };

  // Add a method to select a node
  const selectNode = (node: Node, event?: MouseEvent) => {
    if (state.isCtrlPressed.value && event) {
      // Toggle selection with Ctrl (works in both modes)
      if (state.selectedNodes.value.has(node.id)) {
        state.selectedNodes.value.delete(node.id);
        // If we removed the primary selected node, pick a new one or clear
        if (state.selectedNode.value && state.selectedNode.value.id === node.id) {
          if (state.selectedNodes.value.size > 0) {
            const remainingIds = Array.from(state.selectedNodes.value);
            const nextId = remainingIds[0];
            const nextSelectedNode = nextId === undefined ? undefined : findNodeById(nextId);
            state.selectedNode.value = nextSelectedNode || null;
          } else {
            state.selectedNode.value = null;
          }
        }
      } else {
        state.selectedNodes.value.add(node.id);
        state.selectedNode.value = node; // Make this the primary selection
      }
    } else {
      // Normal single selection - clear previous and select this one
      state.selectedNodes.value.clear();
      state.selectedNodes.value.add(node.id);
      state.selectedNode.value = node;
    }
  };

  // Centralized function to clear all selections
  const clearSelection = () => {
    state.selectedNodes.value.clear();
    state.selectedNode.value = null;
  };

  const deleteSelection = () => {
    if (state.selectedNodes.value.size > 0) {
      // Delete multiple selected nodes
      const nodesToDelete = state.nodes.value.filter((node) =>
        state.selectedNodes.value.has(node.id)
      );
      nodesToDelete.forEach((node) => deleteNode(node));
      clearSelection();
    } else if (state.selectedNode.value) {
      // Delete single selected node
      deleteNode(state.selectedNode.value);
      clearSelection();
    }
  };

  // Add function to clear all nodes and connections
  const clearAll = () => {
    state.nodes.value = [];
    state.connections.value = [];
    clearSelection();
  };

  // Add node by clicking (alternative to dragging)
  const addNodeOfType = (nodeTypeName: string) => {
    // Use validation composable to check if we can add this trigger node
    const triggerNodes = actions.findTriggerNodes();
    const isAddingTrigger = state.nodeTypes.value.some(
      (type) => type.name === nodeTypeName && type.category === "TRIGGER"
    );
    
    if (isAddingTrigger && triggerNodes.length > 0) {
      const errorMessage = `Cannot add trigger node "${nodeTypeName}": Flow already has a trigger node "${triggerNodes[0]?.text || "Trigger"}". Only one trigger node is allowed per flow.`;
      actions.showValidationError(errorMessage);
      console.warn(
        `Cannot add trigger node ${nodeTypeName}: A trigger node already exists in the flow`
      );
      return;
    }

    // Find the node type info
    const nodeTypeInfo = state.nodeTypes.value.find((t) => t.name === nodeTypeName);
    if (!nodeTypeInfo) {
      console.error(`Node type ${nodeTypeName} not found`);
      return;
    }

    // Create node at a default position with some randomness
    const newNode = createNodeAtPosition(
      nodeTypeInfo,
      400 + Math.random() * 100,
      200 + Math.random() * 100
    );

    addNodeToCanvas(newNode);
    console.log(`✅ Added node of type: ${nodeTypeName}`);
  };

  // Node finding utility functions
  const findNodeById = (nodeId: number): Node | undefined => {
    return state.nodes.value.find((n) => n.id === nodeId);
  };

  const findNodeInputById = (node: Node, inputId: string) => {
    return node.inputs?.find((input) => input.id === inputId);
  };

  const findNodeOutputById = (node: Node, outputId: string) => {
    return node.outputs?.find((output) => output.id === outputId);
  };

  const findNodeTypeByName = (typeName: string) => {
    return state.nodeTypes.value.find((nt) => nt.name === typeName);
  };

  // Add helper function to check if a node is a trigger node
  const isTriggerNode = (nodeType: string) => {
    return state.nodeTypes.value.some(
      (type) => type.name === nodeType && type.category === "TRIGGER"
    );
  };

  // Add helper function to check if a trigger node already exists
  const hasTriggerNode = () => {
    return state.nodes.value.some((node) => isTriggerNode(node.type));
  };

  // Add helper function to check if we can add a trigger node
  const canAddTriggerNode = (nodeType: string) => {
    return !isTriggerNode(nodeType) || !hasTriggerNode();
  };

  
  // Keep simplified compatibility functions for debugging only
  const isAiNode = (node: Node): boolean => {
    return node.isAiNode === true || node.type.startsWith("ai-");
  };

//   // Determine if this is an AI node and set appropriate behavior
//   const isAiNode = toolNodeInfo.type_name.toUpperCase() === "AI";

  return {
    createNodeAtPosition,
    addNodeToCanvas,
    deleteNode,
    selectNode,
    clearSelection,
    deleteSelection,
    clearAll,
    addNodeOfType,
    findNodeById,
    findNodeInputById,
    findNodeOutputById,
    findNodeTypeByName,
    isAiNode,
    isTriggerNode,
    hasTriggerNode,
    canAddTriggerNode,
    getNodeDimensions,
    centerNodeOnPosition,
    getDefaultValueForProperty,
  };
}
