# 🔧 VortiDeck Core Separation Guide

## 📋 Executive Summary

This document outlines the complete plan to extract VortiDeck's core flow execution logic into a separate `flow-core` crate, enabling headless deployment while maintaining full compatibility with the existing Tauri application.

## 🏗️ Current Architecture Analysis

### 1. 📂 Complete Tauri App Structure
```
apps/tauri-windows/src-tauri/src/
├── 🚀 ENTRY POINT
│   ├── main.rs                           # 🎯 Tauri app entry (1579 lines)
│   ├── lib.rs                            # 📦 Library root (29 lines)
│   └── shared_state.rs                   # 🔗 Tauri state management (642 lines)
│
├── 🎯 CORE FLOW LOGIC (EXTRACT TO CORE)
│   └── flow_control/
│       ├── mod.rs                        # Module exports
│       ├── executor.rs                   # ⚡ Core execution logic (1169 lines)
│       ├── ai_executor.rs                # 🤖 AI node execution (585 lines)
│       ├── standard_nodes.rs             # 📱 Standard node implementations (358 lines)
│       ├── registry.rs                   # 📚 Node registry management (92 lines)
│       ├── traits.rs                     # 🔧 Core traits and interfaces (38 lines)
│       ├── system_adapter.rs             # 🖥️ System integration layer (74 lines)
│       ├── test_flow.rs                  # 🧪 Testing utilities (215 lines)
│       └── flow_control.rs               # 📋 Main module coordinator (15 lines)
│
├── 🤖 MCP & AI INTEGRATION (NEEDS ABSTRACTION)
│   └── mcp/
│       ├── mod.rs                        # Module exports (29 lines)
│       ├── ai_node.rs                    # ⚡ AI node logic (602 lines)
│       ├── commands.rs                   # 🔗 MCP commands (1280 lines)
│       ├── unified_tool_executor.rs      # 🛠️ Tool execution (273 lines)
│       ├── chat.rs                       # 💬 Chat session management (241 lines)
│       ├── tool.rs                       # 🔧 Tool definitions (147 lines)
│       ├── config.rs                     # ⚙️ MCP configuration (824 lines)
│       ├── server.rs                     # 🖥️ MCP server management (60 lines)
│       ├── flow_adapter.rs               # 🔌 Flow integration (100 lines)
│       ├── tools.rs                      # 🛠️ Tool registry (135 lines)
│       ├── model.rs                      # 📊 Data models (150 lines)
│       ├── client.rs                     # 📱 MCP client (130 lines)
│       └── error.rs                      # ❌ Error handling (23 lines)
│
├── 🖥️ SYSTEM CONTROL (PLATFORM-SPECIFIC)
│   └── system_control/
│       ├── mod.rs                        # Module exports (11 lines)
│       ├── system_control.rs             # 🎮 System operations (535 lines)
│       ├── flow_adapter.rs               # 🔌 Flow integration (122 lines)
│       ├── commands.rs                   # 📞 Tauri commands (39 lines)
│       └── flow_traits.rs                # 🔧 Flow traits (38 lines)
│
├── 💾 DATABASE LAYER (NEEDS ABSTRACTION)
│   └── db/
│       ├── mod.rs                        # Module exports (11 lines)
│       ├── db.rs                         # 🗄️ Database connection (362 lines)
│       ├── utils.rs                      # 🛠️ Database utilities (1634 lines)
│       ├── models.rs                     # 📊 Data models (524 lines)
│       ├── schema.rs                     # 📋 Database schema (124 lines)
│       └── migrations/                   # 🔄 Database migrations
│
├── 📞 TAURI COMMANDS (PLATFORM-SPECIFIC)
│   └── commands/
│       ├── mod.rs                        # Module exports (18 lines)
│       ├── open_mobile_simulator.rs      # 📱 Mobile simulator (222 lines)
│       ├── run_extension.rs              # 🧩 Extension runner (82 lines)
│       ├── generate_qr_code.rs           # 🔲 QR code generation (57 lines)
│       ├── retrieve_peers.rs             # 🌐 Peer discovery (32 lines)
│       ├── get_installed_apps.rs         # 📱 App detection (163 lines)
│       ├── open_logs.rs                  # 📄 Log management (58 lines)
│       ├── plugins.rs                    # 🔌 Plugin system (25 lines)
│       └── discover_websocket.rs         # 🌐 WebSocket discovery (136 lines)
│
├── 🔧 UTILITY MODULES (MIXED)
│   ├── auth.rs                           # 🔐 Authentication (188 lines)
│   ├── utils.rs                          # 🛠️ General utilities (136 lines)
│   ├── macros.rs                         # 📝 Tauri macros (259 lines)
│   ├── obs.rs                            # 📹 OBS integration (318 lines)
│   ├── keys__and_macros.rs               # ⌨️ Key handling (81 lines)
│   ├── icons.rs                          # 🎨 Icon management (80 lines)
│   ├── create_websocket_dns.rs           # 🌐 WebSocket DNS (84 lines)
│   └── create_websocket_mdns.rs          # 🌐 WebSocket mDNS (44 lines)
│
├── 📁 AUDIO MODULE (EMPTY)
│   └── audio/
│
└── 📚 HISTORICAL FILES (IGNORE)
    └── main_history/
        ├── main_new_temp.rs              # 🗂️ Temporary main (221 lines)
        ├── main_old.rs                   # 🗂️ Old main (261 lines)
        ├── main_V2_webs_tls_opt.rs       # 🗂️ WebSocket main (609 lines)
        └── rustlc.txt                    # 📄 Rust config
```

