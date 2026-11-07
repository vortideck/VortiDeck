// ============================================================================
// UNIFIED FLOW FORMAT - TYPESCRIPT INTERFACES
// Matches the Rust structures in apps/tauri-windows/src-tauri/src/flow_control/executor.rs
// ============================================================================

export interface FlowDefinition {
  id: string;
  name: string;
  version: string;
  description?: string;
  createdAt: string; // ISO datetime string (DateTime<Utc> in Rust)
  updatedAt: string; // ISO datetime string (DateTime<Utc> in Rust)
  createdBy?: string;
  tags: string[];
  nodes: FlowNodeDefinition[];
  connections: FlowConnection[];
  startNodeId: string;
  metadata: FlowMetadata;
  /** Active flows may receive external trigger events and must be executable. */
  isActive: boolean;
}

export interface FlowNodeDefinition {
  id: string;
  nodeType: string;
  name: string;
  inputs: Record<string, any>;
  outputs: FlowNodeOutput[];
  aiBehavior?: AiNodeBehavior;
  aiModel?: string; // Per-node AI model selection
  promptTemplate?: string; // Per-node prompt template selection
  toolName?: string;
  position: [number, number]; // Matches Rust (f64, f64) tuple
  uiData: NodeUIData; // Frontend-specific data
  executionConfig: NodeExecutionConfig;
}

export interface ControlFlowConnection {
  kind: "control";
  fromNode: string;
  toNode: string;
  route?: string;
}

export interface DataFlowConnection {
  kind: "data";
  fromNode: string;
  fromOutput: string;
  toNode: string;
  toInput: string;
}

/** Persisted/runtime edge contract. It contains no canvas port identifiers. */
export type FlowConnection = ControlFlowConnection | DataFlowConnection;

interface FlowCanvasConnectionBase {
  id: number;
  from: number;
  to: number;
  fromType: string;
  toType: string;
}

export interface ControlFlowCanvasConnection extends FlowCanvasConnectionBase {
  kind: "control";
  route?: string;
}

export interface DataFlowCanvasConnection extends FlowCanvasConnectionBase {
  kind: "data";
  fromOutput: string;
  toInput: string;
}

/** Editor projection of a typed edge, including its current visual handles. */
export type FlowCanvasConnection =
  | ControlFlowCanvasConnection
  | DataFlowCanvasConnection;

export interface NodeUIData {
  color?: string;
  icon?: string;
  collapsed: boolean;
  selected: boolean;
  zIndex: number; // Matches Rust i32
  customProperties: Record<string, any>;
}

export interface NodeExecutionConfig {
  timeoutMs?: number; // Matches Rust Option<u64>
  retryCount: number; // Matches Rust u32
  retryDelayMs: number; // Matches Rust u64
  skipOnError: boolean;
  parallelExecution: boolean;
}

export interface FlowNodeOutput {
  id: string;
  name: string;
  outputType: string;
  description?: string;
}

export interface FlowMetadata {
  canvasSettings: CanvasSettings;
  variables: Record<string, FlowVariable>;
  permissions: FlowPermissions;
  executionSettings: FlowExecutionSettings;
}

export interface CanvasSettings {
  zoom: number; // Matches Rust f64
  panX: number; // Matches Rust f64
  panY: number; // Matches Rust f64
  gridEnabled: boolean;
  snapToGrid: boolean;
}

export interface FlowVariable {
  name: string;
  value: any;
  variableType: string;
  description?: string;
  scope: VariableScope;
}

export enum VariableScope {
  Global = "Global",
  Local = "Local",
  Session = "Session",
}

export interface FlowPermissions {
  public: boolean;
  collaborators: string[];
  executionRoles: string[];
}

export interface FlowExecutionSettings {
  maxExecutionTimeMs?: number; // Matches Rust Option<u64>
  maxConcurrentNodes: number; // Matches Rust u32
  autoSaveIntervalMs: number; // Matches Rust u64
}

// ============================================================================
// AI NODE CONFIGURATION - Missing types added
// ============================================================================

export enum AiNodeBehavior {
  FreeAgent = "FreeAgent",
  Orchestrator = "Orchestrator", 
  StrictOnly = "StrictOnly",
  TextOnly = "TextOnly",
  Custom = "Custom",
}

export enum ToolAccessPolicy {
  AllTools = "AllTools",
  ConnectedOnly = "ConnectedOnly",
  Whitelist = "Whitelist", // Note: Rust uses Whitelist(Vec<String>)
  NoTools = "NoTools",
}

export enum ResponseFormat {
  Natural = "Natural",
  Structured = "Structured",
  ToolCallsOnly = "ToolCallsOnly",
  Custom = "Custom", // Note: Rust uses Custom(String)
}

// Unified AI Node Configuration - matches Rust AiNodeConfig
export interface AiNodeConfig {
  behavior: AiNodeBehavior;
  toolAccess: ToolAccessPolicy;
  responseFormat: ResponseFormat;
  customPrompt?: string;
}

// AI Node Input structure - matches Rust
export interface AiNodeInput {
  userMessage: string;
  previousOutputs: Record<string, any>;
  contextMemory: string[];
  availableTools: ToolSpec[];
  branchingConditions?: string[];
  flowContext: FlowContext;
  config: AiNodeConfig;
}

