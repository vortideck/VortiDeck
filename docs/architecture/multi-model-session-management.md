# Multi-Model Session Management Architecture

## Current State Analysis

### Current Architecture
- **Single Global Session**: One `ChatSession` instance managed globally
- **Single Model**: All AI operations use the same model configuration
- **Eager Initialization**: Session created during app startup via `init_chat` command
- **Frontend Dependency**: App requires manual initialization before AI operations

### Current Flow
```
App Launch → Frontend calls init_chat → Single ChatSession created → All AI nodes use same session/model
```

### Problems with Current Approach
1. **No Model Flexibility**: All AI nodes forced to use same model
2. **Startup Dependency**: App not ready for flows until manual initialization
3. **Resource Waste**: Single session may use expensive model for simple tasks
4. **Trigger Limitations**: External triggers can't use AI until frontend initializes

## Design Options Analysis

### Option 1: Lazy Session Pool (Recommended)
**Concept**: Create sessions on-demand per model, no upfront initialization required.

#### Architecture
```rust
pub struct ModelSessionManager {
    sessions: HashMap<String, ChatSession>,
    config: Config,
    mcp_clients: HashMap<String, RunningService<RoleClient, ()>>,
    default_model: String,
}

pub struct ChatState {
    session_manager: Arc<Mutex<ModelSessionManager>>,
}
```

#### Flow
```
App Launch → Load config → Ready for any trigger
↓
AI Node Execution → Get/Create session for specified model → Execute
```

#### Pros
- ✅ **Zero startup time** - No upfront session creation
- ✅ **Model flexibility** - Each node can specify different model
- ✅ **Resource efficient** - Only create sessions when needed
- ✅ **External trigger ready** - App immediately ready for any flow
- ✅ **Cost optimization** - Use cheap models for simple tasks, expensive for complex
- ✅ **Backward compatible** - Existing flows work with default model

#### Cons
- ⚠️ **First-use latency** - Small delay when creating new model session
- ⚠️ **Memory usage** - Multiple sessions in memory (but only when used)

### Option 2: Eager Multi-Session Pool
**Concept**: Pre-create sessions for all configured models at startup.

#### Architecture
```rust
pub struct ModelSessionManager {
    sessions: HashMap<String, ChatSession>, // Pre-populated
    default_model: String,
}
```

#### Flow
```
App Launch → Create sessions for ALL models → Ready
```

#### Pros
- ✅ **No runtime latency** - All sessions ready
- ✅ **Model flexibility** - Each node can use different model
- ✅ **Predictable performance** - No surprises during execution

#### Cons
- ❌ **Slow startup** - Must initialize all models upfront
- ❌ **Resource waste** - Memory/connections for unused models
- ❌ **Startup failures** - One bad model config breaks entire app
- ❌ **External trigger delay** - Must wait for full initialization

### Option 3: Hybrid Approach
**Concept**: Pre-create default model session, lazy-load others.

#### Architecture
```rust
pub struct ModelSessionManager {
    sessions: HashMap<String, ChatSession>,
    default_model: String,
    config: Config,
    mcp_clients: HashMap<String, RunningService<RoleClient, ()>>,
}
```

#### Flow
```
App Launch → Create default session → Ready for basic flows
↓
Non-default model needed → Create session on-demand
```

#### Pros
- ✅ **Fast startup** - Only default model initialized
- ✅ **External trigger ready** - Can handle flows immediately
- ✅ **Model flexibility** - Support for multiple models
- ✅ **Resource balanced** - Default ready, others on-demand

#### Cons
- ⚠️ **Complexity** - More complex session management logic
- ⚠️ **Partial latency** - Delay for non-default models

## Recommended Architecture: Lazy Session Pool

### Core Components

#### 1. ModelSessionManager
```rust
pub struct ModelSessionManager {
    sessions: HashMap<String, ChatSession>,
    config: Config,
    mcp_clients: HashMap<String, RunningService<RoleClient, ()>>,
    default_model: String,
    tool_set: ToolSet,
}

impl ModelSessionManager {
    pub async fn get_or_create_session(&mut self, model_id: Option<&str>) -> Result<&ChatSession, String>
    pub async fn create_session_for_model(&mut self, model_id: &str) -> Result<ChatSession, String>
    pub fn get_available_models(&self) -> Vec<ModelInfo>
    pub fn get_default_model(&self) -> &str
}
```

#### 2. Updated ChatState
```rust
pub struct ChatState {
    session_manager: Arc<Mutex<ModelSessionManager>>,
}

impl ChatState {
    pub async fn new() -> Result<Self, String> {
        let config = Config::load_default().await?;
        let mcp_clients = config.create_mcp_clients().await?;
        let tool_set = create_tool_set(&mcp_clients).await?;
        
        let session_manager = ModelSessionManager::new(config, mcp_clients, tool_set)?;
        
        Ok(Self {
            session_manager: Arc::new(Mutex::new(session_manager)),
        })
    }
}
```

#### 3. AI Node Execution Flow
```rust
pub async fn execute_ai_node(
    &self,
    config: AiNodeConfig,
    input: AiNodeInput,
    node_model: Option<String>, // New parameter
) -> Result<AiNodeOutput, String> {
    let model_id = node_model.as_deref();
    
    let session = {
        let mut manager = self.chat_state.session_manager.lock().await;
        manager.get_or_create_session(model_id).await?
    };
    
    // Execute with model-specific session
    self.execute_with_session(session, config, input).await
}
```