### 2. 🔍 Module-by-Module Separation Analysis

#### 🎯 **EXTRACT TO FLOW-CORE** (Platform Agnostic)
```rust
// Core flow execution logic - NO platform dependencies
flow_control/
├── executor.rs          ⚡ Universal flow executor
├── ai_executor.rs       🤖 AI node execution logic  
├── standard_nodes.rs    📱 Platform-agnostic nodes only
├── registry.rs          📚 Node registry (abstracted)
├── traits.rs           🔧 Core execution traits
└── test_flow.rs        🧪 Testing utilities

// MCP client logic - WITH abstraction layer
mcp/
├── ai_node.rs          ⚡ Core AI node logic
├── unified_tool_executor.rs  🛠️ Tool execution engine
├── tool.rs             🔧 Tool definitions
├── tools.rs            🛠️ Tool registry
├── model.rs            📊 Data models
└── error.rs            ❌ Error types

// Database models - ABSTRACTED
db/
├── models.rs           📊 Data models only
└── schema.rs           📋 Database schema definitions
```

#### 🔌 **CREATE ABSTRACTION LAYER** (Bridge Platform-Specific)
```rust
// MCP integration - needs state abstraction
mcp/
├── chat.rs             💬 Chat session (via ChatSessionProvider)
├── config.rs           ⚙️ Configuration (via ConfigProvider) 
├── client.rs           📱 MCP client (via McpClientProvider)
└── server.rs           🖥️ Server management (via ServerProvider)

// Database operations - needs storage abstraction  
db/
├── db.rs               🗄️ Database connection (via StorageProvider)
└── utils.rs            🛠️ Database utilities (via StorageProvider)

// System control - needs system abstraction
system_control/
├── system_control.rs   🎮 System operations (via SystemAdapter)
├── flow_adapter.rs     🔌 Flow integration bridge
└── flow_traits.rs      🔧 System flow traits
```

#### 🏠 **STAY IN TAURI APP** (Platform-Specific)
```rust
// Tauri application layer
main.rs                 🚀 Tauri app entry point
shared_state.rs          🔗 Tauri state management
lib.rs                  📦 Library root

// Tauri-specific commands
commands/
├── open_mobile_simulator.rs  📱 Mobile simulator
├── run_extension.rs          🧩 Extension runner  
├── get_installed_apps.rs     📱 App detection
├── open_logs.rs              📄 Log management
├── plugins.rs                🔌 Plugin system
└── discover_websocket.rs     🌐 WebSocket discovery

// Platform utilities
auth.rs                 🔐 Authentication (Tauri-specific)
macros.rs               📝 Tauri macros
obs.rs                  📹 OBS integration (Windows-specific)
keys__and_macros.rs     ⌨️ Key handling (Windows-specific)
icons.rs                🎨 Icon management (Tauri-specific)
create_websocket_dns.rs  🌐 WebSocket DNS (platform networking)
create_websocket_mdns.rs 🌐 WebSocket mDNS (platform networking)
utils.rs                🛠️ Platform utilities
```