// AI Node Output structure - from flow.vue
export interface AiNodeOutput {
  actions: ToolAction[];
  outputs: Record<string, any>;
  reasoning?: string;
  flowControl?: FlowControl;
}

// Tool Action structure - from flow.vue
export interface ToolAction {
  tool: string;
  args: Record<string, any>;
  targetNodeId?: string;
}

// Flow Control structure - from flow.vue  
export interface FlowControl {
  branchCondition?: string;
  shouldContinue: boolean;
  nextNodeOverride?: string;
}

export interface ToolSpec {
  name: string;
  description: string;
  parameters: any;
  nodeId: string;
  toolType: ToolType;
}

export enum ToolType {
  SystemTool = "SystemTool",
  McpTool = "McpTool", // Note: Rust uses McpTool { server_name: String }
  FlowNode = "FlowNode",
}

export interface FlowContext {
  currentNodeId: string;
  nextNodeTypes: string[];
  expectedOutputs: string[];
}

// ============================================================================
// RUNTIME EXECUTION TYPES
// ============================================================================

export interface FlowExecutionState {
  runId: string;
  flowId: string;
  flowDefinition: FlowDefinition;
  status: FlowStatus;
  currentNode?: string;
  completedNodes: string[]; // Matches Rust HashSet<String> (converted to array for TS)
  failedNodes: string[]; // Matches Rust HashSet<String> (converted to array for TS)
  nodeResults: Record<string, NodeResult>;
  executionLog: ExecutionLogEntry[];
  startedAt: string; // ISO datetime string (DateTime<Utc> in Rust)
  completedAt?: string; // ISO datetime string (Option<DateTime<Utc>> in Rust)
  recoveryDetectedAt?: string;
  pendingNodes: FlowNodeDefinition[];
  // 🆕 Pre-sorted node results for frontend (in execution order)
  sortedNodeResults: NodeResult[];
}

export enum FlowStatus {
  Pending = "Pending",
  Running = "Running",
  RecoveryRequired = "RecoveryRequired",
  Completed = "Completed",
  CompletedWithErrors = "CompletedWithErrors", // Execution finished but some nodes failed
  Failed = "Failed",
  Cancelled = "Cancelled",
}

export interface NodeResult {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  status: NodeStatus;
  inputs: Record<string, any>;
  outputs: Record<string, any>;
  error?: string;
  executionTimeMs: number; // Matches Rust u64
  retryCount: number; // Matches Rust u32
}

export enum NodeStatus {
  Pending = "Pending",
  Running = "Running",
  Completed = "Completed",
  Failed = "Failed",
  Skipped = "Skipped",
}

export interface ExecutionLogEntry {
  timestamp: string; // ISO datetime string (DateTime<Utc> in Rust)
  level: LogLevel;
  nodeId?: string;
  message: string;
  details?: any; // Matches Rust Option<Value>
}

export enum LogLevel {
  Debug = "Debug",
  Info = "Info",
  Warning = "Warning",
  Error = "Error",
}

// ============================================================================
// FLOW EVENTS - Matches Rust #[serde(rename_all = "camelCase")]
// ============================================================================

// ============================================================================
// EXECUTION HISTORY TYPES
// ============================================================================

export interface ExecutionHistoryEntry {
  id: string;
  runId: string;
  flowId: string;
  flowName: string;
  status: FlowStatus;
  startedAt: string; // ISO datetime string
  completedAt?: string; // ISO datetime string
  durationMs?: number;
  reachableNodeCount: number;
  completedNodes: number;
  failedNodes: number;
  errorMessage?: string;
  finalResults?: Record<string, any>;
  /// Detailed execution data for each node including inputs and outputs
  nodeExecutionDetails: NodeExecutionDetail[];
}

/// Detailed execution information for a single node
export interface NodeExecutionDetail {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  status: NodeStatus;
  inputs: Record<string, any>;
  outputs: Record<string, any>;
  error?: string;
  executionTimeMs: number;
  startedAt: string; // ISO datetime string
  completedAt?: string; // ISO datetime string
  retryCount: number;
}

// ============================================================================
// FLOW EVENTS
// ============================================================================

export type FlowEvent =
  | { type: 'flowStarted'; flowId: string; runId: string; flowName: string; }
  | { type: 'nodeStarted'; flowId: string; runId: string; nodeId: string; nodeName: string; }
  | { type: 'nodeCompleted'; flowId: string; runId: string; nodeId: string; result: NodeResult; }
  | { type: 'nodeProgress'; flowId: string; runId: string; nodeId: string; progress: number; message: string; }
  | { type: 'nodeFailed'; flowId: string; runId: string; nodeId: string; error: string; }
  | { type: 'flowCompleted'; flowId: string; runId: string; result: Record<string, any>; }
  | { type: 'flowFailed'; flowId: string; runId: string; error: string; }
  | { type: 'flowCancelled'; flowId: string; runId: string; }
  | { type: 'flowProgressUpdate'; flowId: string; runId: string; progressPercentage: number; reachableNodes: number; completedNodes: number; currentNode?: string; }
  | { type: 'executionStateChanged'; flowId: string; runId: string; executionState: FlowExecutionState; }
  | { type: 'executionHistoryAdded'; flowId: string; runId: string; historyEntry: ExecutionHistoryEntry; };
