// 🚀 UNIVERSAL ARCHITECTURE: Core MCP business logic without platform dependencies
// This module provides platform-agnostic MCP functionality

use crate::services::mcp::client_pool::ClientPool;
use crate::services::mcp::config::Config;
use crate::services::mcp::session_manager::SessionManager;
use crate::services::mcp::tool::ToolSet;
use anyhow::Result;
use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

// Advanced ChatState structure for the core library (suitable for headless usage)
#[derive(Debug, Clone)]
pub struct ChatState {
    /// Session manager for conversation and ephemeral sessions (replaces single global session)
    pub session_manager: SessionManager,
    pub config: Arc<TokioMutex<Option<Config>>>,
    pub mcp_clients: Arc<
        TokioMutex<
            HashMap<
                String,
                RunningService<RoleClient, crate::services::mcp::handler::VortiDeckClientHandler>,
            >,
        >,
    >,
    /// Global MCP tools loaded at startup (shared across all sessions)
    pub mcp_tools: Arc<TokioMutex<ToolSet>>,
    /// Track which tools belong to which server (server_name -> tool_names)
    pub server_tools: Arc<TokioMutex<HashMap<String, Vec<String>>>>,
}

impl ChatState {
    pub fn new() -> Self {
        Self::new_with_flow_executor(None)
    }