#### ❌ **IGNORE/REMOVE** (Legacy)
```rust
main_history/           📚 Historical files (can be deleted)
audio/                  📁 Empty directory (remove)
```

### 3. 🔗 Current Tight Coupling Issues

#### Critical Dependencies to Abstract
```rust
// 1. FlowExecutor ← Tauri AppHandle (executor.rs)
pub struct FlowExecutor {
    app_handle: AppHandle,              // ❌ Direct Tauri coupling
    flow_registry: Arc<FlowRegistryState>, // ❌ Tauri state
    chat_state: Arc<ChatState>,            // ❌ Tauri state
}

// 2. MCP Commands ← Tauri State (mcp/commands.rs - 1280 lines!)
#[tauri::command]
pub async fn chat_with_mcp_tools(
    state: tauri::State<'_, ChatState>, // ❌ Tauri state dependency
    // ... 1280 lines of MCP command wrappers
) -> Result<String, String> { }

// 3. System Control ← Windows APIs (system_control/system_control.rs - 535 lines)
impl SystemControl {
    pub fn new() -> Self {
        // ❌ Direct Windows API calls
        // ❌ No abstraction layer
    }
}

// 4. Database Utils ← Direct SQLite (db/utils.rs - 1634 lines!)
pub async fn create_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // ❌ Direct SQLite dependency
    // ❌ No storage abstraction
}

// 5. MCP Config ← File System (mcp/config.rs - 824 lines)
impl Config {
    pub fn load() -> Result<Self, Error> {
        // ❌ Direct file system access
        // ❌ No configuration abstraction
    }
}
```

### 4. 📊 **Scale of Migration** (Total Lines to Refactor)

#### **Core Extraction** (→ flow-core)
- flow_control/: ~2,546 lines
- mcp/ (core logic): ~1,332 lines  
- db/ (models): ~648 lines
- **Total: ~4,526 lines** to extract

#### **Abstraction Layer Creation** (→ adapters)
- mcp/ (state-dependent): ~2,145 lines
- system_control/: ~696 lines
- db/ (operations): ~1,996 lines
- **Total: ~4,837 lines** to abstract

#### **Platform-Specific** (→ stay in Tauri)
- main.rs + shared_state.rs: ~2,221 lines
- commands/: ~775 lines
- Platform utilities: ~1,470 lines
- **Total: ~4,466 lines** remain platform-specific

#### **Grand Total: ~14,029 lines** to reorganize! 😱

## 🎯 Target Architecture Design

### 1. 📦 New Crate Structure
```
workspace/
├── crates/
│   ├── flow-core/                    # 🎯 NEW: Platform-agnostic core
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── executor/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── universal.rs      # Platform-agnostic executor
│   │   │   │   ├── state.rs          # State abstraction
│   │   │   │   └── traits.rs         # Core traits
│   │   │   ├── nodes/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── standard.rs       # Standard nodes
│   │   │   │   ├── ai.rs             # AI execution logic
│   │   │   │   └── traits.rs         # Node traits
│   │   │   ├── registry/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── universal.rs      # Platform-agnostic registry
│   │   │   │   └── traits.rs         # Registry traits
│   │   │   ├── storage/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── memory.rs         # In-memory storage
│   │   │   │   ├── sqlite.rs         # SQLite implementation
│   │   │   │   └── traits.rs         # Storage traits
│   │   │   ├── mcp/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── client.rs         # Abstracted MCP client
│   │   │   │   └── traits.rs         # MCP traits
│   │   │   └── types.rs              # Core type definitions
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   └── flow-server/                  # 🎯 NEW: Headless server
│       ├── src/
│       │   ├── main.rs
│       │   ├── api/
│       │   ├── adapters/
│       │   └── deployment/
│       ├── Cargo.toml
│       └── README.md
│
├── apps/
│   ├── tauri-windows/                # ✅ UPDATED: Tauri adapters
│   │   ├── src-tauri/src/
│   │   │   ├── flow_control/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── commands.rs       # Tauri command wrappers
│   │   │   │   ├── tauri_adapter.rs  # 🎯 NEW: Tauri state adapter
│   │   │   │   └── system_nodes.rs   # System-specific nodes
│   │   │   ├── shared_state.rs       # Tauri state management
│   │   │   └── main.rs
│   │   └── Cargo.toml
│   │
│   └── web-app/                      # Existing web frontend
│
└── Cargo.toml                        # Workspace root
```

