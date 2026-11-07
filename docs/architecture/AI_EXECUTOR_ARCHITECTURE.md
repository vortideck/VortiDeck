# VortiDeck AI Executor Architecture

## 🎯 Overview

The VortiDeck AI Executor provides a unified, modern architecture for executing AI-powered nodes in visual flows. It replaces the legacy hybrid LLM system with a cleaner, more maintainable approach that supports per-node model selection and advanced tool management.

## 🏗️ Architecture Components

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    VortiDeck Flow Engine                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │  Flow Executor  │    │   AI Executor   │                │
│  │                 │    │                 │                │
│  │ • Node routing  │◄──►│ • AI node exec  │                │
│  │ • Flow control  │    │ • Model mgmt    │                │
│  │ • State mgmt    │    │ • Tool routing  │                │
│  └─────────────────┘    └─────────────────┘                │
│           │                       │                        │
│           ▼                       ▼                        │
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │ System Tools    │    │   MCP Tools     │                │
│  │                 │    │                 │                │
│  │ • File ops      │    │ • External APIs │                │
│  │ • System ctrl   │    │ • Integrations  │                │
│  │ • Audio/Video   │    │ • Custom tools  │                │
│  └─────────────────┘    └─────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

### AI Node Behaviors

The system supports four distinct AI node behaviors:

| Behavior | Description | Use Case | Tool Access | Status |
|----------|-------------|----------|-------------|--------|
| **FreeAgent** | Full autonomy with all tools | Research, exploration | All available tools | ✅ Implemented |
| **Orchestrator** | Structured execution with connected tools | Most workflows | Connected tools only | ✅ Implemented |
| **StrictOnly** | Precise tool execution with validation | Data validation, APIs | Specific tools only | ⚠️ Architecture ready |
| **TextOnly** | Pure text generation, no tools | Content creation, analysis | No tools | ✅ Implemented |

## 🎨 Per-Node Model Selection

### Model Architecture
Each AI node can specify its own AI model, enabling:
- **Specialized Models**: Use GPT-4 for complex reasoning, GPT-3.5 for simple tasks
- **Cost Optimization**: Route expensive operations to appropriate models
- **Performance**: Use faster models for real-time operations
- **Experimentation**: A/B test different models in same flow

### Implementation
```rust
// Frontend: Each node stores selected model
interface Node {
  aiModel?: string;  // e.g., "gpt-4", "gpt-3.5-turbo"
  // ... other properties
}

// Backend: AI Executor respects node model selection
pub async fn execute_ai_node(
    &self,
    config: AiNodeConfig,
    input: AiNodeInput,
    node_model: Option<String>,  // Per-node model override
) -> Result<AiNodeOutput, String>
```

### Model Management Strategies
1. **Dynamic Model Switching**: Switch model in existing session
2. **Model-Specific Sessions**: Maintain separate sessions per model
3. **Session Pool**: Reuse sessions for nodes with same model

## 🔄 Execution Flow

### 1. Flow Initialization
```rust
// Initialize the flow executor with AI capabilities
let flow_executor = FlowExecutor::new(
    flow_registry,
    chat_state,
    app_handle
);

// AI executor is created internally
let ai_executor = AiExecutor::new(flow_registry.clone(), chat_state.clone());
```

### 2. AI Node Execution
```rust
// When an AI node is encountered in FlowExecutor
async fn execute_ai_node(&self, flow_id: &str, node_def: &FlowNodeDefinition) -> Result<HashMap<String, Value>, String> {
    // 1. Build AI configuration from node definition
    let config = AiNodeConfig {
        behavior: node_def.ai_behavior.unwrap_or(AiNodeBehavior::Orchestrator),
        tool_access: ToolAccessPolicy::ConnectedOnly,
        response_format: ResponseFormat::Structured,
        custom_prompt: None,
    };
    
    // 2. Gather connected tools and previous outputs
    let connected_tools = self.get_connected_tools(flow_id, node_def).await?;
    let previous_outputs = self.get_previous_outputs(flow_id, node_def).await?;
    
    // 3. Build AI input
    let ai_input = AiNodeInput {
        user_message: node_def.inputs.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Execute the connected tools")
            .to_string(),
        previous_outputs,
        context_memory: Vec::new(),
        available_tools: connected_tools,
        branching_conditions: None,
        flow_context: FlowContext {
            current_node_id: node_def.id.clone(),
            next_node_types: vec!["tool".to_string()],
            expected_outputs: vec!["result".to_string()],
        },
        config: config.clone(),
    };
    
    // 4. Execute via AI executor with model selection
    let output = self.ai_executor.execute_ai_node(config, ai_input).await?;
    
    // 5. Return outputs for flow continuation
    Ok(output.outputs)
}
```

### 3. Tool Execution via UnifiedToolExecutor
```rust
// Tools are executed automatically by AI nodes
impl UnifiedToolExecutor {
    pub async fn execute_tool_actions(&self, actions: &[ToolAction]) -> Result<HashMap<String, Value>, String> {
        let mut results = HashMap::new();
        
        for action in actions {
            let result = match self.find_and_execute_tool(&action.tool_name, &action.args).await {
                Ok(value) => value,
                Err(e) => Value::String(format!("Error: {}", e)),
            };
            results.insert(action.tool_name.clone(), result);
        }
        
        Ok(results)
    }
}
```

## 🛠️ Tool Management

### System Tools
Built-in tools for common operations:
- **Audio Control**: `system.audio.volume`, `system.audio.play`
- **System Control**: Volume, brightness, power management
- **File Operations**: Read, write, move, delete files
- **Network**: HTTP requests, webhooks
- **Data Processing**: JSON manipulation, text processing