    pub fn new_with_flow_executor(
        flow_executor: Option<crate::services::mcp::tools::SharedFlowExecutor>,
    ) -> Self {
        let mcp_tools = Arc::new(TokioMutex::new(ToolSet::with_native_tools(
            None,
            None,
            flow_executor,
        )));
        let client_pool = ClientPool::new();
        let session_manager = SessionManager::new(client_pool, Arc::clone(&mcp_tools));

        Self {
            session_manager,
            config: Arc::new(TokioMutex::new(None)),
            mcp_clients: Arc::new(TokioMutex::new(HashMap::new())),
            mcp_tools,
            server_tools: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }

    /// Atomically replace the tools owned by one outbound MCP connection.
    ///
    /// Raw MCP tool names remain stable for existing flows, so ambiguous names
    /// are rejected instead of being silently shadowed across native tools or
    /// two remote servers.
    pub async fn replace_server_tools<T>(
        &self,
        server_name: &str,
        tools: &[T],
    ) -> Result<usize, String>
    where
        T: Tool + Clone + 'static,
    {
        let incoming_names: Vec<String> = tools.iter().map(Tool::name).collect();
        let unique_names: std::collections::HashSet<&str> =
            incoming_names.iter().map(String::as_str).collect();
        if unique_names.len() != incoming_names.len() {
            return Err(format!(
                "MCP server '{server_name}' advertised duplicate tool names"
            ));
        }
        let mut owners = self.server_tools.lock().await;
        let current_names = owners.get(server_name).cloned().unwrap_or_default();
        let mut registry = self.mcp_tools.lock().await;

        for name in &incoming_names {
            if let Some((owner, _)) = owners
                .iter()
                .find(|(owner, names)| owner.as_str() != server_name && names.contains(name))
            {
                return Err(format!(
                    "MCP tool '{name}' from '{server_name}' conflicts with server '{owner}'"
                ));
            }
            if !current_names.contains(name) && registry.get_tool(name).is_some() {
                return Err(format!(
                    "MCP tool '{name}' from '{server_name}' conflicts with a native VortiDeck tool"
                ));
            }
        }

        for name in &current_names {
            registry.remove_tool(name);
        }
        for tool in tools {
            registry.add_tool(tool.clone());
        }
        owners.insert(server_name.to_string(), incoming_names);
        Ok(tools.len())
    }

    /// Remove only tools owned by the named outbound MCP connection.
    pub async fn remove_server_tools(&self, server_name: &str) -> usize {
        let mut owners = self.server_tools.lock().await;
        let names = owners.remove(server_name).unwrap_or_default();
        let mut registry = self.mcp_tools.lock().await;
        for name in &names {
            registry.remove_tool(name);
        }
        names.len()
    }
}

// Re-export core types
pub use crate::services::mcp::{
    ai_node::{AiNodeConfig, AiNodeInput, AiNodeOutput, ToolSpec},
    model::*,
    tool::Tool,
};

// Core business logic functions that don't require Tauri

/// Core MCP config loading logic
pub async fn load_mcp_config(
    config_path: &std::path::Path,
) -> Result<crate::services::mcp::config::Config, String> {
    crate::services::mcp::config::Config::load(config_path)
        .await
        .map_err(|e| format!("Failed to load config: {}", e))
}

/// Core tool execution logic for flow tool nodes. Calls always carry a
/// non-interactive Flow trigger so interactive-only tools (overlay authoring)
/// reject them and effect tools never label them as manual user actions.
pub async fn execute_mcp_tool_core(
    chat_state: &ChatState,
    tool_name: String,
    args: Value,
) -> Result<Value, String> {
    let tool = {
        let tools = chat_state.mcp_tools.lock().await;
        tools
            .get_tool(&tool_name)
            .ok_or_else(|| format!("MCP tool '{tool_name}' is not registered"))?
    };
    let call_id = uuid::Uuid::new_v4().to_string();
    let context = crate::services::mcp::tool::ToolCallContext {
        call_id: call_id.clone(),
        execution: crate::services::execution::ExecutionContext {
            trigger_source: crate::services::execution::TriggerSource::Flow {
                flow_id: "flow-tool-node".to_string(),
                node_id: tool_name.clone(),
            },
            user_id: None,
            session_id: format!("flow-tool:{tool_name}:{call_id}"),
            variables: HashMap::new(),
            timeout_ms: None,
            retry_config: crate::services::execution::RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: crate::services::execution::ButtonExecutionMode::RunOnce,
            execution_config: None,
        },
    };
    let result = tool
        .call_with_context(args, &context)
        .await
        .map_err(|error| format!("MCP tool '{tool_name}' failed: {error}"))?;
    Ok(serde_json::from_str(&result).unwrap_or(Value::String(result)))
}

/// Core AI node execution logic
pub async fn execute_ai_node_core(
    _config: AiNodeConfig,
    _input: AiNodeInput,
) -> Result<AiNodeOutput, String> {
    Err("AI node execution requires the provider-backed AiExecutor".to_string())
}

// System tool adapter for flow integration
pub struct SystemToolAdapter {
    name: String,
    description: String,
    parameters: Value,
    node: Arc<dyn crate::services::flow::traits::FlowNode>,
}

impl SystemToolAdapter {
    pub fn new(
        name: String,
        description: String,
        parameters: Value,
        node: Arc<dyn crate::services::flow::traits::FlowNode>,
    ) -> Self {
        Self {
            name,
            description,
            parameters,
            node,
        }
    }
}

#[async_trait::async_trait]
impl Tool for SystemToolAdapter {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    async fn call(&self, args: Value) -> anyhow::Result<String> {
        let result = self.node.execute(args).await.map_err(anyhow::Error::msg)?;
        serde_json::to_string(&result).map_err(Into::into)
    }
}

/// Schema-only tool exposed to an LLM while execution remains owned by
/// `UnifiedToolExecutor`. Calling it directly is always an error.
pub struct ToolSchemaAdapter {
    name: String,
    description: String,
    parameters: Value,
}

impl ToolSchemaAdapter {
    pub fn new(name: String, description: String, parameters: Value) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

#[async_trait::async_trait]
impl Tool for ToolSchemaAdapter {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    fn output_schema(&self) -> Option<Value> {
        None
    }

    async fn call(&self, _args: Value) -> anyhow::Result<String> {
        anyhow::bail!(
            "Tool '{}' is schema-only; execute it through UnifiedToolExecutor",
            self.name
        )
    }
}

// ✅ CLEAN ARCHITECTURE COMPLETE: Platform-agnostic core module
// All Tauri-specific commands moved to adapters::mcp_commands
// Core library now contains only pure business logic

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::flow::standard_nodes::DelayNode;

