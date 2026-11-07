import { type Ref, computed } from "vue";
import type { AiNodeBehavior, FlowCanvasConnection } from "../types/flow";
import { getTypeColor } from "../utils/typeColors";
import {
  canonicalizeFlowOutputPort,
  createFlowCanvasConnection,
} from "../utils/portSemantics";

// Local UI types for connections
interface Node {
  id: number;
  type: string;
  text: string;
  x: number;
  y: number;
  minimized?: boolean;
  inputs?: NodeIO[];
  outputs?: NodeIO[];
  isTool?: boolean;
  toolName?: string;
  parameters?: any;
  aiBehavior?: AiNodeBehavior;
  isAiNode?: boolean;
  aiModel?: string;
  promptTemplate?: string;
  executionState?: ExecutionState;
}

type Connection = FlowCanvasConnection;

interface NodeIO {
  id: string;
  name: string;
  type: string;
  value?: any;
  attached?: boolean;
  enum?: string[];
  isDynamicType?: boolean;
}

interface ExecutionState {
  status: "idle" | "running" | "completed" | "error";
  last_execution?: Date;
  last_output?: any;
  error_message?: string;
}

interface ConnectionState {
  nodes: Ref<Node[]>;
  connections: Ref<Connection[]>;
  camera: Ref<{ x: number; y: number; scale: number }>;
}

interface ConnectionActions {
  forceConnectionUpdate: () => void;
  findNodeById: (nodeId: number) => Node | undefined;
  findNodeInputById: (node: Node, inputId: string) => NodeIO | undefined;
  findNodeOutputById: (node: Node, outputId: string) => NodeIO | undefined;
  isTriggerNode: (nodeType: string) => boolean;
}

