# VortiDeck Complete Architecture

## 🎯 Overview

VortiDeck's architecture provides a comprehensive visual workflow system that combines traditional flow control with advanced AI capabilities. This document outlines the complete system from frontend visual design to backend execution, including per-node AI model selection, tool integration, and both AI-powered and direct tool execution.

## 🏗️ System Architecture

### High-Level Overview
```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend Layer (Vue.js)                 │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │ Visual Editor   │    │ Model Selector  │                │
│  │                 │    │                 │                │
│  │ • Node creation │◄──►│ • Per-node AI   │                │
│  │ • Connections   │    │ • Model config  │                │
│  │ • Flow design  │    │ • UI integration │                │
│  └─────────────────┘    └─────────────────┘                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼ Tauri Commands
┌─────────────────────────────────────────────────────────────┐
│                   Backend Layer (Rust)                     │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │ Flow Executor   │    │   AI Executor   │                │
│  │                 │    │                 │                │
│  │ • Flow control  │◄──►│ • AI behaviors  │                │
│  │ • Node routing  │    │ • Model mgmt    │                │
│  │ • State mgmt    │    │ • Tool routing  │                │
│  │ • Direct tools  │    │ • AI-tool coord │                │
│  └─────────────────┘    └─────────────────┘                │
│           │                       │                        │
│           ▼                       ▼                        │
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │ Flow Registry   │    │ Unified Tool    │                │
│  │                 │    │ Executor        │                │
│  │ • Node types    │    │ • System tools  │                │
│  │ • Tool registry │    │ • MCP tools     │                │
│  │ • Flow nodes    │    │ • Direct exec   │                │
│  └─────────────────┘    └─────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

### Key Architectural Principles

1. **Dual Execution Paths**: 
   - **AI-Powered**: AI nodes coordinate and execute tools intelligently
   - **Direct Execution**: Flow Executor can execute tools directly without AI intervention

2. **Per-Node Model Selection**: Each AI node can use different models (GPT-3.5, GPT-4, etc.)

3. **Unified Tool System**: Both system tools and MCP tools are handled by the same executor

4. **Event-Driven**: Real-time communication between frontend and backend via events

## 🎨 Frontend Architecture

### Visual Flow Editor (`flow.vue`)

#### Core Components
```typescript
// Main flow editor state
const nodes = ref<Node[]>([]);
const connections = ref<Connection[]>([]);
const selectedNodes = ref<Set<number>>(new Set());

// Camera and interaction state
const camera = ref({ x: 0, y: 0, scale: 1 });
const isDragging = ref(false);
const isConnecting = ref(false);
const isSelecting = ref(false);

// AI model selection state
const showAiModelSelector = ref(false);
const selectedAiNode = ref<Node | null>(null);
const availableAiModels = ref([
  { id: "gpt-3.5-turbo", name: "GPT-3.5 Turbo", provider: "OpenAI" },
  { id: "gpt-4", name: "GPT-4", provider: "OpenAI" },
  { id: "gpt-4-turbo", name: "GPT-4 Turbo", provider: "OpenAI" }
]);
```

#### Node Types and Categories
```typescript
// Node type definitions with AI integration
const nodeTypes = [
  // Trigger Nodes (Entry Points)
  {
    name: "webhook-trigger",
    label: "🌐 Webhook Trigger",
    category: "TRIGGER",
    inputs: [],
    outputs: [
      { id: "method", name: "HTTP Method", type: "string" },
      { id: "headers", name: "Headers", type: "object" },
      { id: "body", name: "Body", type: "object" },
      { id: "trigger", name: "Flow", type: "flow" }
    ]
  },
  
  // Control Flow Nodes (Direct Execution)
  {
    name: "delay",
    label: "Delay",
    category: "CONTROL",
    inputs: [{ id: "duration", name: "Duration (ms)", type: "number" }],
    outputs: [{ id: "output", name: "Output", type: "flow" }]
  },
  
  // AI Nodes with Behaviors
  {
    name: "ai-orchestrator",
    label: "🧠 AI Orchestrator",
    category: "AI",
    aiBehavior: AiBehavior.Orchestrator,
    isAiNode: true,
    inputs: [
      { id: "user_message", name: "Goal", type: "string" },
      { id: "previous_data", name: "Previous Data", type: "object" }
    ],
    outputs: [
      { id: "outputs", name: "Outputs", type: "object" },
      { id: "tool_results", name: "Tool Results", type: "object" },
      { id: "branch_condition", name: "Branch Condition", type: "string" },
      { id: "flow", name: "Flow", type: "flow" }
    ]
  }
];
```

#### Per-Node Model Selection
```typescript
// AI Model selection functions
const openAiModelSelector = (node: Node) => {
  selectedAiNode.value = node;
  showAiModelSelector.value = true;
};

