# 🚀 Simple Core Separation Approach

## 📋 Executive Summary

This document outlines a **pragmatic, low-risk approach** to extract VortiDeck's flow execution core into a standalone crate that can run headless. Unlike the comprehensive restructuring proposed in `core-separation-guide.md`, this approach focuses on **quick wins with minimal complexity**.

## 🎯 Philosophy: Evolution over Revolution

### **The Problem with Complex Architecture**
The existing `core-separation-guide.md` proposes an excellent but **over-engineered solution**:
- 14,029+ lines to refactor
- Complex multi-trait hierarchy
- 8-week timeline with high risk
- Multiple new crates and workspace restructuring

### **Our Simple Solution: Adapter Pattern + Event Abstraction**
- **~4,500 lines** to refactor initially
- **Single abstraction point** (events)
- **2-3 week timeline** with low risk
- **Feature-gated** single crate approach

## 🏗️ Target Architecture

### **Current State**
```
FlowExecutor
├── app_handle: AppHandle          ❌ Tight Tauri coupling
├── flow_registry: Arc<FlowRegistryState>
├── chat_state: Arc<ChatState>
└── Methods directly use tauri::Emitter
```

### **Target State**
```
libs/vortideck-flow-core/
├── src/
│   ├── lib.rs                     # Main exports
│   ├── executor/
│   │   ├── mod.rs
│   │   ├── core.rs               # Platform-agnostic executor
│   │   └── events.rs             # Event abstraction
│   ├── nodes/
│   │   ├── mod.rs
│   │   ├── traits.rs             # FlowNode trait
│   │   ├── standard.rs           # Standard nodes
│   │   └── registry.rs           # Node registry
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── integration.rs        # MCP tool integration
│   │   └── adapter.rs            # Flow adapter
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── traits.rs             # Storage abstraction
│   │   └── sqlite.rs             # SQLite implementation
│   └── adapters/
│       ├── mod.rs
│       ├── tauri.rs              # Tauri integration
│       └── headless.rs           # Headless runtime
└── Cargo.toml                    # Feature-gated dependencies
```

## 🔧 Core Abstractions

### **1. Event System Abstraction**

**The Key Insight:** Most Tauri coupling comes from event emission. Abstract this single point:

```rust
// libs/vortideck-flow-core/src/executor/events.rs
use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit<T: Serialize + Send>(&self, event: &str, payload: T) -> Result<(), String>;
}

// Headless implementation (logging or no-op)
pub struct HeadlessEventEmitter {
    enable_logging: bool,
}

#[async_trait]
impl EventEmitter for HeadlessEventEmitter {
    async fn emit<T: Serialize + Send>(&self, event: &str, payload: T) -> Result<(), String> {
        if self.enable_logging {
            let payload_str = serde_json::to_string(&payload).unwrap_or_default();
            tracing::info!("Event [{}]: {}", event, payload_str);
        }
        Ok(())
    }
}

// Tauri implementation (feature-gated)
#[cfg(feature = "tauri")]
pub struct TauriEventEmitter {
    app_handle: tauri::AppHandle,
}

#[cfg(feature = "tauri")]
#[async_trait]
impl EventEmitter for TauriEventEmitter {
    async fn emit<T: Serialize + Send>(&self, event: &str, payload: T) -> Result<(), String> {
        use tauri::Emitter;
        self.app_handle.emit(event, payload).map_err(|e| e.to_string())
    }
}
```

### **2. Storage Abstraction**

```rust
// libs/vortideck-flow-core/src/storage/traits.rs
use async_trait::async_trait;
use crate::types::{FlowDefinition, FlowExecutionState, FlowSummary};

#[async_trait]
pub trait FlowStorage: Send + Sync {
    async fn save_flow(&self, flow: &FlowDefinition) -> Result<(), String>;
    async fn load_flow(&self, flow_id: &str) -> Result<FlowDefinition, String>;
    async fn save_execution_state(&self, state: &FlowExecutionState) -> Result<(), String>;
    async fn load_execution_state(&self, flow_id: &str) -> Result<FlowExecutionState, String>;
    async fn list_flows(&self) -> Result<Vec<FlowSummary>, String>;
    async fn delete_flow(&self, flow_id: &str) -> Result<(), String>;
}

// SQLite implementation (works for both Tauri and headless)
pub struct SqliteFlowStorage {
    db_path: String,
}

#[async_trait]
impl FlowStorage for SqliteFlowStorage {
    // Implementation moved from current executor.rs
    async fn save_flow(&self, flow: &FlowDefinition) -> Result<(), String> {
        // Current implementation from executor.rs
    }
    // ... other methods
}
```