export const useFlowConnections = (state: ConnectionState, actions: ConnectionActions) => {
  const isValueDrivenNodeType = (nodeType: string): boolean =>
    nodeType === "get-variable" || nodeType === "set-variable";
  const isInputPort = (portType: string): boolean =>
    portType === "flow-input" || portType.startsWith("value-input-");
  const isOutputPort = (portType: string): boolean =>
    portType.startsWith("flow-output") || portType.startsWith("value-output-");
  const valuePortId = (portType: string, prefix: string): string =>
    portType.startsWith(prefix) ? portType.slice(prefix.length) : "";

  // Find connections by node
  const findConnectionsByNode = (nodeId: number) => {
    return state.connections.value.filter(
      (conn) => conn.from === nodeId || conn.to === nodeId
    );
  };

  // Find connections by port
  const findConnectionsByPort = (nodeId: number, portType: string) => {
    return state.connections.value.filter((conn) => {
      if (portType.includes("input")) {
        return conn.to === nodeId && conn.toType === portType;
      } else {
        return conn.from === nodeId && conn.fromType === portType;
      }
    });
  };

  // Check if port is already connected
  const isPortAlreadyConnected = (nodeId: number, portType: string): boolean => {
    // For value inputs, each port can only have one incoming connection
    if (portType.includes("value-input")) {
      const inputId = valuePortId(portType, "value-input-");
      return state.connections.value.some(
        (conn) => conn.kind === "data" && conn.to === nodeId && conn.toInput === inputId
      );
    }

    // Control inputs and outputs may have multiple edges. Multiple incoming
    // control edges form a join; the scheduler waits for all reachable sources.
    return false;
  };

  // Validate node connection
  const validateNodeConnection = (
    sourceNode: Node,
    targetNode: Node,
    sourcePortType: string,
    targetPortType: string
  ): boolean => {
    console.log("Validating connection:", {
      sourceNode,
      targetNode,
      sourcePortType,
      targetPortType,
    });

    // Don't allow connecting a node to itself
    if (sourceNode.id === targetNode.id) {
      console.log("Cannot connect a node to itself");
      return false;
    }

    // Don't allow connecting TO trigger nodes (they don't have inputs)
    if (actions.isTriggerNode(targetNode.type)) {
      console.log("Cannot connect to trigger nodes - they don't have inputs");
      return false;
    }

    // Generic control-flow output is implicit. Only value ports require a
    // declared output schema entry.
    if (
      sourcePortType.startsWith("value-output-") &&
      (!sourceNode.outputs || sourceNode.outputs.length === 0)
    ) {
      console.log("Cannot connect value output from a node without declared outputs");
      return false;
    }

    // Don't allow connecting TO nodes that don't have inputs (except flow connections to non-trigger nodes)
    if (
      targetPortType.includes("value-input") &&
      (!targetNode.inputs || targetNode.inputs.length === 0)
    ) {
      console.log("Cannot connect to nodes without value inputs");
      return false;
    }

    // Prevent invalid connection types
    const isSourceInput = isInputPort(sourcePortType);
    const isSourceOutput = isOutputPort(sourcePortType);
    const isTargetInput = isInputPort(targetPortType);
    const isTargetOutput = isOutputPort(targetPortType);

    // Rule: Can only connect outputs to inputs
    if (isSourceInput && isTargetInput) {
      console.log("Cannot connect input to input");
      return false;
    }

    if (isSourceOutput && isTargetOutput) {
      console.log("Cannot connect output to output");
      return false;
    }

    if (isSourceInput && isTargetOutput) {
      console.log("Cannot connect input to output (wrong direction)");
      return false;
    }

    // Only allow output-to-input connections
    if (!isSourceOutput || !isTargetInput) {
      console.log("Invalid connection direction: must be output to input");
      return false;
    }

    // Validate flow connections (flow-output to flow-input)
    if (sourcePortType.startsWith("flow-output") && targetPortType === "flow-input") {
      if (isValueDrivenNodeType(sourceNode.type) || isValueDrivenNodeType(targetNode.type)) {
        console.log("Cannot create flow connection to/from value-driven nodes");
        return false;
      }
      return true;
    }

    // Validate value connections (value-output to value-input)
    if (sourcePortType.includes("value-output") && targetPortType.includes("value-input")) {
      // Get the actual input ID from the target port type
      const targetInputId = valuePortId(targetPortType, "value-input-");
      const targetInput = targetNode.inputs?.find((input) => input.id === targetInputId);

      if (!targetInput) {
        console.log(`Target input ${targetInputId} not found`);
        return false;
      }

      // Get the actual output ID from the source port type
      const sourceOutputId = valuePortId(sourcePortType, "value-output-");
      const sourceOutput = sourceNode.outputs?.find(
        (output) => output.id === sourceOutputId
      );

      if (!sourceOutput) {
        console.log(`Source output ${sourceOutputId} not found`);
        return false;
      }

      // Check type compatibility
      const sourceType = sourceOutput.type;
      const targetType = targetInput.type;

      // "any" type rules:
      // - Input "any" can accept from ANY output type (wildcard input)
      // - Output "any" can ONLY connect to input "any" (not to specific types)
      // - EXCEPTION: Outputs with isDynamicType=true can connect "any" to specific types
      if (targetType === "any") {
        return true; // Input "any" accepts everything
      }
      if (sourceType === "any") {
        // Exception: Outputs marked as dynamic can connect "any" to specific types
        // (they adopt the actual type at runtime)
        if (sourceOutput.isDynamicType) {
          return true; // Allow dynamic type outputs to connect
        }
        return false; // Other "any" outputs cannot connect to specific types
      }

      // If types match exactly, allow connection
      if (sourceType === targetType) {
        return true;
      }

      // Type conversion rules
      const typeConversions: Record<string, string[]> = {
        number: ["string"],
        string: ["number"],
        boolean: ["string", "number"],
      };

      // Check if source type can be converted to target type
      const allowedConversions = typeConversions[sourceType] || [];
      const isCompatible = allowedConversions.includes(targetType);

      if (!isCompatible) {
        console.log(`Incompatible types: ${sourceType} cannot connect to ${targetType}`);
        return false;
      }

      return true;
    }

    // Don't allow mixing flow and value connections
    if (sourcePortType.includes("flow") && targetPortType.includes("value")) {
      console.log("Cannot connect flow port to value port");
      return false;
    }

    if (sourcePortType.includes("value") && targetPortType.includes("flow")) {
      console.log("Cannot connect value port to flow port");
      return false;
    }

    // If we get here, it's an incompatible connection
    console.log(`Incompatible port types: ${sourcePortType} and ${targetPortType}`);
    return false;
  };

  // Update variable node output types based on connections
  const updateVariableNodeTypes = (
    sourceNode: Node,
    targetNode: Node,
    sourcePortType: string,
    targetPortType: string
  ) => {
    // Case 1: Connecting TO a set-variable node's "value" input
    if (targetNode.type === "set-variable" && targetPortType.includes("value-input-value")) {
      // Find the source output port to get its type
      if (sourceNode.outputs && sourcePortType.includes("value-output")) {
        const outputId = valuePortId(sourcePortType, "value-output-");
        const sourceOutput = sourceNode.outputs.find((o) => o.id === outputId);

        if (sourceOutput && sourceOutput.type && sourceOutput.type !== "any") {
          // Update set-variable's output type to match the source
          if (targetNode.outputs) {
            const valueOutput = targetNode.outputs.find((o) => o.id === "value");
            if (valueOutput) {
              valueOutput.type = sourceOutput.type;
              console.log(`🔄 Updated set-variable output type to: ${sourceOutput.type}`);
            }
          }

          // Update all get-variable nodes with the same variable name
          const variableName = targetNode.inputs?.find((i) => i.id === "variableName")?.value;
          if (variableName) {
            state.nodes.value.forEach((node) => {
              if (node.type === "get-variable") {
                const nodeVarName = node.inputs?.find((i) => i.id === "variableName")?.value;
                if (nodeVarName === variableName && node.outputs) {
                  const getValueOutput = node.outputs.find((o) => o.id === "value");
                  if (getValueOutput) {
                    getValueOutput.type = sourceOutput.type;
                    console.log(`🔄 Updated get-variable output type to: ${sourceOutput.type} for variable: ${variableName}`);
                  }
                }
              }
            });
          }
        }
      }
    }
  };

  // Create connection
  const createConnection = (
    fromNodeId: number,
    toNodeId: number,
    fromType: string,
    toType: string
  ): Connection => {
    return createFlowCanvasConnection(
      Date.now(),
      fromNodeId,
      toNodeId,
      fromType,
      toType,
    );
  };

  // Add connection with validation
  const addConnectionWithValidation = (
    sourceNode: Node,
    targetNode: Node,
    sourcePortType: string,
    targetPortType: string
  ): boolean => {
    // Dragging may begin at either side. Persist every connection in one
    // canonical output -> input direction so validation and execution never
    // depend on which port the user grabbed first.
    let canonicalSourceNode = sourceNode;
    let canonicalTargetNode = targetNode;
    let canonicalSourcePortType = canonicalizeFlowOutputPort(sourcePortType);
    let canonicalTargetPortType = canonicalizeFlowOutputPort(targetPortType);
    if (isInputPort(sourcePortType) && isOutputPort(targetPortType)) {
      canonicalSourceNode = targetNode;
      canonicalTargetNode = sourceNode;
      canonicalSourcePortType = targetPortType;
      canonicalTargetPortType = sourcePortType;
    }

    // Validate the connection
    if (
      !validateNodeConnection(
        canonicalSourceNode,
        canonicalTargetNode,
        canonicalSourcePortType,
        canonicalTargetPortType
      )
    ) {
      console.log("Connection validation failed");
      return false;
    }

    // Check if the target port is already connected
    if (
      isInputPort(canonicalTargetPortType) &&
      isPortAlreadyConnected(canonicalTargetNode.id, canonicalTargetPortType)
    ) {
      console.log(`Port ${canonicalTargetPortType} already has a connection`);
      return false;
    }

    const newConnection = createConnection(
      canonicalSourceNode.id,
      canonicalTargetNode.id,
      canonicalSourcePortType,
      canonicalTargetPortType
    );

    // Compare semantic identities, not visual handle strings.
    const exists = state.connections.value.some((conn) => {
      if (
        conn.kind !== newConnection.kind
        || conn.from !== newConnection.from
        || conn.to !== newConnection.to
      ) {
        return false;
      }
      if (conn.kind === "control" && newConnection.kind === "control") {
        return conn.route === newConnection.route;
      }
      return conn.kind === "data"
        && newConnection.kind === "data"
        && conn.fromOutput === newConnection.fromOutput
        && conn.toInput === newConnection.toInput;
    });

    if (!exists) {
      state.connections.value.push(newConnection);

      // Mark the input as attached if it's a value input
      if (canonicalTargetPortType.startsWith("value-input-")) {
        const inputId = valuePortId(canonicalTargetPortType, "value-input-");
        if (inputId && canonicalTargetNode.inputs) {
          const input = actions.findNodeInputById(canonicalTargetNode, inputId);
          if (input) {
            input.attached = true;
          }
        }
      }

      // Dynamic type tracking for variable nodes
      updateVariableNodeTypes(
        canonicalSourceNode,
        canonicalTargetNode,
        canonicalSourcePortType,
        canonicalTargetPortType
      );

      console.log("Created connection:", newConnection);
      return true;
    }

    return false;
  };

  // Delete connection
  const deleteConnection = (connectionId: number) => {
    console.log(`🗑️ Attempting to delete connection ${connectionId}`);
    const connectionIndex = state.connections.value.findIndex((conn) => conn.id === connectionId);

    if (connectionIndex !== -1) {
      const connectionToDelete = state.connections.value[connectionIndex];
      if (!connectionToDelete) return;

      // Update the attached status for value input ports
      if (connectionToDelete.kind === "data") {
        const targetNode = actions.findNodeById(connectionToDelete.to);
        if (targetNode && targetNode.inputs) {
          const input = actions.findNodeInputById(targetNode, connectionToDelete.toInput);
          if (input) {
            input.attached = false;
            console.log(`✅ Updated input ${connectionToDelete.toInput} attached status to false`);
          }
        }
      }

      // Remove the connection
      state.connections.value.splice(connectionIndex, 1);
      console.log(`✅ Deleted connection ${connectionId}`);
      actions.forceConnectionUpdate();
    } else {
      console.log(`❌ Connection ${connectionId} not found`);
    }
  };

  // Get node dimensions
  const getNodeDimensions = (nodeId: number) => {
    const node = actions.findNodeById(nodeId);

    // Better default dimensions based on node type
    let width = 200; // Default width
    let height = 140; // Default height
    // Adjust defaults for trigger nodes which have more outputs
    if (node && actions.isTriggerNode(node.type)) {
      width = 320; // Wider for trigger nodes
      height = 180; // Taller for multiple outputs
    }

    try {
      // Try to get actual dimensions from DOM
      const nodeElem = document.querySelector(`[data-node-id="${nodeId}"]`);

      if (nodeElem) {
        const rect = nodeElem.getBoundingClientRect();
        width = rect.width / state.camera.value.scale;
        height = rect.height / state.camera.value.scale;
      }
    } catch (e) {
      console.error("Error getting node dimensions:", e);
    }

    return { width, height };
  };

  // Get node position for connections
  const getNodePosition = (nodeId: number, type: string) => {
    const node = actions.findNodeById(nodeId);
    if (!node) {
      console.warn(`Node ${nodeId} not found`);
      return { x: 0, y: 0, portPosition: "right" };
    }

    // In minimized mode, route all input ports to the single left anchor
    // and all output ports to the single right anchor.
    const isMinimized = Boolean(node.minimized);
    const useGeneralInputAnchor = isMinimized && type?.includes("input");
    const useGeneralOutputAnchor = isMinimized && type?.includes("output");

    // Try to get actual DOM element position first
    try {
      const nodeElement = document.querySelector(`[data-node-id="${nodeId}"]`);
      if (nodeElement) {
        let portElement: Element | null = null;
        let portPosition = "right";

        // Find the specific port element based on type
        if (type === "flow-input" || useGeneralInputAnchor) {
          portElement = nodeElement.querySelector(".flow-input-port");
          portPosition = "left";
        } else if (type?.startsWith("flow-output-")) {
          const outputId = type.slice("flow-output-".length);
          const outputContainer = nodeElement.querySelector(
            `[data-flow-output-id="${outputId}"]`
          );
          if (outputContainer) {
            portElement = outputContainer.querySelector(".flow-output-port");
          } else {
            portElement = nodeElement.querySelector(".flow-output-port");
          }
          portPosition = "right";
        } else if (type === "flow-output" || useGeneralOutputAnchor) {
          portElement = nodeElement.querySelector(".flow-output-port");
          portPosition = "right";
        } else if (type?.startsWith("value-input-") || type?.includes("value-input-")) {
          const inputId = type.startsWith("value-input-")
            ? type.slice("value-input-".length)
            : type.replace(/^.*value-input-/, "");
          const inputContainer =
            nodeElement.querySelector(`[data-input-id="${inputId}"]`) ||
            (CSS.escape ? nodeElement.querySelector(`[data-input-id="${CSS.escape(inputId)}"]`) : null);
          if (inputContainer) {
            portElement = inputContainer.querySelector(".value-input-port");
          } else {
            const allInputPorts = nodeElement.querySelectorAll(".value-input-port");
            const inputIndex = node.inputs?.findIndex((input: any) => input.id === inputId);
            if (inputIndex != null && inputIndex >= 0) {
              portElement = allInputPorts.item(inputIndex);
            } else if (allInputPorts.length === 1) {
              portElement = allInputPorts.item(0);
            }
          }
          portPosition = "left";
        } else if (type?.startsWith("value-output-") || type?.includes("value-output-")) {
          const outputId = type.startsWith("value-output-")
            ? type.slice("value-output-".length)
            : type.replace(/^.*value-output-/, "");
          const outputContainer =
            nodeElement.querySelector(`[data-output-id="${outputId}"]`) ||
            (CSS.escape ? nodeElement.querySelector(`[data-output-id="${CSS.escape(outputId)}"]`) : null);
          if (outputContainer) {
            portElement = outputContainer.querySelector(".value-output-port");
          } else {
            const allOutputPorts = nodeElement.querySelectorAll(".value-output-port");
            const outputIndex = node.outputs?.findIndex((output: any) => output.id === outputId);
            if (outputIndex != null && outputIndex >= 0) {
              portElement = allOutputPorts.item(outputIndex);
            } else if (allOutputPorts.length === 1) {
              portElement = allOutputPorts.item(0);
            }
          }
          portPosition = "right";
        }

        // If we found the port element, use its actual position
        if (portElement) {
          // For fixed positioned elements, we can use getBoundingClientRect directly
          const portRect = portElement.getBoundingClientRect();

          // Calculate the center point of the port (already in screen coordinates)
          const portCenterX = portRect.left + portRect.width / 2;
          const portCenterY = portRect.top + portRect.height / 2;

          // Return screen coordinates directly since everything is fixed positioned now
          return {
            x: portCenterX,
            y: portCenterY,
            portPosition,
          };
        }
      }
    } catch (error) {
      console.warn(
        "Error getting DOM position, falling back to calculated position:",
        error
      );
    }

    // Fallback to mathematical calculation if DOM method fails
    const nodeDimensions = getNodeDimensions(nodeId);
    const nodeWidth = nodeDimensions.width;
    const nodeHeight = nodeDimensions.height;
    const headerHeight = 40; // Node header height
    const padding = 12; // Internal padding

    // Calculate base position from node coordinates with manual transform applied
    const baseX = node.x * state.camera.value.scale + state.camera.value.x;
    const baseY = node.y * state.camera.value.scale + state.camera.value.y;

    // Calculate port positions mathematically as fallback (in screen coordinates)
    let portX = baseX;
    let portY = baseY;
    let portPosition = "right";

    if (useGeneralInputAnchor) {
      return {
        x: baseX,
        y: baseY + (nodeHeight * state.camera.value.scale) / 2,
        portPosition: "left",
      };
    }

    if (useGeneralOutputAnchor) {
      return {
        x: baseX + nodeWidth * state.camera.value.scale,
        y: baseY + (nodeHeight * state.camera.value.scale) / 2,
        portPosition: "right",
      };
    }

    const scale = state.camera.value.scale;

    if (type === "flow-input") {
      portX = baseX;
      portY = baseY + (headerHeight / 2) * scale;
      portPosition = "left";
    } else if (type?.startsWith("flow-output")) {
      portX = baseX + nodeWidth * scale;
      portY = baseY + (headerHeight / 2) * scale;
      portPosition = "right";
    } else if (type?.includes("value-input-")) {
      const inputId = type.startsWith("value-input-")
        ? type.slice("value-input-".length)
        : type.replace(/^.*value-input-/, "");
      const inputIndex = Math.max(0, node.inputs?.findIndex((input: any) => input.id === inputId) ?? 0);

      portX = baseX;
      portY = baseY + (headerHeight + padding + 16 + inputIndex * 36) * scale;
      portPosition = "left";
    } else if (type?.includes("value-output-")) {
      const outputId = type.startsWith("value-output-")
        ? type.slice("value-output-".length)
        : type.replace(/^.*value-output-/, "");
      const outputIndex = Math.max(0, node.outputs?.findIndex((output: any) => output.id === outputId) ?? 0);

      portX = baseX + nodeWidth * scale;
      portY = baseY + (headerHeight + padding + 16 + outputIndex * 36) * scale;
      portPosition = "right";
    }

    return {
      x: portX,
      y: portY,
      portPosition,
    };
  };

  // Create smooth bezier path for connections
  const createSmoothBezierPath = (start: any, end: any) => {
    // Calculate dynamic offset based on distance and port positions
    const dx = Math.abs(end.x - start.x);
    const dy = Math.abs(end.y - start.y);
    const distance = Math.sqrt(dx * dx + dy * dy);

    // Scale all offsets with camera zoom to maintain consistent curve appearance
    const scale = state.camera.value.scale;

    // Much larger base offset with higher minimum and maximum bounds (scaled)
    const minOffset = 120 * scale;
    const maxOffset = 300 * scale;
    const baseOffset = Math.min(Math.max(distance * 0.5, minOffset), maxOffset);

    // Much larger additional offset for maximum port visibility (scaled)
    const portOffset = 80 * scale;

    let cp1x, cp1y, cp2x, cp2y;

    // Determine port orientations
    const startPortPosition = start.portPosition || "right";
    const endPortPosition = end.portPosition || "left";

    // Enhanced control point calculations based on port positions
    if (startPortPosition === "right" || startPortPosition === "left") {
      // Horizontal flow from start port
      const startDirection = startPortPosition === "right" ? 1 : -1;
      cp1x = start.x + (baseOffset + portOffset) * startDirection;
      cp1y = start.y;

      if (endPortPosition === "left" || endPortPosition === "right") {
        // Horizontal to horizontal
        const endDirection = endPortPosition === "left" ? -1 : 1;
        cp2x = end.x + (baseOffset + portOffset) * endDirection;
        cp2y = end.y;
      } else {
        // Horizontal to vertical
        const endDirection = endPortPosition === "top" ? -1 : 1;
        cp2x = end.x;
        cp2y = end.y + (baseOffset + portOffset) * endDirection;
      }
    } else {
      // Vertical flow from start port
      const startDirection = startPortPosition === "top" ? -1 : 1;
      cp1x = start.x;
      cp1y = start.y + (baseOffset + portOffset) * startDirection;

      if (endPortPosition === "left" || endPortPosition === "right") {
        // Vertical to horizontal
        const endDirection = endPortPosition === "left" ? -1 : 1;
        cp2x = end.x + (baseOffset + portOffset) * endDirection;
        cp2y = end.y;
      } else {
        // Vertical to vertical
        const endDirection = endPortPosition === "top" ? -1 : 1;
        cp2x = end.x;
        cp2y = end.y + (baseOffset + portOffset) * endDirection;
      }
    }

    // Create extremely pronounced curve with very strong control points
    return `M ${start.x} ${start.y} C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${end.x} ${end.y}`;
  };

  // Get connection color based on type
  const getConnectionColor = (conn: Connection) => {
    if (conn.kind === "control") {
      return "url(#flowGradient)";
    }

    // Determine connection type from source output or target input
    let connectionType = "string"; // default

    if (conn.kind === "data") {
      const sourceNode = actions.findNodeById(conn.from);
      const output = sourceNode
        ? actions.findNodeOutputById(sourceNode, conn.fromOutput)
        : null;
      if (output) {
        connectionType = output.type;
      }
    }

    // Use type color utility for consistent coloring
    return getTypeColor(connectionType);
  };

  // Get connection stroke width - fixed widths, no scaling
  const getConnectionStrokeWidth = (conn: Connection) => {
    if (conn.kind === "control") {
      return 3; // Slightly thicker for better visibility
    }
    return 2.5; // Slightly thicker for data connections
  };

  // Get animation duration
  const getAnimationDuration = (conn: Connection) => {
    if (conn.kind === "control") {
      return 2; // 2 seconds for flow animations
    }
    return 3; // Slower for data connections
  };

  // Get connection midpoint
  const getConnectionMidpoint = (conn: Connection) => {
    const startPos = getNodePosition(conn.from, conn.fromType || "");
    const endPos = getNodePosition(conn.to, conn.toType || "");
    return {
      x: (startPos.x + endPos.x) / 2,
      y: (startPos.y + endPos.y) / 2,
    };
  };

  return {
    findConnectionsByNode,
    findConnectionsByPort,
    isPortAlreadyConnected,
    validateNodeConnection,
    addConnectionWithValidation,
    deleteConnection,
    getNodePosition,
    createSmoothBezierPath,
    getConnectionColor,
    getConnectionStrokeWidth,
    getAnimationDuration,
    getConnectionMidpoint,
  };
};
