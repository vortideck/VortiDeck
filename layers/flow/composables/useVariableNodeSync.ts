import { watch, type Ref } from 'vue';
import type { FlowCanvasConnection } from '../types/flow';

interface NodeIO {
  id: string;
  name: string;
  type: string;
  value?: any;
  [key: string]: any;
}

interface Node {
  id: number;
  type: string;
  inputs?: NodeIO[];
  outputs?: NodeIO[];
  [key: string]: any;
}

type Connection = FlowCanvasConnection;

/**
 * Event-driven variable node type synchronization
 *
 * Instead of deep-watching all nodes, this provides targeted update functions
 * that should be called when specific events occur (connection created/removed, variable name changed)
 */
export function useVariableNodeSync(nodes: Ref<Node[]>, connections: Ref<Connection[]>) {
  const inferValueType = (value: any): string => {
    if (value === null || value === undefined) return "any";
    if (Array.isArray(value)) return "array";

    switch (typeof value) {
      case "string":
        return "string";
      case "number":
        return Number.isFinite(value) ? "number" : "any";
      case "boolean":
        return "boolean";
      case "object":
        return "object";
      default:
        return "any";
    }
  };

  // Cache for quick lookups
  const variableNodesCache = new Map<number, Node>();

  // Update cache when nodes change
  const updateCache = () => {
    variableNodesCache.clear();
    nodes.value.forEach(node => {
      if (node.type === 'set-variable' || node.type === 'get-variable') {
        variableNodesCache.set(node.id, node);
      }
    });
  };

  /**
   * Update a specific Set Variable node's output type based on its incoming connection
   */
  const updateSetVariableType = (setVarNode: Node) => {
    const varNameInput = setVarNode.inputs?.find(i => i.id === 'variableName');
    const valueInput = setVarNode.inputs?.find(i => i.id === 'value');
    const valueTypeInput = setVarNode.inputs?.find(i => i.id === 'valueType');
    const valueOutput = setVarNode.outputs?.find(o => o.id === 'value');

    if (!valueOutput) return;

    // Update output name
    if (varNameInput?.value) {
      valueOutput.name = varNameInput.value as string;
    } else {
      valueOutput.name = 'Value';
      valueOutput.type = 'any';
      return;
    }

    // Find incoming connection to this node's value input
    const incomingConnection = connections.value.find(conn =>
      conn.kind === 'data' && conn.to === setVarNode.id && conn.toInput === 'value'
    );

    if (incomingConnection?.kind === 'data') {
      const sourceNode = nodes.value.find(n => n.id === incomingConnection.from);
      if (sourceNode) {
        const sourceOutput = sourceNode.outputs?.find(
          o => o.id === incomingConnection.fromOutput,
        );
        if (sourceOutput) {
          valueOutput.type = sourceOutput.type;
          return;
        }
      }
    }

    // No connection: use explicit valueType when set, otherwise infer from manual value
    const selectedType = typeof valueTypeInput?.value === "string"
      ? valueTypeInput.value.toLowerCase()
      : "any";

    if (
      selectedType === "string" ||
      selectedType === "number" ||
      selectedType === "boolean" ||
      selectedType === "object" ||
      selectedType === "array"
    ) {
      valueOutput.type = selectedType;
      return;
    }

    valueOutput.type = inferValueType(valueInput?.value);
  };

  /**
   * Update a specific Get Variable node's output type from matching Set Variable
   */
  const updateGetVariableType = (getVarNode: Node) => {
    const varNameInput = getVarNode.inputs?.find(i => i.id === 'variableName');
    const valueOutput = getVarNode.outputs?.find(o => o.id === 'value');

    if (!valueOutput) return;

    // Update output name
    if (varNameInput?.value) {
      valueOutput.name = varNameInput.value as string;
    } else {
      valueOutput.name = 'Value';
      valueOutput.type = 'any';
      return;
    }

    // Find matching Set Variable node
    const setVarNode = nodes.value.find(n =>
      n.type === 'set-variable' &&
      n.inputs?.find(i => i.id === 'variableName')?.value === varNameInput.value
    );

    if (setVarNode) {
      const setVarOutput = setVarNode.outputs?.find(o => o.id === 'value');
      valueOutput.type = setVarOutput?.type !== 'any' ? setVarOutput?.type ?? 'any' : 'any';
    } else {
      valueOutput.type = 'any';
    }
  };

  /**
   * Called when a connection is created or removed
   * Only updates the directly affected nodes
   */
  const onConnectionChange = (_sourceNodeId: number, targetNodeId: number) => {
    updateCache(); // Refresh cache

    const targetNode = variableNodesCache.get(targetNodeId);

    // If target is Set Variable, update its type
    if (targetNode?.type === 'set-variable') {
      updateSetVariableType(targetNode);

      // Find all Get Variable nodes with matching variable name
      const varName = targetNode.inputs?.find(i => i.id === 'variableName')?.value;
      if (varName) {
        variableNodesCache.forEach(node => {
          if (node.type === 'get-variable' &&
              node.inputs?.find(i => i.id === 'variableName')?.value === varName) {
            updateGetVariableType(node);
          }
        });
      }
    }
  };

  /**
   * Called when a variable name is changed
   * Updates the node and any related Get/Set Variable nodes
   */
  const onVariableNameChange = (nodeId: number) => {
    updateCache();

    const node = variableNodesCache.get(nodeId);
    if (!node) return;

    if (node.type === 'set-variable') {
      updateSetVariableType(node);

      // Update all Get Variable nodes with this variable name
      const varName = node.inputs?.find(i => i.id === 'variableName')?.value;
      if (varName) {
        variableNodesCache.forEach(n => {
          if (n.type === 'get-variable' &&
              n.inputs?.find(i => i.id === 'variableName')?.value === varName) {
            updateGetVariableType(n);
          }
        });
      }
    } else if (node.type === 'get-variable') {
      updateGetVariableType(node);
    }
  };

  /**
   * Full sync - use sparingly (only on initial load or when adding variable nodes)
   */
  const syncAllVariableNodes = () => {
    updateCache();

    variableNodesCache.forEach(node => {
      if (node.type === 'set-variable') {
        updateSetVariableType(node);
      } else if (node.type === 'get-variable') {
        updateGetVariableType(node);
      }
    });
  };

  // Initial sync on mount
  syncAllVariableNodes();

  // Lightweight watch only for when nodes are added/removed (not deep)
  watch(() => nodes.value.length, syncAllVariableNodes);

  return {
    onConnectionChange,
    onVariableNameChange,
    syncAllVariableNodes,
  };
}