### 2. 🔧 Core Abstraction Traits

#### State Provider Abstraction
```rust
// flow-core/src/executor/traits.rs
#[async_trait]
pub trait StateProvider: Send + Sync + 'static {
    async fn get_flow_registry(&self) -> Arc<dyn FlowRegistryProvider>;
    async fn get_chat_session(&self) -> Arc<dyn ChatSessionProvider>;
    async fn get_storage(&self) -> Arc<dyn FlowStorage>;
    async fn emit_event(&self, event: FlowEvent) -> Result<(), CoreError>;
    async fn get_system_adapter(&self) -> Option<Arc<dyn SystemAdapter>>;
}

#[async_trait]
pub trait FlowRegistryProvider: Send + Sync {
    fn get_node(&self, id: &str) -> Option<Arc<dyn FlowNode>>;
    fn get_all_node_info(&self) -> Vec<ToolNodeInfo>;
    fn register_node(&mut self, node: Arc<dyn FlowNode>);
    fn get_mcp_tools(&self) -> Vec<Arc<dyn Tool>>;
}

#[async_trait]
pub trait ChatSessionProvider: Send + Sync {
    async fn get_session(&self) -> Option<ChatSession>;
    async fn execute_tool(&self, name: &str, args: Value) -> Result<String, CoreError>;
    async fn list_available_tools(&self) -> Vec<ToolSpec>;
}

#[async_trait]
pub trait SystemAdapter: Send + Sync {
    async fn execute_system_command(&self, command: &str, args: Value) -> Result<Value, CoreError>;
    async fn get_system_info(&self) -> SystemInfo;
    fn get_allowed_operations(&self) -> Vec<String>;
}
```

#### Universal Flow Executor
```rust
// flow-core/src/executor/universal.rs
pub struct UniversalFlowExecutor {
    state_provider: Arc<dyn StateProvider>,
    active_flows: Arc<Mutex<HashMap<String, FlowExecutionState>>>,
    event_bus: Arc<dyn EventBus>,
}

impl UniversalFlowExecutor {
    pub async fn new(state_provider: Arc<dyn StateProvider>) -> Result<Self, CoreError> {
        Ok(Self {
            state_provider,
            active_flows: Arc::new(Mutex::new(HashMap::new())),
            event_bus: Arc::new(DefaultEventBus::new()),
        })
    }
    
    pub async fn execute_flow(&self, flow: FlowDefinition) -> Result<FlowExecutionResult, CoreError> {
        // Platform-agnostic flow execution logic
        let registry = self.state_provider.get_flow_registry().await;
        let storage = self.state_provider.get_storage().await;
        
        // Initialize execution state
        let execution_state = FlowExecutionState::new(&flow);
        self.active_flows.lock().await.insert(flow.id.clone(), execution_state);
        
        // Execute flow logic (moved from current executor.rs)
        self.execute_flow_internal(flow, registry, storage).await
    }
    
    async fn emit_event(&self, event: FlowEvent) -> Result<(), CoreError> {
        // Delegate to state provider
        self.state_provider.emit_event(event).await
    }
    
    async fn execute_standard_node(&self, node_def: &FlowNodeDefinition) -> Result<HashMap<String, Value>, CoreError> {
        // Platform-agnostic node execution
        let registry = self.state_provider.get_flow_registry().await;
        
        if let Some(node) = registry.get_node(&node_def.node_type) {
            let args = serde_json::to_value(&node_def.inputs)?;
            let result = node.execute(args).await?;
            
            let mut outputs = HashMap::new();
            outputs.insert("result".to_string(), result);
            Ok(outputs)
        } else {
            Err(CoreError::NodeNotFound(node_def.node_type.clone()))
        }
    }
}
```

### 3. 🔌 Platform Adapters