const selectAiModel = (modelId: string) => {
  if (selectedAiNode.value) {
    selectedAiNode.value.aiModel = modelId;
  }
};

const saveAiModel = () => {
  console.log(`🤖 AI Model updated for node ${selectedAiNode.value?.id}: ${selectedAiNode.value?.aiModel}`);
  closeAiModelSelector();
};

// Helper functions for model display
const getAiModelShortName = (modelId: string) => {
  switch (modelId) {
    case 'gpt-3.5-turbo': return 'GPT-3.5';
    case 'gpt-4': return 'GPT-4';
    case 'gpt-4-turbo': return 'GPT-4T';
    default: return modelId;
  }
};
```

#### Flow Execution Integration
```typescript
// Backend flow execution
const executeFlowBackend = async () => {
  try {
    console.log("🎯 Using Backend Flow Executor");
    
    // Convert frontend flow to backend format (preserves aiModel property)
    const backendFlow = await invoke('convert_frontend_flow', {
      nodes: nodes.value,
      connections: connections.value
    });
    
    // Start execution (backend respects per-node model selection)
    const flowId = await invoke('start_flow_execution', {
      flowDefinition: backendFlow
    });
    
    activeFlowId.value = flowId;
    monitorFlowStatus(flowId);
  } catch (error) {
    console.error("❌ Backend flow execution failed:", error);
  }
};
```

### Connection Management
```typescript
// Advanced connection validation
const validateNodeConnection = (
  sourceNode: Node,
  targetNode: Node,
  sourcePortType: string,
  targetPortType: string
): boolean => {
  // Prevent self-connections
  if (sourceNode.id === targetNode.id) return false;
  
  // Prevent connecting to trigger nodes
  if (isTriggerNode(targetNode.type)) return false;
  
  // Validate port types (output to input only)
  const isSourceOutput = sourcePortType.includes("output");
  const isTargetInput = targetPortType.includes("input");
  
  if (!isSourceOutput || !isTargetInput) return false;
  
  // Type compatibility checking
  if (sourcePortType.includes("value-output") && targetPortType.includes("value-input")) {
    const sourceOutput = findNodeOutput(sourceNode, sourcePortType);
    const targetInput = findNodeInput(targetNode, targetPortType);
    
    // Check type compatibility (string ↔ number, boolean → string/number)
    return areTypesCompatible(sourceOutput.type, targetInput.type);
  }
  
  return true;
};
```

## 🔧 Backend Architecture

### Flow Executor (Rust)

The Flow Executor is the core component that manages workflow execution. It can execute flows in two modes:

#### 1. Direct Tool Execution (No AI Required)
```rust
impl FlowExecutor {
    // Execute tools directly without AI intervention
    async fn execute_direct_tool_node(&self, node_def: &FlowNodeDefinition) -> Result<HashMap<String, Value>, String> {
        match &node_def.node_type {
            NodeType::SystemTool { tool_name, parameters } => {
                // Direct system tool execution
                let result = self.tool_executor.execute_system_tool(tool_name, parameters).await?;
                Ok(result)
            }
            NodeType::McpTool { tool_name, parameters } => {
                // Direct MCP tool execution
                let result = self.tool_executor.execute_mcp_tool(tool_name, parameters).await?;
                Ok(result)
            }
            NodeType::ControlFlow { flow_type } => {
                // Direct control flow execution (delay, branch, etc.)
                self.execute_control_flow(flow_type, &node_def.inputs).await
            }
            _ => Err("Node type not supported for direct execution".to_string())
        }
    }
}
```

#### 2. AI-Coordinated Execution
```rust
impl FlowExecutor {
    // Execute AI nodes that coordinate tool usage
    async fn execute_ai_node(&self, node_def: &FlowNodeDefinition) -> Result<HashMap<String, Value>, String> {
        let ai_input = self.prepare_ai_input(node_def).await?;
        
        // Use the AI Executor for intelligent tool coordination
        let ai_output = self.ai_executor.execute_ai_node(
            &node_def.ai_behavior.unwrap(),
            &node_def.ai_model,  // Per-node model selection
            ai_input
        ).await?;
        
        // Process AI-generated tool actions
        let tool_results = self.tool_executor.execute_tool_actions(&ai_output.actions).await?;
        
        Ok(self.merge_ai_and_tool_results(ai_output, tool_results))
    }
}
```

### AI Executor with Behavior Management

#### AI Behavior Types
The system supports four distinct AI behaviors:

| Behavior | Frontend Name | Backend Enum | Description | Tool Access | Status |
|----------|---------------|--------------|-------------|-------------|--------|
| **Free Agent** | `ai-free` | `FreeAgent` | Full autonomy with all available tools | All tools | ✅ Implemented |
| **Orchestrator** | `ai-orchestrator` | `Orchestrator` | Structured execution with connected tools | Connected only | ✅ Implemented |
| **Strict Executor** | `ai-strict-executor` | `StrictOnly` | Precise tool execution with validation | Specific tools | ⚠️ Architecture ready |
| **Text Generator** | `ai-text-generator` | `TextOnly` | Pure text generation, no tool access | No tools | ✅ Implemented |

#### Behavior Implementation

##### 1. Free Agent (`FreeAgent`)
```rust
// Backend implementation
AiNodeBehavior::FreeAgent => {
    // Provide access to ALL available tools
    let all_tools = self.tool_executor.get_all_available_tools().await?;
    for tool_spec in &all_tools {
        ai_node.add_tool(tool_adapter);
    }
    
    // Allow full autonomy in tool selection and execution
    ai_node.set_autonomy_level(AutonomyLevel::Full);
}
```

##### 2. Orchestrator (`Orchestrator`)
```rust
// Backend implementation  
AiNodeBehavior::Orchestrator => {
    // Only provide tools connected in the flow
    for tool_spec in &input.available_tools {
        ai_node.add_tool(tool_spec);
    }
    
    // Structured execution with flow awareness
    ai_node.set_flow_context(input.flow_context);
    ai_node.set_autonomy_level(AutonomyLevel::Guided);
}
```

##### 3. Text Generator (`TextOnly`)
```rust
// Backend implementation
AiNodeBehavior::TextOnly => {
    // No tools provided - pure text generation
    ai_node.set_tool_access(ToolAccess::None);
    ai_node.set_response_format(ResponseFormat::TextOnly);
}
```

### Unified Tool Executor

#### Multi-Source Tool Execution
```rust
pub struct UnifiedToolExecutor {
    flow_registry: Arc<Mutex<FlowRegistry>>,
    chat_session: Arc<Mutex<ChatSession>>,
}