### Configuration Structure

#### Enhanced Config Format
```json
{
  "models": {
    "default": "gpt-3.5-turbo",
    "model": [
      {
        "id": "gpt-3.5-turbo",
        "name": "GPT-3.5 Turbo",
        "provider": "openai",
        "url": "https://api.openai.com/v1/chat/completions",
        "apiKey": "sk-...",
        "cost_per_token": 0.0000015,
        "max_tokens": 4096,
        "capabilities": ["text", "function_calling"]
      },
      {
        "id": "gpt-4",
        "name": "GPT-4",
        "provider": "openai", 
        "url": "https://api.openai.com/v1/chat/completions",
        "apiKey": "sk-...",
        "cost_per_token": 0.00003,
        "max_tokens": 8192,
        "capabilities": ["text", "function_calling", "advanced_reasoning"]
      }
    ]
  },
  "mcp": {
    "server": [...]
  }
}
```

### Frontend Integration

#### AI Node Model Selection
```vue
<template>
  <div class="ai-node">
    <select v-model="selectedModel" @change="updateNodeModel">
      <option value="">Default Model</option>
      <option v-for="model in availableModels" :key="model.id" :value="model.id">
        {{ model.name }} ({{ model.provider }})
      </option>
    </select>
  </div>
</template>

<script>
export default {
  data() {
    return {
      selectedModel: '',
      availableModels: []
    }
  },
  async mounted() {
    this.availableModels = await invoke('get_available_models');
  }
}
</script>
```

#### Flow Execution
```javascript
// Frontend flow execution - no changes needed
await invoke('start_flow_execution', { 
  flow: flowDefinition 
});

// Backend automatically handles model selection per node
```

### Migration Strategy

#### Phase 1: Backward Compatibility
1. Keep existing `init_chat` command working
2. Add new session manager alongside current system
3. Default to current behavior if no model specified

#### Phase 2: Enhanced Features
1. Add `get_available_models` command
2. Update frontend to show model selection
3. Add model selection to AI node configuration

#### Phase 3: Full Migration
1. Remove dependency on `init_chat` for basic functionality
2. Make session manager the primary system
3. Update documentation and examples

### Performance Considerations

#### Memory Management
- **Session Cleanup**: Implement LRU cache for sessions
- **Connection Pooling**: Reuse HTTP connections per model
- **Resource Limits**: Maximum number of concurrent sessions

#### Latency Optimization
- **Connection Warmup**: Pre-establish connections for frequently used models
- **Caching**: Cache model metadata and capabilities
- **Parallel Initialization**: Create sessions concurrently when possible

### Error Handling

#### Graceful Degradation
```rust
pub async fn get_or_create_session(&mut self, model_id: Option<&str>) -> Result<&ChatSession, String> {
    let target_model = model_id.unwrap_or(&self.default_model);
    
    // Try requested model
    if let Ok(session) = self.create_session_for_model(target_model).await {
        return Ok(session);
    }
    
    // Fallback to default if different
    if target_model != self.default_model {
        warn!("Failed to create session for {}, falling back to default", target_model);
        return self.create_session_for_model(&self.default_model).await;
    }
    
    Err("Failed to create session for any available model".to_string())
}
```

#### Model Validation
- Validate model configurations at startup
- Test connectivity for critical models
- Provide clear error messages for configuration issues

### External Trigger Readiness

#### Immediate Availability
```rust
// App startup sequence
async fn setup_app() -> Result<(), Error> {
    // 1. Load configuration (fast)
    let config = Config::load_default().await?;
    
    // 2. Initialize MCP clients (moderate)
    let mcp_clients = config.create_mcp_clients().await?;
    
    // 3. Create session manager (fast - no sessions yet)
    let session_manager = ModelSessionManager::new(config, mcp_clients)?;
    
    // 4. App ready for any trigger!
    Ok(())
}
```

#### Trigger Handling
```rust
// External trigger (webhook, schedule, etc.)
async fn handle_external_trigger(flow_id: &str) -> Result<(), String> {
    // App is immediately ready - no init_chat needed
    let flow_def = load_flow_definition(flow_id)?;
    let executor = FlowExecutor::new(/* ... */);
    executor.start_flow(flow_def).await
}
```

## Implementation Checklist

### Core Changes
- [ ] Create `ModelSessionManager` struct
- [ ] Update `ChatState` to use session manager
- [ ] Modify `AiExecutor` to accept model parameter
- [ ] Update `FlowExecutor` to extract and pass model info
- [ ] Add `get_available_models` command

### Configuration
- [ ] Enhance config format with model metadata
- [ ] Add default model specification
- [ ] Validate model configurations at startup

### Frontend Updates
- [ ] Add model selection UI to AI nodes
- [ ] Update flow serialization to include model info
- [ ] Add model management in settings

### Testing & Migration
- [ ] Ensure backward compatibility
- [ ] Test external trigger scenarios
- [ ] Performance testing with multiple models
- [ ] Documentation updates

## Conclusion

The **Lazy Session Pool** approach provides the best balance of:
- **Immediate readiness** for external triggers
- **Resource efficiency** through on-demand creation
- **Model flexibility** for cost and performance optimization
- **Backward compatibility** with existing flows

This architecture ensures VortiDeck is always ready to handle any flow execution, regardless of trigger source, while providing the flexibility to optimize AI operations through intelligent model selection. 