### **3. Core Executor (Platform Agnostic)**

```rust
// libs/vortideck-flow-core/src/executor/core.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::executor::events::EventEmitter;
use crate::storage::FlowStorage;
use crate::types::*;

pub struct FlowExecutorCore<E: EventEmitter, S: FlowStorage> {
    flow_registry: Arc<FlowRegistryState>,
    chat_state: Arc<ChatState>,
    event_emitter: Arc<E>,
    active_flows: Arc<Mutex<HashMap<String, FlowExecutionState>>>,
    storage: Arc<S>,
}

impl<E: EventEmitter, S: FlowStorage> FlowExecutorCore<E, S> {
    pub fn new(
        flow_registry: Arc<FlowRegistryState>,
        chat_state: Arc<ChatState>,
        event_emitter: Arc<E>,
        storage: Arc<S>,
    ) -> Self {
        Self {
            flow_registry,
            chat_state,
            event_emitter,
            active_flows: Arc::new(Mutex::new(HashMap::new())),
            storage,
        }
    }

    // All current executor methods, but using self.event_emitter instead of app_handle
    pub async fn start_flow(&self, flow_definition: FlowDefinition) -> Result<String, String> {
        // Exact same logic as current executor.rs:354
        // But replace: self.app_handle.emit() 
        // With: self.event_emitter.emit()
    }

    pub async fn execute_flow_internal(&self, flow_id: String) -> Result<(), String> {
        // Exact same logic as current executor.rs:446
        // But use self.event_emitter for all events
    }

    // ... all other methods from current executor.rs
    // Just replace app_handle.emit() calls with self.event_emitter.emit()
}
```

## 📦 Feature-Gated Dependencies

```toml
# libs/vortideck-flow-core/Cargo.toml
[package]
name = "vortideck-flow-core"
version = "0.1.0"
edition = "2021"

[features]
default = ["headless"]
headless = ["tracing"]
tauri = ["dep:tauri"]

[dependencies]
# Core dependencies (always included)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread", "sync"] }
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite"] }

# Standard node dependencies
rodio = "0.17"  # For audio nodes

# Optional Tauri integration
tauri = { version = "2.0", optional = true, features = ["api-all"] }
tracing = { version = "0.1", optional = true }
```

## 🔌 Adapter Implementations

### **Tauri Adapter**

```rust
// libs/vortideck-flow-core/src/adapters/tauri.rs
#[cfg(feature = "tauri")]
pub struct TauriFlowExecutor {
    core: FlowExecutorCore<TauriEventEmitter, SqliteFlowStorage>,
}

#[cfg(feature = "tauri")]
impl TauriFlowExecutor {
    pub fn new(
        flow_registry: Arc<FlowRegistryState>,
        chat_state: Arc<ChatState>,
        app_handle: tauri::AppHandle,
        db_path: &str,
    ) -> Self {
        let event_emitter = Arc::new(TauriEventEmitter::new(app_handle));
        let storage = Arc::new(SqliteFlowStorage::new(db_path));
        let core = FlowExecutorCore::new(flow_registry, chat_state, event_emitter, storage);
        
        Self { core }
    }

    // Delegate all methods to core
    pub async fn start_flow(&self, flow: FlowDefinition) -> Result<String, String> {
        self.core.start_flow(flow).await
    }

    pub async fn get_flow_status(&self, flow_id: &str) -> Result<FlowExecutionState, String> {
        self.core.get_flow_status(flow_id).await
    }

    // ... all other public methods
}
```

### **Headless Adapter**