impl UnifiedToolExecutor {
    pub async fn execute_tool_actions(&self, actions: &[ToolAction]) -> Result<HashMap<String, Value>, String> {
        let mut results = HashMap::new();
        
        for action in actions {
            println!("🔧 Executing tool: {} with args: {:?}", action.tool_name, action.args);
            
            let result = match self.find_and_execute_tool(&action.tool_name, &action.args).await {
                Ok(value) => {
                    println!("✅ Tool {} completed successfully", action.tool_name);
                    value
                }
                Err(e) => {
                    println!("❌ Tool {} failed: {}", action.tool_name, e);
                    Value::String(format!("Error: {}", e))
                }
            };
            
            results.insert(action.tool_name.clone(), result);
        }
        
        Ok(results)
    }
    
    async fn find_and_execute_tool(&self, tool_name: &str, args: &Value) -> Result<Value, String> {
        // Try system tools first (FlowRegistry)
        if let Ok(result) = self.execute_system_tool(tool_name, args).await {
            return Ok(result);
        }
        
        // Try MCP tools (ChatSession)
        if let Ok(result) = self.execute_mcp_tool(tool_name, args).await {
            return Ok(result);
        }
        
        Err(format!("Tool '{}' not found in any executor", tool_name))
    }
}
```

## 🔄 Flow Execution Pipeline

### 1. Flow Definition Conversion
```rust
// Convert frontend flow to backend format
pub async fn convert_frontend_flow(
    nodes: Vec<FrontendNode>,
    connections: Vec<FrontendConnection>
) -> Result<FlowDefinition, String> {
    let mut flow_nodes = Vec::new();
    
    for frontend_node in nodes {
        let flow_node = FlowNodeDefinition {
            id: frontend_node.id.to_string(),
            node_type: determine_node_type(&frontend_node),
            inputs: convert_inputs(frontend_node.inputs),
            outputs: convert_outputs(frontend_node.outputs),
            ai_behavior: frontend_node.ai_behavior,
            ai_model: frontend_node.ai_model,  // 🆕 Preserve model selection
            position: (frontend_node.x, frontend_node.y),
            metadata: extract_metadata(&frontend_node),
        };
        
        flow_nodes.push(flow_node);
    }
    
    let flow_connections = convert_connections(connections)?;
    
    Ok(FlowDefinition {
        id: generate_flow_id(),
        name: "User Flow".to_string(),
        nodes: flow_nodes,
        connections: flow_connections,
        settings: FlowSettings::default(),
    })
}
```

### 2. Flow Execution Lifecycle
```rust
impl FlowExecutor {
    pub async fn start_flow_execution(&self, flow_def: FlowDefinition) -> Result<String, String> {
        let flow_id = generate_flow_id();
        
        // Create flow execution context
        let flow_execution = FlowExecution {
            id: flow_id.clone(),
            definition: flow_def,
            status: FlowStatus::Running,
            current_nodes: Vec::new(),
            completed_nodes: HashSet::new(),
            node_results: HashMap::new(),
            start_time: std::time::Instant::now(),
            execution_log: Vec::new(),
        };
        
        // Store in active flows
        {
            let mut active_flows = self.active_flows.lock().await;
            active_flows.insert(flow_id.clone(), flow_execution);
        }
        
        // Emit flow started event
        self.emit_flow_event(FlowEvent::FlowStarted {
            flow_id: flow_id.clone(),
            flow_name: "User Flow".to_string(),
        }).await;
        
        // Start execution in background
        let executor = self.clone();
        let execution_flow_id = flow_id.clone();
        tokio::spawn(async move {
            if let Err(e) = executor.execute_flow_async(&execution_flow_id).await {
                println!("❌ Flow execution failed: {}", e);
                executor.emit_flow_event(FlowEvent::FlowFailed {
                    flow_id: execution_flow_id,
                    error: e,
                }).await;
            }
        });
        
        Ok(flow_id)
    }
    
