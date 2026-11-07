import { computed, getCurrentScope, onScopeDispose, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  FlowDefinition,
  FlowExecutionState,
  FlowEvent,
} from "../../flow/types/flow";
import { FlowStatus } from "../../flow/types/flow";
import { normalizeFlowConnections } from "../../flow/utils/portSemantics";
import { assertSafeFlowExport } from '../../flow/utils/flowRecipe';
import {
  assertSupportedFlowDocument,
  flowRevision,
  reidentifyImportedFlow,
} from "../../flow/utils/flowSerialization";

// ============================================================================
// ERROR HANDLING TYPES
// ============================================================================

interface FlowError {
  type:
    | "NETWORK"
    | "VALIDATION"
    | "EXECUTION"
    | "PERSISTENCE"
    | "INITIALIZATION";
  code: string;
  message: string;
  details?: any;
  timestamp: string;
  flowId?: string;
}

interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

interface DurableExecutionHistoryEntry {
  id: string;
  runId: string;
  flowId: string;
  flowName: string;
  status: FlowStatus;
  startedAt: string;
  completedAt?: string;
  nodeExecutionDetails: Array<FlowExecutionState["nodeResults"][string]>;
}

export function useFlowManager() {
  // ============================================================================
  // STATE MANAGEMENT
  // ============================================================================

  const currentFlow = ref<FlowDefinition | null>(null);
  const persistedFlowRevision = ref<string | null>(null);
  const activeFlowExecution = ref<FlowExecutionState | null>(null);
  const isExecutorInitialized = ref(false);
  const savedFlows = ref<FlowDefinition[]>([]);
  const isLoading = ref(false);
  const error = ref<FlowError | null>(null);

  const lastValidationResult = ref<ValidationResult | null>(null);

  // Node progress tracking (ComfyUI-style)
  const nodeProgress = ref<Map<string, { progress: number; message: string }>>(
    new Map(),
  );

  let monitorRevision = 0;
  let monitoredRunId: string | null = null;
  let monitorTimer: ReturnType<typeof setTimeout> | undefined;
  let flowEventUnlisten: (() => void) | undefined;
  let disposed = false;

  const executionRunId = (execution: FlowExecutionState | null) =>
    execution?.runId || execution?.flowId || null;

  const isTerminalStatus = (status: FlowStatus) =>
    status === FlowStatus.Completed ||
    status === FlowStatus.CompletedWithErrors ||
    status === FlowStatus.Failed ||
    status === FlowStatus.Cancelled;

  const stopMonitoring = (runId: string) => {
    if (monitoredRunId !== runId) return;
    monitorRevision += 1;
    monitoredRunId = null;
    if (monitorTimer) {
      clearTimeout(monitorTimer);
      monitorTimer = undefined;
    }
  };

  const stateFromArchivedExecution = (
    entry: DurableExecutionHistoryEntry,
    previous: FlowExecutionState,
  ): FlowExecutionState => {
    const archivedResults = Object.fromEntries(
      (entry.nodeExecutionDetails || []).map((result) => [
        result.nodeId,
        result,
      ]),
    );
    const nodeResults =
      Object.keys(archivedResults).length > 0
        ? archivedResults
        : previous.nodeResults;
    const sortedNodeResults = entry.nodeExecutionDetails?.length
      ? entry.nodeExecutionDetails
      : previous.sortedNodeResults;
    const completedNodes = Object.values(nodeResults)
      .filter(
        (result) =>
          result.status === "Completed" || result.status === "Skipped",
      )
      .map((result) => result.nodeId);
    const failedNodes = Object.values(nodeResults)
      .filter((result) => result.status === "Failed")
      .map((result) => result.nodeId);

    return {
      ...previous,
      status: entry.status,
      currentNode: undefined,
      completedNodes,
      failedNodes,
      nodeResults,
      sortedNodeResults,
      startedAt: entry.startedAt,
      completedAt: entry.completedAt,
      pendingNodes: [],
    };
  };

  const reconcileArchivedExecution = async (
    runId: string,
  ): Promise<boolean> => {
    const previous = activeFlowExecution.value;
    if (!previous || executionRunId(previous) !== runId) return false;

    const entries = await invoke<DurableExecutionHistoryEntry[]>(
      "list_flow_execution_history",
      {
        flowId: previous.flowId,
        limit: 100,
      },
    );
    const archived = entries.find((entry) => entry.runId === runId);
    if (!archived || !isTerminalStatus(archived.status)) return false;

    // Recheck after the await so an older monitor cannot overwrite a newer run.
    if (executionRunId(activeFlowExecution.value) !== runId) return false;
    activeFlowExecution.value = stateFromArchivedExecution(archived, previous);
    stopMonitoring(runId);
    clearError();
    return true;
  };

  // ============================================================================
  // ERROR HANDLING UTILITIES
  // ============================================================================

  const createError = (
    type: FlowError["type"],
    code: string,
    message: string,
    details?: any,
    flowId?: string,
  ): FlowError => ({
    type,
    code,
    message,
    details,
    timestamp: new Date().toISOString(),
    flowId,
  });

  const handleError = (errorObj: FlowError) => {
    console.error(
      `❌ [${errorObj.type}:${errorObj.code}] ${errorObj.message}`,
      errorObj.details,
    );
    error.value = errorObj;
  };

  const clearError = () => {
    error.value = null;
  };

  // ============================================================================
  // VALIDATION UTILITIES
  // ============================================================================

  const validateFlow = (flow: FlowDefinition): ValidationResult => {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Required fields
    if (!flow.id) errors.push("Flow must have an ID");
    if (!flow.name || flow.name.trim() === "")
      errors.push("Flow must have a name");
    if (!flow.nodes || flow.nodes.length === 0)
      errors.push("Flow must have at least one node");

    // Start node validation
    if (!flow.startNodeId) {
      errors.push("Flow must have a start node ID");
    } else if (!flow.nodes.find((n) => n.id === flow.startNodeId)) {
      errors.push("Start node ID does not exist in flow nodes");
    }

    // Node validation
    flow.nodes.forEach((node, index) => {
      if (!node.id) errors.push(`Node at index ${index} is missing ID`);
      if (!node.nodeType) errors.push(`Node ${node.id} is missing node type`);
      if (!node.name) warnings.push(`Node ${node.id} is missing name`);
    });

    // Connection validation
    const connectionKeys = new Set<string>();
    flow.connections.forEach((conn, index) => {
      if (!conn.fromNode || !conn.toNode) {
        errors.push(`Connection at index ${index} is missing node references`);
      } else {
        const fromExists = flow.nodes.find((n) => n.id === conn.fromNode);
        const toExists = flow.nodes.find((n) => n.id === conn.toNode);

        if (!fromExists)
          errors.push(
            `Connection references non-existent source node: ${conn.fromNode}`,
          );
        if (!toExists)
          errors.push(
            `Connection references non-existent target node: ${conn.toNode}`,
          );
        if (conn.fromNode === conn.toNode) {
          errors.push(
            `Connection at index ${index} cannot connect a node to itself`,
          );
        }

        if (conn.kind === "control") {
          const key = `control:${conn.fromNode}:${conn.route ?? ""}:${conn.toNode}`;
          if (connectionKeys.has(key))
            errors.push(`Duplicate control connection at index ${index}`);
          connectionKeys.add(key);

          if (
            conn.route &&
            !fromExists?.outputs.some(
              (output) =>
                output.outputType === "flow" && output.id === conn.route,
            )
          ) {
            errors.push(
              `Control route ${conn.route} is not declared by node ${conn.fromNode}`,
            );
          }
        } else if (conn.kind === "data") {
          const key = `data:${conn.fromNode}:${conn.fromOutput}:${conn.toNode}:${conn.toInput}`;
          if (connectionKeys.has(key))
            errors.push(`Duplicate data connection at index ${index}`);
          connectionKeys.add(key);

          if (
            !fromExists?.outputs.some(
              (output) =>
                output.outputType !== "flow" && output.id === conn.fromOutput,
            )
          ) {
            errors.push(
              `Data output ${conn.fromOutput} is not declared by node ${conn.fromNode}`,
            );
          }
          if (toExists && !(conn.toInput in toExists.inputs)) {
            errors.push(
              `Data input ${conn.toInput} is not declared by node ${conn.toNode}`,
            );
          }
        } else {
          errors.push(`Connection at index ${index} has an invalid kind`);
        }
      }
    });

    // Orphaned nodes check (warning only)
    const connectedNodes = new Set([
      flow.startNodeId,
      ...flow.connections.flatMap((c) => [c.fromNode, c.toNode]),
    ]);

    flow.nodes.forEach((node) => {
      if (!connectedNodes.has(node.id)) {
        warnings.push(
          `Node ${node.name || node.id} is not connected to the flow`,
        );
      }
    });

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
    };
  };

  // ============================================================================
  // COMPUTED PROPERTIES
  // ============================================================================

  const isFlowRunning = computed(() => {
    return (
      activeFlowExecution.value?.status === FlowStatus.Running ||
      activeFlowExecution.value?.status === FlowStatus.Pending
    );
  });

  const canExecuteFlow = computed(() => {
    if (
      !currentFlow.value ||
      !isExecutorInitialized.value ||
      isFlowRunning.value
    ) {
      return false;
    }

    // Validate flow before allowing execution
    const validation = validateFlow(currentFlow.value);
    lastValidationResult.value = validation;

    return validation.isValid;
  });

  const flowProgress = computed(() => {
    if (!activeFlowExecution.value) return 0;

    const total = activeFlowExecution.value.flowDefinition.nodes.length;
    const completed = activeFlowExecution.value.completedNodes.length;

    return total > 0 ? (completed / total) * 100 : 0;
  });

  // ============================================================================
  // FLOW MANAGEMENT
  // ============================================================================

  /**
   * Create a new empty flow
   */
  const createNewFlow = (name?: string): FlowDefinition => {
    const now = new Date().toISOString();

    const newFlow: FlowDefinition = {
      id: `flow-${Date.now()}`,
      name: name || "New Flow",
      version: "1.0.0",
      description: undefined,
      createdAt: now,
      updatedAt: now,
      createdBy: undefined,
      tags: [],
      nodes: [],
      connections: [],
      startNodeId: "",
      isActive: false,
      metadata: {
        canvasSettings: {
          zoom: 1.0,
          panX: 0,
          panY: 0,
          gridEnabled: true,
          snapToGrid: false,
        },
        variables: {},
        permissions: {
          public: false,
          collaborators: [],
          executionRoles: [],
        },
        executionSettings: {
          maxExecutionTimeMs: undefined,
          maxConcurrentNodes: 5,
          autoSaveIntervalMs: 30000,
        },
      },
    };

    currentFlow.value = newFlow;
    persistedFlowRevision.value = null;
    return newFlow;
  };

  /**
   * Update current flow metadata
   */
  const updateFlowMetadata = (updates: Partial<FlowDefinition>) => {
    if (!currentFlow.value) return;

    currentFlow.value = {
      ...currentFlow.value,
      ...updates,
      updatedAt: new Date().toISOString(),
    };
  };

  /**
   * Update flow canvas settings
   */
  const updateCanvasSettings = (zoom: number, panX: number, panY: number) => {
    if (!currentFlow.value) return;

    currentFlow.value.metadata.canvasSettings = {
      ...currentFlow.value.metadata.canvasSettings,
      zoom,
      panX,
      panY,
    };
  };

  // ============================================================================
  // PERSISTENCE
  // ============================================================================

  /**
   * Save current flow to backend storage
   */
  const saveFlow = async (): Promise<void> => {
    if (!currentFlow.value) {
      throw new Error("No flow to save");
    }

    isLoading.value = true;
    clearError();

    try {
      await invoke("save_flow", {
        flow: currentFlow.value,
        expectedRevision: persistedFlowRevision.value,
      });
      persistedFlowRevision.value = await flowRevision(currentFlow.value);
      console.log(`✅ Flow "${currentFlow.value.name}" saved successfully`);
    } catch (err) {
      const errorObj = createError(
        "PERSISTENCE",
        "SAVE_FLOW",
        `Failed to save flow: ${err}`,
        err,
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    } finally {
      isLoading.value = false;
    }
  };

  /**
   * Load a flow from backend storage
   */
  const loadFlow = async (flowId: string): Promise<FlowDefinition> => {
    isLoading.value = true;
    clearError();

    try {
      const flow = (await invoke("load_flow", { flowId })) as FlowDefinition;
      assertSupportedFlowDocument(flow);
      currentFlow.value = flow;
      persistedFlowRevision.value = await flowRevision(flow);
      console.log(`✅ Flow "${flow.name}" loaded successfully`);
      return flow;
    } catch (err) {
      const errorObj = createError(
        "PERSISTENCE",
        "LOAD_FLOW",
        `Failed to load flow: ${err}`,
        err,
        flowId,
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    } finally {
      isLoading.value = false;
    }
  };

  /**
   * List all saved flows
   */
  const listFlows = async (): Promise<FlowDefinition[]> => {
    isLoading.value = true;
    clearError();

    try {
      const flows = (await invoke("list_flows")) as FlowDefinition[];
      savedFlows.value = flows;
      console.log(`✅ Loaded ${flows.length} flows`);
      return flows;
    } catch (err) {
      const errorObj = createError(
        "PERSISTENCE",
        "LIST_FLOWS",
        `Failed to list flows: ${err}`,
        err,
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    } finally {
      isLoading.value = false;
    }
  };

  /**
   * Delete a flow
   */
  const deleteFlow = async (flowId: string): Promise<void> => {
    isLoading.value = true;
    clearError();

    try {
      await invoke("delete_flow", { flowId });

      // Remove from local list
      savedFlows.value = savedFlows.value.filter((flow) => flow.id !== flowId);

      // Clear current flow if it's the deleted one
      if (currentFlow.value?.id === flowId) {
        currentFlow.value = null;
        persistedFlowRevision.value = null;
      }

      console.log(`✅ Flow deleted successfully`);
    } catch (err) {
      const errorObj = createError(
        "PERSISTENCE",
        "DELETE_FLOW",
        `Failed to delete flow: ${err}`,
        err,
        flowId,
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    } finally {
      isLoading.value = false;
    }
  };

  // ============================================================================
  // EXECUTION
  // ============================================================================

  /**
   * Initialize the backend flow executor
   */
  const initializeExecutor = async (): Promise<void> => {
    if (isExecutorInitialized.value) return;

    isLoading.value = true;
    clearError();

    try {
      await invoke("init_flow_executor");
      isExecutorInitialized.value = true;
      console.log("✅ Flow executor initialized");
    } catch (err) {
      const errorObj = createError(
        "INITIALIZATION",
        "INIT_EXECUTOR",
        `Failed to initialize executor: ${err}`,
        err,
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    } finally {
      isLoading.value = false;
    }
  };

  /**
   * Execute the current flow
   */
  const executeFlow = async (inputs?: Record<string, import('@base/types/actionInputs').InvocationScalar>): Promise<string> => {
    if (!currentFlow.value) {
      const errorObj = createError(
        "VALIDATION",
        "NO_FLOW",
        "No flow to execute",
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    }

    // Validate flow before execution
    const validation = validateFlow(currentFlow.value);
    if (!validation.isValid) {
      const errorObj = createError(
        "VALIDATION",
        "INVALID_FLOW",
        "Flow validation failed",
        {
          errors: validation.errors,
          warnings: validation.warnings,
        },
      );
      handleError(errorObj);
      throw new Error(
        `Flow validation failed: ${validation.errors.join(", ")}`,
      );
    }

    // Flow executor is now initialized eagerly at app startup
    // No need to initialize here due to eager initialization in main.rs

    isLoading.value = true;
    clearError();

    try {
      console.log("🚀 Starting flow execution...");
      console.log(
        "📋 Flow data being sent:",
        JSON.stringify(currentFlow.value, null, 2),
      );
      const executingFlow = currentFlow.value;
      const runId = (await invoke("start_flow_execution", {
        flowDefinition: executingFlow,
        ...(inputs ? { inputs } : {}),
      })) as string;

      console.log(`✅ Flow started with run ID: ${runId}`);

      // Establish run identity before the first poll. Very small flows can
      // finish and be archived before that poll (or before their terminal event
      // is processed), so reconciliation must not depend on a fetched snapshot.
      activeFlowExecution.value = {
        runId,
        flowId: executingFlow.id,
        flowDefinition: executingFlow,
        status: FlowStatus.Pending,
        currentNode: undefined,
        completedNodes: [],
        failedNodes: [],
        nodeResults: {},
        executionLog: [],
        startedAt: new Date().toISOString(),
        pendingNodes: executingFlow.nodes,
        sortedNodeResults: [],
      };

      // Start monitoring
      monitorFlowExecution(runId);

      return runId;
    } catch (err) {
      const errorObj = createError(
        "EXECUTION",
        "EXECUTE_FLOW",
        `Failed to execute flow: ${err}`,
        err,
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    } finally {
      isLoading.value = false;
    }
  };

  /**
   * Monitor flow execution status
   */
  const monitorFlowExecution = (runId: string) => {
    monitorRevision += 1;
    const revision = monitorRevision;
    monitoredRunId = runId;
    if (monitorTimer) clearTimeout(monitorTimer);

    const checkStatus = async () => {
      if (revision !== monitorRevision || monitoredRunId !== runId) return;
      try {
        const status = (await invoke("get_flow_execution_status", {
          flowId: runId,
        })) as FlowExecutionState;
        if (revision !== monitorRevision || monitoredRunId !== runId) return;
        activeFlowExecution.value = status;

        console.log(`📊 Flow status: ${status.status}`);

        // Continue monitoring if still running
        if (
          status.status === FlowStatus.Running ||
          status.status === FlowStatus.Pending
        ) {
          monitorTimer = setTimeout(checkStatus, 1000); // Check every second
        } else {
          stopMonitoring(runId);
          console.log(
            `🏁 Flow execution completed with status: ${status.status}`,
          );

          if (status.status === FlowStatus.Completed) {
            console.log("📤 Final results:", status.nodeResults);
          } else if (status.status === FlowStatus.CompletedWithErrors) {
            console.warn("⚠️ Flow completed with errors:", status.failedNodes);
            console.log("📤 Partial results:", status.nodeResults);
          } else if (status.status === FlowStatus.Failed) {
            console.error("❌ Flow failed:", status.executionLog);
          } else if (status.status === FlowStatus.Cancelled) {
            console.log("🛑 Flow was cancelled");
          }
          // Terminal history is persisted authoritatively by the Rust runtime.
        }
      } catch (err) {
        if (revision !== monitorRevision || monitoredRunId !== runId) return;
        try {
          if (await reconcileArchivedExecution(runId)) {
            console.log(`🏁 Reconciled archived terminal execution: ${runId}`);
            return;
          }
        } catch (historyError) {
          console.error(
            "Failed to reconcile archived flow status:",
            historyError,
          );
        }
        if (revision !== monitorRevision || monitoredRunId !== runId) return;
        console.error("❌ Failed to get flow status:", err);
        handleError(
          createError(
            "NETWORK",
            "STATUS_CHECK",
            `Failed to check flow status: ${err}`,
            err,
            runId,
          ),
        );
      }
    };

    void checkStatus();
  };

  /**
   * Cancel the running flow
   */
  const cancelFlow = async (): Promise<void> => {
    if (!activeFlowExecution.value) {
      console.warn("⚠️ No active flow to cancel");
      return;
    }

    try {
      const runId =
        activeFlowExecution.value.runId || activeFlowExecution.value.flowId;
      await invoke("cancel_flow_execution", { flowId: runId });
      stopMonitoring(runId);
      console.log("🛑 Flow cancelled");
      activeFlowExecution.value = null;
    } catch (err) {
      const runId =
        activeFlowExecution.value?.runId || activeFlowExecution.value?.flowId;
      const errorObj = createError(
        "EXECUTION",
        "CANCEL_FLOW",
        `Failed to cancel flow: ${err}`,
        err,
        runId,
      );
      handleError(errorObj);
      throw new Error(errorObj.message);
    }
  };

  // ============================================================================
  // EVENT HANDLING
  // ============================================================================

  /**
   * Set up event listeners for flow events
   */
  const setupEventListeners = () => {
    void listen("flow_event", (event: any) => {
      const flowEvent = event.payload as FlowEvent;
      console.log("🔔 Flow event received:", flowEvent);

      // Handle different event types
      // Note: Backend serializes with camelCase due to #[serde(rename_all = "camelCase")]
      switch (flowEvent.type) {
        case "flowStarted":
          console.log(`🚀 Flow "${flowEvent.flowName}" started`);
          break;
        case "nodeStarted":
          console.log(`🔄 Node "${flowEvent.nodeName}" started`);
          break;
        case "nodeCompleted":
          console.log(`✅ Node completed:`, flowEvent.result);
          // Set to 100% first to show completion
          nodeProgress.value.set(flowEvent.nodeId, {
            progress: 100,
            message: "Completed",
          });
          // Clear after brief delay so user sees 100%
          setTimeout(() => {
            nodeProgress.value.delete(flowEvent.nodeId);
          }, 500);
          break;
        case "nodeProgress":
          // Update node progress
          console.log(
            `📊 NodeProgress event - updating Map for node ${flowEvent.nodeId}:`,
            {
              progress: flowEvent.progress,
              message: flowEvent.message,
            },
          );
          nodeProgress.value.set(flowEvent.nodeId, {
            progress: flowEvent.progress,
            message: flowEvent.message,
          });
          console.log(`📊 nodeProgress Map size: ${nodeProgress.value.size}`);
          break;
        case "nodeFailed":
          console.error(`❌ Node failed:`, flowEvent.error);
          // Clear progress for failed node
          nodeProgress.value.delete(flowEvent.nodeId);
          break;
        case "flowCompleted":
          console.log(`🎉 Flow completed with results:`, flowEvent.result);
          break;
        case "flowFailed":
          console.error(`💥 Flow failed:`, flowEvent.error);
          break;
        case "flowCancelled":
          console.log(`🛑 Flow cancelled`);
          break;
        case "executionStateChanged": {
          const activeRunId = executionRunId(activeFlowExecution.value);
          if (
            activeRunId === flowEvent.runId ||
            monitoredRunId === flowEvent.runId
          ) {
            activeFlowExecution.value = flowEvent.executionState;
            if (isTerminalStatus(flowEvent.executionState.status)) {
              stopMonitoring(flowEvent.runId);
            }
          }
          break;
        }
      }
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          flowEventUnlisten = unlisten;
        }
      })
      .catch((listenerError) => {
        console.error("Failed to subscribe to flow events:", listenerError);
      });
  };

  // ============================================================================
  // IMPORT/EXPORT
  // ============================================================================

  /**
   * Export flow to JSON file
   */
  const exportFlow = (flow?: FlowDefinition): string => {
    const flowToExport = flow || currentFlow.value;
    if (!flowToExport) {
      throw new Error("No flow to export");
    }

    assertSafeFlowExport(flowToExport);
    return JSON.stringify(flowToExport, null, 2);
  };

  /**
   * Import flow from JSON
   */
  const importFlow = (jsonString: string): FlowDefinition => {
    try {
      const flow = JSON.parse(jsonString) as FlowDefinition;

      // Basic structure validation
      if (
        !flow.id ||
        !flow.name ||
        !Array.isArray(flow.nodes) ||
        !Array.isArray(flow.connections)
      ) {
        const errorObj = createError(
          "VALIDATION",
          "INVALID_FORMAT",
          "Invalid flow format: missing required fields",
        );
        handleError(errorObj);
        throw new Error(errorObj.message);
      }

      assertSupportedFlowDocument(flow);

      const now = new Date().toISOString();
      const imported = reidentifyImportedFlow(
        {
          ...flow,
          connections: normalizeFlowConnections(flow.connections as unknown[]),
        },
        { id: `flow-${Date.now()}`, now },
      );

      // Validate the imported flow
      const validation = validateFlow(imported);
      if (!validation.isValid) {
        const errorObj = createError(
          "VALIDATION",
          "IMPORT_VALIDATION",
          "Imported flow validation failed",
          {
            errors: validation.errors,
            warnings: validation.warnings,
          },
        );
        handleError(errorObj);
        console.warn(
          "⚠️ Flow imported with validation warnings:",
          validation.warnings,
        );
        // Don't throw for warnings, just log them
        if (validation.errors.length > 0) {
          throw new Error(
            `Import validation failed: ${validation.errors.join(", ")}`,
          );
        }
      }

      currentFlow.value = imported;
      persistedFlowRevision.value = null;
      console.log("✅ Flow imported successfully");
      return imported;
    } catch (err) {
      if (err instanceof SyntaxError) {
        const errorObj = createError(
          "VALIDATION",
          "JSON_PARSE",
          "Invalid JSON format",
          err,
        );
        handleError(errorObj);
        throw new Error("Invalid JSON format in imported flow");
      }
      throw err; // Re-throw if it's already our custom error
    }
  };

  // ============================================================================
  // INITIALIZATION
  // ============================================================================

  // Auto-setup event listeners
  setupEventListeners();

  if (getCurrentScope()) {
    onScopeDispose(() => {
      disposed = true;
      monitorRevision += 1;
      monitoredRunId = null;
      if (monitorTimer) clearTimeout(monitorTimer);
      monitorTimer = undefined;
      flowEventUnlisten?.();
      flowEventUnlisten = undefined;
    });
  }

  return {
    // State
    currentFlow,
    activeFlowExecution,
    isExecutorInitialized,
    savedFlows,
    isLoading,
    error,

    // Enhanced state
    lastValidationResult,
    nodeProgress,

    // Computed
    isFlowRunning,
    canExecuteFlow,
    flowProgress,

    // Flow Management
    createNewFlow,
    updateFlowMetadata,
    updateCanvasSettings,

    // Persistence
    saveFlow,
    loadFlow,
    listFlows,
    deleteFlow,

    // Execution
    initializeExecutor,
    executeFlow,
    cancelFlow,

    // Import/Export
    exportFlow,
    importFlow,

    // Enhanced utilities
    validateFlow,
    clearError,
  };
}