```rust
// libs/vortideck-flow-core/src/adapters/headless.rs
pub struct HeadlessFlowExecutor {
    core: FlowExecutorCore<HeadlessEventEmitter, SqliteFlowStorage>,
}

impl HeadlessFlowExecutor {
    pub fn new(
        flow_registry: Arc<FlowRegistryState>,
        chat_state: Arc<ChatState>,
        db_path: &str,
        enable_logging: bool,
    ) -> Self {
        let event_emitter = Arc::new(HeadlessEventEmitter::new(enable_logging));
        let storage = Arc::new(SqliteFlowStorage::new(db_path));
        let core = FlowExecutorCore::new(flow_registry, chat_state, event_emitter, storage);
        
        Self { core }
    }

    // Synchronous execution for CLI/server use
    pub async fn run_flow_sync(&self, flow: FlowDefinition) -> Result<HashMap<String, Value>, String> {
        let flow_id = self.core.start_flow(flow).await?;
        
        // Wait for completion
        loop {
            let status = self.core.get_flow_status(&flow_id).await?;
            match status.status {
                FlowStatus::Completed => {
                    return Ok(status.node_results.into_iter()
                        .map(|(k, v)| (k, serde_json::to_value(v.outputs).unwrap_or_default()))
                        .collect());
                }
                FlowStatus::Failed => {
                    return Err(format!("Flow execution failed"));
                }
                _ => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}
```

## 📋 Implementation Plan

### **Phase 1: Create Core Crate (Week 1)**

1. **Create crate structure**:
```bash
mkdir -p libs/vortideck-flow-core/src/{executor,nodes,storage,adapters,mcp}
```

2. **Move core types** (from current executor.rs):
   - `FlowDefinition`, `FlowExecutionState`, `FlowEvent`, etc.
   - Move to `libs/vortideck-flow-core/src/types.rs`

3. **Extract traits** (current traits.rs is already clean):
   - Copy `apps/tauri-windows/src-tauri/src/flow_control/traits.rs`
   - To `libs/vortideck-flow-core/src/nodes/traits.rs`

### **Phase 2: Abstract Event System (Week 1)**

1. **Create event abstraction** (`src/executor/events.rs`)
2. **Implement both variants** (Tauri + Headless)
3. **Test event emission** works in both modes

### **Phase 3: Extract Core Logic (Week 2)**

1. **Copy executor.rs** to `src/executor/core.rs`
2. **Replace all `app_handle.emit()`** with `self.event_emitter.emit()`
3. **Generic over EventEmitter and Storage** traits
4. **Remove direct Tauri imports**

### **Phase 4: Create Adapters (Week 2)**

1. **Implement TauriFlowExecutor** adapter
2. **Implement HeadlessFlowExecutor** adapter
3. **Update Tauri app** to use adapter
4. **Test compatibility** with existing flows

### **Phase 5: Extract Additional Components (Week 3)**

1. **Move standard nodes** (platform-agnostic only)
2. **Extract MCP integration** (remove Tauri coupling)
3. **Create storage abstraction**
4. **Test all components**

## 🔄 Migration Strategy

### **Backward Compatibility**

Update Tauri app gradually:

```rust
// apps/tauri-windows/src-tauri/src/main.rs
use vortideck_flow_core::adapters::tauri::TauriFlowExecutor;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Create existing state (unchanged)
            let flow_registry = Arc::new(FlowRegistryState::new());
            let chat_state = Arc::new(ChatState::new());
            
            // Create new executor (using adapter)
            let flow_executor = Arc::new(TauriFlowExecutor::new(
                flow_registry.clone(),
                chat_state.clone(),
                app.handle().clone(),
                "flows.db",
            ));
            
            // Manage both for transition period
            app.manage(flow_registry);  // Keep for compatibility
            app.manage(chat_state);     // Keep for compatibility
            app.manage(flow_executor);  // New executor
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Update commands to use new executor
            execute_flow_new,
            // Keep old commands during transition
            execute_flow_old,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// New command using core
#[tauri::command]
async fn execute_flow_new(
    flow: FlowDefinition,
    executor: tauri::State<'_, Arc<TauriFlowExecutor>>,
) -> Result<String, String> {
    executor.start_flow(flow).await
}

// Old command (keep during transition)
#[tauri::command]
async fn execute_flow_old(
    flow: FlowDefinition,
    flow_registry: tauri::State<'_, Arc<FlowRegistryState>>,
    chat_state: tauri::State<'_, Arc<ChatState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Current implementation
}
```