    async fn execute_flow_async(&self, flow_id: &str) -> Result<(), String> {
        // Find trigger nodes (entry points)
        let trigger_nodes = self.find_trigger_nodes(flow_id).await?;
        
        if trigger_nodes.is_empty() {
            return Err("No trigger nodes found in flow".to_string());
        }
        
        // Execute from each trigger node
        for trigger_node in trigger_nodes {
            self.execute_from_node(flow_id, &trigger_node).await?;
        }
        
        // Mark flow as completed
        self.complete_flow_execution(flow_id).await?;
        
        Ok(())
    }
}
```

### 3. Node Execution with Dependencies
```rust
impl FlowExecutor {
    async fn execute_from_node(&self, flow_id: &str, node_def: &FlowNodeDefinition) -> Result<(), String> {
        // Check if node already executed
        if self.is_node_completed(flow_id, &node_def.id).await {
            return Ok(());
        }
        
        // Wait for dependencies
        self.wait_for_dependencies(flow_id, node_def).await?;
        
        // Emit node started event
        self.emit_flow_event(FlowEvent::NodeStarted {
            flow_id: flow_id.to_string(),
            node_id: node_def.id.clone(),
            node_name: node_def.node_type.clone(),
        }).await;
        
        // Execute the node (AI or Direct)
        let outputs = match &node_def.ai_behavior {
            Some(_) => {
                // AI-coordinated execution
                self.execute_ai_node(node_def).await?
            }
            None => {
                // Direct tool execution
                self.execute_direct_tool_node(node_def).await?
            }
        };
        
        // Store results and continue execution
        self.store_node_results(flow_id, &node_def.id, outputs.clone()).await;
        
        // Emit completion event
        self.emit_flow_event(FlowEvent::NodeCompleted {
            flow_id: flow_id.to_string(),
            node_id: node_def.id.clone(),
            result: NodeResult { outputs },
        }).await;
        
        // Execute connected nodes
        let connected_nodes = self.get_connected_output_nodes(flow_id, node_def).await?;
        for connected_node in connected_nodes {
            self.execute_from_node(flow_id, &connected_node).await?;
        }
        
        Ok(())
    }
}
```

## 🎛️ Per-Node Model Selection

### Frontend Model Management

#### Node Interface Extension
```typescript
interface Node {
  id: number;
  type: string;
  text: string;
  x: number;
  y: number;
  