#### Tauri State Adapter
```rust
// apps/tauri-windows/src-tauri/src/flow_control/tauri_adapter.rs
use flow_core::executor::traits::*;
use flow_core::types::*;

pub struct TauriStateProvider {
    flow_registry: Arc<FlowRegistryState>,
    chat_state: Arc<ChatState>,
    app_handle: AppHandle,
    storage: Arc<dyn FlowStorage>,
}

impl TauriStateProvider {
    pub fn new(
        flow_registry: Arc<FlowRegistryState>,
        chat_state: Arc<ChatState>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            flow_registry,
            chat_state,
            app_handle,
            storage: Arc::new(SqliteFlowStorage::new("flows.db")),
        }
    }
}

#[async_trait]
impl StateProvider for TauriStateProvider {
    async fn get_flow_registry(&self) -> Arc<dyn FlowRegistryProvider> {
        Arc::new(TauriFlowRegistryAdapter::new(self.flow_registry.clone()))
    }
    
    async fn get_chat_session(&self) -> Arc<dyn ChatSessionProvider> {
        Arc::new(TauriChatAdapter::new(self.chat_state.clone()))
    }
    
    async fn get_storage(&self) -> Arc<dyn FlowStorage> {
        self.storage.clone()
    }
    
    async fn emit_event(&self, event: FlowEvent) -> Result<(), CoreError> {
        self.app_handle.emit("flow_event", &event)
            .map_err(|e| CoreError::EventEmissionFailed(e.to_string()))
    }
    
    async fn get_system_adapter(&self) -> Option<Arc<dyn SystemAdapter>> {
        Some(Arc::new(TauriSystemAdapter::new(self.app_handle.clone())))
    }
}

// Bridge between Tauri state and core traits
pub struct TauriFlowRegistryAdapter {
    registry: Arc<FlowRegistryState>,
}

impl TauriFlowRegistryAdapter {
    pub fn new(registry: Arc<FlowRegistryState>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl FlowRegistryProvider for TauriFlowRegistryAdapter {
    fn get_node(&self, id: &str) -> Option<Arc<dyn FlowNode>> {
        // Bridge to existing Tauri registry logic
        let registry_guard = self.registry.registry.lock().unwrap();
        registry_guard.get_node(id)
    }
    
    fn get_all_node_info(&self) -> Vec<ToolNodeInfo> {
        let registry_guard = self.registry.registry.lock().unwrap();
        registry_guard.get_all_node_info()
    }
    
    fn register_node(&mut self, node: Arc<dyn FlowNode>) {
        let mut registry_guard = self.registry.registry.lock().unwrap();
        registry_guard.register_node(node);
    }
    
    fn get_mcp_tools(&self) -> Vec<Arc<dyn Tool>> {
        // Bridge to existing MCP tools
        let registry_guard = self.registry.registry.lock().unwrap();
        registry_guard.get_mcp_tools()
    }
}
```

#### Headless State Adapter
```rust
// crates/flow-server/src/adapters/headless_adapter.rs
use flow_core::executor::traits::*;
use flow_core::types::*;

pub struct HeadlessStateProvider {
    registry: Arc<HeadlessFlowRegistry>,
    chat_session: Arc<HeadlessChat>,
    storage: Arc<dyn FlowStorage>,
    event_bus: Arc<dyn EventBus>,
}

impl HeadlessStateProvider {
    pub async fn new(config: ServerConfig) -> Result<Self, CoreError> {
        let registry = Arc::new(HeadlessFlowRegistry::new(&config).await?);
        let chat_session = Arc::new(HeadlessChat::new(&config).await?);
        let storage = Arc::new(PostgresFlowStorage::new(&config.database_url).await?);
        let event_bus = Arc::new(RedisEventBus::new(&config.redis_url).await?);
        
        Ok(Self {
            registry,
            chat_session, 
            storage,
            event_bus,
        })
    }
}

#[async_trait]
impl StateProvider for HeadlessStateProvider {
    async fn get_flow_registry(&self) -> Arc<dyn FlowRegistryProvider> {
        self.registry.clone()
    }
    
    async fn get_chat_session(&self) -> Arc<dyn ChatSessionProvider> {
        self.chat_session.clone()
    }
    
    async fn get_storage(&self) -> Arc<dyn FlowStorage> {
        self.storage.clone()
    }
    
    async fn emit_event(&self, event: FlowEvent) -> Result<(), CoreError> {
        self.event_bus.emit(event).await
    }
    
    async fn get_system_adapter(&self) -> Option<Arc<dyn SystemAdapter>> {
        // No system adapter in headless mode by default
        // Could be bridged through WebSocket to user's local agent
        None
    }
}
```