## 🖥️ Headless Usage Examples

### **CLI Tool**

```rust
// examples/cli-executor/src/main.rs
use vortideck_flow_core::adapters::headless::HeadlessFlowExecutor;
use vortideck_flow_core::types::FlowDefinition;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flow_json = std::fs::read_to_string("flow.json")?;
    let flow: FlowDefinition = serde_json::from_str(&flow_json)?;
    
    // Create headless executor
    let registry = Arc::new(FlowRegistryState::new());
    let chat_state = Arc::new(ChatState::new());
    
    let executor = HeadlessFlowExecutor::new(
        registry,
        chat_state,
        "headless_flows.db",
        true, // enable logging
    );
    
    // Execute flow synchronously
    let results = executor.run_flow_sync(flow).await?;
    
    println!("Flow completed: {}", serde_json::to_string_pretty(&results)?);
    Ok(())
}
```

### **HTTP Server**

```rust
// examples/flow-server/src/main.rs
use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use vortideck_flow_core::adapters::headless::HeadlessFlowExecutor;

#[tokio::main]
async fn main() {
    let executor = Arc::new(create_headless_executor());
    
    let app = Router::new()
        .route("/execute", post(execute_flow_endpoint))
        .with_state(executor);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn execute_flow_endpoint(
    State(executor): State<Arc<HeadlessFlowExecutor>>,
    Json(flow): Json<FlowDefinition>,
) -> Result<Json<Value>, StatusCode> {
    match executor.run_flow_sync(flow).await {
        Ok(results) => Ok(Json(serde_json::to_value(results).unwrap())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

## ✅ Benefits of This Approach

### **1. Low Risk**
- **Single abstraction point** (events) = easy to debug
- **Incremental migration** = can rollback at any point
- **Backward compatibility** maintained throughout

### **2. Quick Time to Market**
- **2-3 weeks** to working headless core
- **Minimal code changes** required
- **Immediate value** from headless capability

### **3. Evolutionary Architecture**
- **Foundation for future complexity** if needed
- **Can adopt complex architecture later** when requirements are clearer
- **Proven patterns** (Adapter, Strategy) well-understood

### **4. Practical Benefits**
- **Feature-gated builds** = smaller binaries when Tauri not needed
- **Clean separation** = easier testing and maintenance
- **Platform flexibility** = can integrate with other frameworks later

## 🎯 Success Metrics

### **Week 1: Foundation**
- [ ] Core crate compiles without Tauri dependencies
- [ ] Event abstraction works for both modes
- [ ] Types extracted and shared

### **Week 2: Core Functionality**
- [ ] Headless executor can run simple flows
- [ ] Tauri adapter maintains compatibility
- [ ] All existing tests pass

### **Week 3: Full Feature Parity**
- [ ] All node types work headless
- [ ] MCP integration works headless
- [ ] Storage abstraction complete
- [ ] Performance within 10% of current

## 🚨 Potential Risks & Mitigations

### **Risk: Event Abstraction Insufficient**
- **Mitigation**: Start with logging-based headless mode to identify missing events
- **Backup**: Add more specific trait methods if needed

### **Risk: MCP State Coupling**
- **Mitigation**: Extract MCP client to be stateless where possible
- **Backup**: Create MCP state adapter similar to event adapter

### **Risk: Performance Degradation**
- **Mitigation**: Benchmark during development
- **Backup**: Zero-cost abstractions and optimization passes

This approach prioritizes **getting working headless capability quickly** while maintaining a clear path to more sophisticated architecture as needed. The key insight is that most Tauri coupling comes from event emission - abstract that one concern well, and the rest follows naturally. 