    #[derive(Clone)]
    struct NamedTool(&'static str);

    #[async_trait::async_trait]
    impl Tool for NamedTool {
        fn name(&self) -> String {
            self.0.to_string()
        }

        fn description(&self) -> String {
            "MCP registry fixture".to_string()
        }

        fn parameters(&self) -> Value {
            serde_json::json!({ "type": "object" })
        }

        async fn call(&self, _args: Value) -> anyhow::Result<String> {
            Ok(self.0.to_string())
        }
    }

    #[tokio::test]
    async fn core_dispatch_executes_the_registered_tool() {
        let result = execute_mcp_tool_core(
            &ChatState::new(),
            "calculator".to_string(),
            serde_json::json!({ "operation": "multiply", "a": 6, "b": 7 }),
        )
        .await
        .expect("calculator must execute through the registry");

        assert_eq!(result["result"], 42.0);
    }

    #[tokio::test]
    async fn core_dispatch_rejects_an_unregistered_tool() {
        let error = execute_mcp_tool_core(
            &ChatState::new(),
            "not-a-real-tool".to_string(),
            serde_json::json!({}),
        )
        .await
        .expect_err("missing tools must not report success");

        assert!(error.contains("not registered"));
    }

    struct UnreachableOverlaySettings;

    #[async_trait::async_trait]
    impl crate::services::mcp::overlay_settings::OverlayWidgetSettingsApi
        for UnreachableOverlaySettings
    {
        async fn list(&self) -> anyhow::Result<Value> {
            panic!("flow calls must be rejected before overlay access")
        }
        async fn inspect(&self, _element_id: &str) -> anyhow::Result<Value> {
            panic!("flow calls must be rejected before overlay access")
        }
        async fn configure(
            &self,
            _element_id: &str,
            _config: Value,
            _expected_revision: u64,
        ) -> anyhow::Result<Value> {
            panic!("flow calls must be rejected before overlay access")
        }
    }

    #[tokio::test]
    async fn flow_tool_node_cannot_use_interactive_overlay_authoring() {
        let state = ChatState::new();
        state.mcp_tools.lock().await.add_tool(
            crate::services::mcp::overlay_settings::InspectOverlayWidgets(Arc::new(
                UnreachableOverlaySettings,
            )),
        );
        let error = execute_mcp_tool_core(
            &state,
            "vortideck_inspect_overlay_widgets".to_string(),
            serde_json::json!({}),
        )
        .await
        .expect_err("flow-originated authoring must be rejected");
        assert!(error.contains("interactive assistant"), "{error}");
    }

    #[tokio::test]
    async fn system_adapter_executes_its_real_flow_node() {
        let adapter = SystemToolAdapter::new(
            "delay".to_string(),
            "Delay".to_string(),
            serde_json::json!({}),
            Arc::new(DelayNode),
        );

        let result: Value = serde_json::from_str(
            &adapter
                .call(serde_json::json!({ "duration": 1 }))
                .await
                .expect("adapter must call the underlying node"),
        )
        .expect("node output must remain JSON");
        assert_eq!(result["success"], true);
        assert_eq!(result["duration"], 1);
    }

    #[tokio::test]
    async fn schema_adapter_never_fabricates_execution() {
        let adapter = ToolSchemaAdapter::new(
            "preview".to_string(),
            "schema only".to_string(),
            serde_json::json!({}),
        );

        let error = adapter
            .call(serde_json::json!({}))
            .await
            .expect_err("schema-only adapters must reject execution");
        assert!(error.to_string().contains("schema-only"));
    }

    #[tokio::test]
    async fn server_tool_replacement_is_owned_and_atomic() {
        let state = ChatState::new();
        state
            .replace_server_tools("alpha", &[NamedTool("remote_alpha")])
            .await
            .expect("first server tools");

        let conflict = state
            .replace_server_tools("beta", &[NamedTool("remote_alpha")])
            .await
            .expect_err("another server cannot shadow the same raw name");
        assert!(conflict.contains("conflicts with server 'alpha'"));
        assert!(state
            .mcp_tools
            .lock()
            .await
            .get_tool("remote_alpha")
            .is_some());

        state
            .replace_server_tools("alpha", &[NamedTool("remote_alpha_v2")])
            .await
            .expect("same owner may replace its schema");
        let registry = state.mcp_tools.lock().await;
        assert!(registry.get_tool("remote_alpha").is_none());
        assert!(registry.get_tool("remote_alpha_v2").is_some());
        drop(registry);

        assert_eq!(state.remove_server_tools("alpha").await, 1);
        assert!(state
            .mcp_tools
            .lock()
            .await
            .get_tool("remote_alpha_v2")
            .is_none());
    }
}