## 📋 Migration Plan

### Phase 1: Setup Infrastructure (Week 1)

#### 1.1 Create Workspace Structure
```bash
# Create new crates
mkdir -p crates/flow-core/src
mkdir -p crates/flow-server/src
mkdir -p apps/tauri-windows/src-tauri/src/flow_control

# Initialize Cargo.toml files
```

#### 1.2 Setup Cargo Workspace
```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/flow-core",
    "crates/flow-server", 
    "apps/tauri-windows/src-tauri",
    "apps/web-app"
]

[workspace.dependencies]
# Shared dependencies
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
async-trait = "0.1"
```

#### 1.3 flow-core Cargo.toml
```toml
# crates/flow-core/Cargo.toml
[package]
name = "flow-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
async-trait = { workspace = true }

# Storage
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "postgres"] }

# Optional MCP (feature-gated)
mcp-client = { path = "../../mcp-client", optional = true }

[features]
default = ["sqlite-storage"]
sqlite-storage = []
postgres-storage = []
mcp-integration = ["mcp-client"]
```

### Phase 2: Extract Core Types (Week 1-2)

#### 2.1 Move Type Definitions
```rust
// crates/flow-core/src/types.rs
// Move all types from layers/desktop/types/flow.ts equivalent Rust types
// And from current flow_control modules

pub use crate::executor::types::*;
pub use crate::nodes::types::*;
pub use crate::registry::types::*;

// Core error type
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
    #[error("Flow execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Event emission failed: {0}")]
    EventEmissionFailed(String),
    
    #[error("MCP error: {0}")]
    McpError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
```

#### 2.2 Extract Core Traits
```rust
// crates/flow-core/src/lib.rs
pub mod executor;
pub mod nodes;
pub mod registry;
pub mod storage;
pub mod mcp;
pub mod types;

// Re-export main types and traits
pub use executor::{UniversalFlowExecutor, StateProvider};
pub use nodes::{FlowNode, StandardNode};
pub use registry::{FlowRegistry, FlowRegistryProvider};
pub use storage::{FlowStorage, SqliteFlowStorage};
pub use types::*;

#[cfg(feature = "mcp-integration")]
pub use mcp::*;
```

### Phase 3: Extract Executor Logic (Week 2-3)

#### 3.1 Move Core Execution Logic
```bash
# Copy and refactor current executor.rs content
cp apps/tauri-windows/src-tauri/src/flow_control/executor.rs crates/flow-core/src/executor/universal.rs

# Refactor to remove Tauri dependencies
# Replace AppHandle with StateProvider trait
# Replace direct state access with trait methods
```

#### 3.2 Extract AI Executor
```bash
# Copy and refactor AI execution logic
cp apps/tauri-windows/src-tauri/src/flow_control/ai_executor.rs crates/flow-core/src/nodes/ai.rs

# Remove Tauri-specific chat state dependencies
# Abstract MCP client access through traits
```

#### 3.3 Extract Standard Nodes
```bash
# Copy standard node implementations
cp apps/tauri-windows/src-tauri/src/flow_control/standard_nodes.rs crates/flow-core/src/nodes/standard.rs

# Remove system-specific nodes (move to Tauri adapter)
# Keep only platform-agnostic nodes
```

### Phase 4: Create Tauri Adapters (Week 3-4)

#### 4.1 Create Tauri State Provider
```rust
// apps/tauri-windows/src-tauri/src/flow_control/tauri_adapter.rs
// Implement TauriStateProvider as shown above
// Bridge existing Tauri state to core traits
```

#### 4.2 Update Tauri Commands
```rust
// apps/tauri-windows/src-tauri/src/flow_control/commands.rs
use flow_core::{UniversalFlowExecutor, FlowDefinition};
use crate::services::flow::tauri_adapter::TauriStateProvider;

#[tauri::command]
pub async fn execute_flow(
    flow: FlowDefinition,
    state: tauri::State<'_, Arc<TauriStateProvider>>,
) -> Result<String, String> {
    let executor = UniversalFlowExecutor::new(state.inner().clone()).await
        .map_err(|e| e.to_string())?;
    
    let result = executor.execute_flow(flow).await
        .map_err(|e| e.to_string())?;
    
    Ok(serde_json::to_string(&result).map_err(|e| e.to_string())?)
}
```