### MCP Tools
External tools via Model Context Protocol:
- **APIs**: REST/GraphQL integrations
- **Databases**: SQL queries, data retrieval
- **Cloud Services**: AWS, Azure, GCP operations
- **Custom Integrations**: User-defined tools

### Tool Registration and Discovery
```rust
// System tools are registered at startup in FlowRegistry
impl FlowRegistry {
    pub fn register_system_tools(&mut self) {
        self.register_node("system.audio.volume", VolumeControlNode::new());
        self.register_node("system.audio.play", AudioPlayNode::new());
        // ... more system tools
    }
}

// MCP tools are discovered dynamically via ChatSession
impl ChatSession {
    pub async fn initialize_mcp_tools(&mut self) -> Result<(), String> {
        let servers = self.config.get_mcp_servers();
        for server in servers {
            let tools = self.discover_server_tools(&server).await?;
            for tool in tools {
                self.tool_set.add_tool(tool);
            }
        }
        Ok(())
    }
}
```

## 🎮 Frontend Integration

### Node Creation with Model Selection
```typescript
// AI nodes are created with specific configurations and model selection
const createAiNode = (behavior: AiBehavior, selectedModel?: string) => {
  return {
    id: generateId(),
    type: `ai-${behavior.toLowerCase()}`,
    aiBehavior: behavior,
    aiModel: selectedModel || 'gpt-3.5-turbo', // Default model
    isAiNode: true,
    inputs: getDefaultInputs(behavior),
    outputs: getDefaultOutputs(behavior)
  };
};

// Model selector component shows current model and allows changes
const AiModelSelector = {
  // Shows: "GPT-4 ⚙️" or "Select Model ⚙️"
  // Click opens modal with available models from app settings
};
```

### Flow Execution with Model Awareness
```typescript
// Execute flow via backend with model information preserved
const executeFlow = async () => {
  try {
    // Convert frontend flow to backend format (preserves aiModel property)
    const backendFlow = await invoke('convert_frontend_flow', {
      nodes: nodes.value,
      connections: connections.value
    });
    
    // Start execution (backend respects per-node model selection)
    const flowId = await invoke('start_flow_execution', {
      flowDefinition: backendFlow
    });
    
    // Monitor progress with model-aware events
    monitorFlowExecution(flowId);
  } catch (error) {
    console.error('Flow execution failed:', error);
  }
};
```

## 🔍 Debugging & Monitoring

### Flow Events
The system emits detailed events for monitoring:
```rust
pub enum FlowEvent {
    FlowStarted { flow_id: String, flow_name: String },
    NodeStarted { flow_id: String, node_id: String, node_name: String },
    NodeCompleted { flow_id: String, node_id: String, result: NodeResult },
    NodeFailed { flow_id: String, node_id: String, error: String },
    FlowCompleted { flow_id: String, result: HashMap<String, Value> },
    FlowFailed { flow_id: String, error: String },
    FlowCancelled { flow_id: String },
}
```

### AI Execution Logging
```rust
// Detailed logging for AI node execution
println!("🤖 AI Executor | Behavior: {:?} | Goal: {}", config.behavior, input.user_message);
println!("🧠 AI Planning completed | Generated {} actions", result.actions.len());
println!("🔧 Tool execution phase completed");
println!("✅ AI Executor completed | Total time: {:?}", start_time.elapsed());
```

## 🔧 Configuration Management

### Model Configuration
Models are configured in `config.json`:
```json
{
  "models": {
    "model": [
      {
        "name": "gpt-4",
        "url": "https://api.openai.com/v1/chat/completions",
        "apiKey": "sk-..."
      },
      {
        "name": "gpt-3.5-turbo", 
        "url": "https://api.openai.com/v1/chat/completions",
        "apiKey": "sk-..."
      }
    ]
  }
}
```

### Tool Access Policies
```rust
// Configure tool access per AI behavior
match config.tool_access {
    ToolAccessPolicy::AllTools => {
        // Free Agent: All system + MCP tools
        let all_tools = self.tool_executor.get_all_available_tools().await?;
        for tool_spec in &all_tools {
            ai_node.add_tool(tool_adapter);
        }
    }
    ToolAccessPolicy::ConnectedOnly => {
        // Orchestrator: Only connected tools in flow
        for tool_spec in &input.available_tools {
            ai_node.add_tool(tool);
        }
    }
    ToolAccessPolicy::NoTools => {
        // Text Only: No tools configured
    }
}
```

## 🚀 Performance & Scalability

### Concurrent Execution
- **Thread-Safe**: All components use Arc<Mutex<>> for safe concurrent access
- **Async/Await**: Non-blocking execution throughout the pipeline
- **Event-Driven**: Real-time updates without polling

### Resource Management
- **Model Session Pooling**: Reuse sessions for same model
- **Tool Caching**: Cache tool discovery results
- **Memory Efficiency**: Minimal overhead per flow execution

### Error Handling
- **Graceful Degradation**: Continue flow execution on non-critical failures
- **Retry Logic**: Automatic retry for transient failures
- **Comprehensive Logging**: Detailed error reporting and debugging

## 🔮 Future Enhancements

### Planned Features
1. **StrictOnly Implementation**: Complete strict parameter validation
2. **Conversation History**: Persistent context across flow executions
3. **Model Load Balancing**: Distribute requests across multiple model instances
4. **Advanced Caching**: Cache AI responses for repeated queries
5. **Custom Model Support**: Support for local and custom AI models

### Architecture Extensions
1. **Plugin System**: Allow custom AI behaviors via plugins
2. **Workflow Templates**: Pre-built AI workflow patterns
3. **Performance Analytics**: Detailed execution metrics and optimization
4. **Multi-Model Orchestration**: Coordinate multiple models in single flow

---

This architecture provides a solid foundation for building complex AI-powered workflows while maintaining simplicity and performance. 