  // AI Node specific properties
  aiBehavior?: AiBehavior;
  isAiNode?: boolean;
  aiModel?: string;           // 🆕 Per-node model selection
  flowContext?: FlowContext;
  executionState?: ExecutionState;
  
  inputs?: NodeIO[];
  outputs?: NodeIO[];
}
```

#### Model Selector Component
```typescript
// AI Model selection functions
const openAiModelSelector = (node: Node) => {
  selectedAiNode.value = node;
  showAiModelSelector.value = true;
};

const selectAiModel = (modelId: string) => {
  if (selectedAiNode.value) {
    selectedAiNode.value.aiModel = modelId;
  }
};

// Available models from app settings
const availableAiModels = ref([
  { id: "gpt-3.5-turbo", name: "GPT-3.5 Turbo", provider: "OpenAI" },
  { id: "gpt-4", name: "GPT-4", provider: "OpenAI" },
  { id: "gpt-4-turbo", name: "GPT-4 Turbo", provider: "OpenAI" }
]);
```

### Backend Model Handling

#### Model Session Management
```rust
pub struct ModelSessionManager {
    sessions: HashMap<String, ChatSession>,
    model_configs: HashMap<String, ModelConfig>,
}

impl ModelSessionManager {
    pub async fn get_or_create_session(&mut self, model_id: &str) -> Result<&mut ChatSession, String> {
        if !self.sessions.contains_key(model_id) {
            let config = self.model_configs.get(model_id)
                .ok_or_else(|| format!("Model config not found: {}", model_id))?;
            
            let session = ChatSession::new(config.clone()).await?;
            self.sessions.insert(model_id.to_string(), session);
        }
        
        Ok(self.sessions.get_mut(model_id).unwrap())
    }
    
    pub async fn execute_with_model(&mut self, model_id: &str, input: AiNodeInput) -> Result<AiNodeOutput, String> {
        let session = self.get_or_create_session(model_id).await?;
        session.execute_ai_node(input).await
    }
}
```

## 🚀 Performance & Scalability

### Optimization Strategies

1. **Connection Path Caching**: Smart caching of SVG paths for smooth visual updates
2. **Throttled Updates**: Performance optimization for expensive operations
3. **Smart DOM Management**: Cached element references and efficient updates
4. **Async Execution**: Non-blocking flow execution with real-time feedback

### Model Management Strategies

1. **Dynamic Model Switching**: Per-node model selection without flow restart
2. **Session Pooling**: Reuse model sessions across multiple executions
3. **Model-Specific Optimization**: Different strategies for different model types

## 🔮 Future Enhancements

### Planned Features

1. **StrictOnly Behavior**: Complete implementation of precise tool execution
2. **Visual Debugging**: Step-through debugging with breakpoints
3. **Flow Templates**: Pre-built workflow templates for common use cases
4. **Advanced Branching**: Complex conditional logic and parallel execution
5. **Model Performance Analytics**: Track and optimize model usage patterns

### Architecture Improvements

1. **Distributed Execution**: Support for multi-machine flow execution
2. **Plugin Architecture**: Third-party node and tool development
3. **Advanced Caching**: Intelligent result caching and memoization
4. **Real-time Collaboration**: Multi-user flow editing and execution

---

This architecture provides a robust foundation for both simple automation tasks and complex AI-powered workflows, with the flexibility to execute tools directly or coordinate them through AI, all while maintaining high performance and user experience. 