#### 4.3 Update Tauri State Setup
```rust
// apps/tauri-windows/src-tauri/src/main.rs
use flow_core::*;
use crate::services::flow::tauri_adapter::TauriStateProvider;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Create existing Tauri state
            let flow_registry = Arc::new(FlowRegistryState::new());
            let chat_state = Arc::new(ChatState::new());
            
            // Create core adapter
            let state_provider = Arc::new(TauriStateProvider::new(
                flow_registry,
                chat_state,
                app.handle().clone(),
            ));
            
            // Manage both old and new state during transition
            app.manage(state_provider);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::services::flow::commands::execute_flow,
            // ... other commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Phase 5: Create Headless Implementation (Week 4-5)

#### 5.1 Headless Flow Registry
```rust
// crates/flow-server/src/adapters/headless_registry.rs
use flow_core::registry::FlowRegistryProvider;

pub struct HeadlessFlowRegistry {
    nodes: HashMap<String, Arc<dyn FlowNode>>,
    mcp_clients: HashMap<String, McpClient>,
}

impl HeadlessFlowRegistry {
    pub async fn new(config: &ServerConfig) -> Result<Self, CoreError> {
        let mut registry = Self {
            nodes: HashMap::new(),
            mcp_clients: HashMap::new(),
        };
        
        // Register standard nodes (no system dependencies)
        registry.register_standard_nodes();
        
        // Initialize MCP connections based on config
        registry.initialize_mcp_connections(config).await?;
        
        Ok(registry)
    }
    
    fn register_standard_nodes(&mut self) {
        // Only register platform-agnostic nodes
        self.register_node(Arc::new(DelayNode));
        self.register_node(Arc::new(TextValueNode));
        self.register_node(Arc::new(ConditionalNode));
        // ... other safe nodes
    }
    
    async fn initialize_mcp_connections(&mut self, config: &ServerConfig) -> Result<(), CoreError> {
        for server_config in &config.mcp_servers {
            let client = McpClient::connect(server_config).await?;
            self.mcp_clients.insert(server_config.name.clone(), client);
        }
        Ok(())
    }
}
```

#### 5.2 Headless Server Main
```rust
// crates/flow-server/src/main.rs
use flow_core::UniversalFlowExecutor;
use crate::adapters::HeadlessStateProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();
    
    let config = ServerConfig::from_env()?;
    let state_provider = Arc::new(HeadlessStateProvider::new(config).await?);
    
    // Create API server
    let app = create_api_routes(state_provider).await?;
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Flow server listening on 0.0.0.0:3000");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn create_api_routes(
    state_provider: Arc<HeadlessStateProvider>
) -> Result<Router, CoreError> {
    let executor = Arc::new(UniversalFlowExecutor::new(state_provider).await?);
    
    Ok(Router::new()
        .route("/api/flows", post(deploy_flow))
        .route("/api/flows/:id/execute", post(execute_flow))
        .route("/api/flows/:id/status", get(get_flow_status))
        .with_state(executor))
}
```

### Phase 6: Testing & Validation (Week 5-6)

#### 6.1 Core Unit Tests
```rust
// crates/flow-core/src/executor/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockStateProvider;
    
    #[tokio::test]
    async fn test_basic_flow_execution() {
        let mock_provider = Arc::new(MockStateProvider::new());
        let executor = UniversalFlowExecutor::new(mock_provider).await.unwrap();
        
        let flow = create_test_flow();
        let result = executor.execute_flow(flow).await.unwrap();
        
        assert_eq!(result.status, FlowStatus::Completed);
    }
}
```

#### 6.2 Integration Tests
```rust
// apps/tauri-windows/src-tauri/src/flow_control/integration_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tauri_adapter_compatibility() {
        // Test that Tauri adapter works with existing flows
        let (flow_registry, chat_state, app_handle) = setup_test_tauri_state();
        
        let adapter = TauriStateProvider::new(flow_registry, chat_state, app_handle);
        let executor = UniversalFlowExecutor::new(Arc::new(adapter)).await.unwrap();
        
        // Execute existing test flows
        for test_flow in get_existing_test_flows() {
            let result = executor.execute_flow(test_flow).await;
            assert!(result.is_ok(), "Flow execution should succeed");
        }
    }
}
```

### Phase 7: Gradual Migration (Week 6-8)

#### 7.1 Parallel Operation
```rust
// Run both old and new executors side by side
// Compare results to ensure compatibility

#[tauri::command]
pub async fn execute_flow_with_validation(
    flow: FlowDefinition,
    old_executor: tauri::State<'_, FlowExecutor>,
    new_state: tauri::State<'_, Arc<TauriStateProvider>>,
) -> Result<String, String> {
    // Execute with both systems
    let old_result = old_executor.execute_flow(flow.clone()).await;
    
    let new_executor = UniversalFlowExecutor::new(new_state.inner().clone()).await
        .map_err(|e| e.to_string())?;
    let new_result = new_executor.execute_flow(flow).await;
    
    // Compare results and log differences
    compare_execution_results(&old_result, &new_result).await;
    
    // Return new result, but validate against old
    new_result.map(|r| serde_json::to_string(&r).unwrap())
        .map_err(|e| e.to_string())
}
```

#### 7.2 Feature Flag Migration
```rust
// Use feature flags to gradually switch to new executor
#[tauri::command]
pub async fn execute_flow(
    flow: FlowDefinition,
    use_new_executor: bool,
    // ... both state parameters
) -> Result<String, String> {
    if use_new_executor {
        // Use new core executor
        execute_with_new_executor(flow).await
    } else {
        // Use old executor
        execute_with_old_executor(flow).await
    }
}
```

## 📊 Implementation Timeline

### Week 1: Infrastructure Setup
- [ ] Create workspace structure
- [ ] Setup Cargo.toml files
- [ ] Extract core types to flow-core
- [ ] Create basic trait definitions

### Week 2: Core Extraction
- [ ] Move executor logic to flow-core
- [ ] Extract AI execution logic
- [ ] Move standard nodes (platform-agnostic only)
- [ ] Create basic storage abstraction

### Week 3: Tauri Adapters
- [ ] Implement TauriStateProvider
- [ ] Create Tauri adapter bridges
- [ ] Update Tauri commands to use core
- [ ] Maintain backward compatibility

### Week 4: Headless Implementation
- [ ] Create HeadlessStateProvider
- [ ] Implement headless registry
- [ ] Create basic server API
- [ ] Test headless execution

### Week 5: Testing & Validation
- [ ] Write comprehensive unit tests
- [ ] Create integration tests
- [ ] Performance benchmarking
- [ ] Compatibility validation

### Week 6-7: Gradual Migration
- [ ] Parallel operation testing
- [ ] Feature flag implementation
- [ ] Gradual rollout to users
- [ ] Monitor for issues

### Week 8: Cleanup & Documentation
- [ ] Remove old executor code
- [ ] Update documentation
- [ ] Performance optimization
- [ ] Final testing

## ✅ Success Criteria

### Functional Requirements
- [ ] All existing flows execute correctly with new core
- [ ] Tauri app maintains full functionality
- [ ] Headless server can execute flows independently
- [ ] MCP integration works in both modes
- [ ] System nodes work through bridge agent

### Performance Requirements
- [ ] No performance regression in Tauri app
- [ ] Headless execution within 2x of local performance
- [ ] Memory usage increase < 20%
- [ ] Startup time increase < 1 second

### Quality Requirements
- [ ] 90%+ test coverage for flow-core
- [ ] All integration tests pass
- [ ] Zero breaking changes for existing users
- [ ] Clean separation of concerns
- [ ] Comprehensive documentation

## 🚨 Risk Mitigation

### Technical Risks
1. **State Coupling**: Gradual extraction with adapters
2. **Performance Impact**: Benchmarking and optimization
3. **Breaking Changes**: Parallel operation and validation
4. **MCP Integration**: Careful abstraction and testing

### Mitigation Strategies
1. **Feature Flags**: Allow rollback if issues arise
2. **Comprehensive Testing**: Prevent regressions
3. **Gradual Rollout**: Identify issues early
4. **Documentation**: Ensure smooth transition

This plan ensures a smooth transition from the monolithic Tauri-only architecture to a flexible, multi-platform core that enables both local and cloud deployment! 🚀 