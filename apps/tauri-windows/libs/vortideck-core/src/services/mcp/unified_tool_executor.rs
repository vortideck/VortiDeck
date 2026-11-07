use crate::services::flow::commands::FlowRegistryState;
use crate::services::flow::traits::{AiToolExecutionPolicy, FlowNode};
use crate::services::mcp::ai_node::{ToolAction, ToolSpec, ToolType};
use crate::services::mcp::commands::ChatState;
use crate::services::mcp::tool::{Tool, ToolCallContext};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionResult {
    pub call_id: String,
    pub tool_name: String,
    pub args: Value,
    pub succeeded: bool,
    pub status: ToolExecutionStatus,
    pub result: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    Applied,
    TransportAccepted,
    Partial,
    Skipped,
    Failed,
}

impl ToolExecutionResult {
    fn accepted(action: &ToolAction, result: Value) -> Self {
        let status = match result.get("status").and_then(Value::as_str) {
            Some("transport_accepted") => ToolExecutionStatus::TransportAccepted,
            Some("partial") => ToolExecutionStatus::Partial,
            Some("skipped") => ToolExecutionStatus::Skipped,
            _ => ToolExecutionStatus::Applied,
        };
        Self {
            call_id: action.call_id.clone(),
            tool_name: action.tool_name.clone(),
            args: action.args.clone(),
            succeeded: true,
            status,
            result: Some(result),
            error: None,
        }
    }

    fn failed(action: &ToolAction, error: String) -> Self {
        Self {
            call_id: action.call_id.clone(),
            tool_name: action.tool_name.clone(),
            args: action.args.clone(),
            succeeded: false,
            status: ToolExecutionStatus::Failed,
            result: None,
            error: Some(error),
        }
    }
}

/// Unified tool executor that centralizes all tool execution logic
/// This replaces the fragmented tool execution across AI Assistant, AI Executor, Flow Executor, etc.
pub struct UnifiedToolExecutor {
    flow_registry: FlowRegistryState,
    chat_state: ChatState,
}

impl UnifiedToolExecutor {
    pub fn new(flow_registry: FlowRegistryState, chat_state: ChatState) -> Self {
        Self {
            flow_registry,
            chat_state,
        }
    }

    /// Execute a tool by its specification
    pub async fn execute_tool(
        &self,
        tool_spec: &ToolSpec,
        args: Value,
        context: &ToolCallContext,
    ) -> Result<Value, String> {
        println!(
            "🔧 Unified Tool Executor | Executing: {} (Type: {:?})",
            tool_spec.name, tool_spec.tool_type
        );

        match &tool_spec.tool_type {
            ToolType::SystemTool => self.execute_system_tool(tool_spec, args).await,
            ToolType::McpTool { server_name: _ } => {
                self.execute_mcp_tool(tool_spec, args, context).await
            }
            ToolType::FlowNode => self.execute_flow_node_tool(tool_spec, args).await,
        }
    }

    /// Execute a tool action (contains tool name and args)
    pub async fn execute_tool_action(
        &self,
        action: &ToolAction,
        execution: &crate::services::execution::ExecutionContext,
    ) -> Result<Value, String> {
        // Find the tool spec by name
        let tool_spec = self.find_tool_by_name(&action.tool_name).await?;
        let mut execution = execution.clone();
        execution.execution_config = Some(serde_json::json!({
            "invocationId": action.call_id,
            "idempotencyKey": format!("ai-tool:{}", action.call_id),
        }));
        self.execute_tool(
            &tool_spec,
            action.args.clone(),
            &ToolCallContext {
                call_id: action.call_id.clone(),
                execution,
            },
        )
        .await
    }

    /// Execute multiple tool actions sequentially
    pub async fn execute_tool_actions(
        &self,
        actions: &[ToolAction],
        execution: &crate::services::execution::ExecutionContext,
    ) -> Vec<ToolExecutionResult> {
        let mut results = Vec::with_capacity(actions.len());

        println!(
            "🔧 Unified Tool Executor | Executing {} actions sequentially",
            actions.len()
        );

        for (i, action) in actions.iter().enumerate() {
            let step_num = i + 1;
            let total_steps = actions.len();

            println!(
                "🔄 STEP {}/{}: Executing '{}'",
                step_num, total_steps, action.tool_name
            );

            let step_start = std::time::Instant::now();

            match self.execute_tool_action(action, execution).await {
                Ok(result) => {
                    let duration = step_start.elapsed();
                    println!(
                        "   ✅ STEP {}/{} COMPLETED in {:?}",
                        step_num, total_steps, duration
                    );
                    results.push(ToolExecutionResult::accepted(action, result));

                    // Small delay between steps for proper sequencing
                    if step_num < total_steps {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
                Err(e) => {
                    let duration = step_start.elapsed();
                    println!(
                        "   ❌ STEP {}/{} FAILED in {:?}: {}",
                        step_num, total_steps, duration, e
                    );
                    results.push(ToolExecutionResult::failed(action, e));
                }
            }
        }

        results
    }

    /// Get all available tools (for Free Agent mode)
    pub async fn get_all_available_tools(&self) -> Result<Vec<ToolSpec>, String> {
        let mut tools = Vec::new();

        // Add all system tools
        let system_tools = self.get_all_system_tools().await?;
        tools.extend(system_tools);

        // Add all MCP tools
        let mcp_tools = self.get_all_mcp_tools().await?;
        tools.extend(mcp_tools);

        Ok(tools)
    }

    /// Find a tool by name across all tool types
    pub async fn find_tool_by_name(&self, tool_name: &str) -> Result<ToolSpec, String> {
        // Try system tools first
        if let Ok(tool_spec) = self.find_system_tool_by_name(tool_name).await {
            return Ok(tool_spec);
        }

        // Try MCP tools
        if let Ok(tool_spec) = self.find_mcp_tool_by_name(tool_name).await {
            return Ok(tool_spec);
        }

        Err(format!("Tool '{}' not found in any registry", tool_name))
    }

    // ============================================================================
    // PRIVATE IMPLEMENTATION METHODS
    // ============================================================================

    /// Execute an explicitly pure system tool from the flow registry.
    async fn execute_system_tool(
        &self,
        tool_spec: &ToolSpec,
        args: Value,
    ) -> Result<Value, String> {
        println!(
            "⚡ Executing FlowNode: {} (ID: {})",
            tool_spec.name, tool_spec.node_id
        );

        let execution_start = std::time::Instant::now();

        // Get the node first (drop the lock before await)
        let node = {
            let registry = self
                .flow_registry
                .registry
                .lock()
                .map_err(|e| e.to_string())?;
            registry
                .get_node(&tool_spec.node_id)
                .ok_or_else(|| format!("FlowNode '{}' not found in registry", tool_spec.node_id))?
        }; // Lock dropped here

        if node.ai_tool_execution_policy() != AiToolExecutionPolicy::PureComputation {
            return Err(format!(
                "Flow node '{}' is not approved for direct AI execution; expose an invocation-backed tool instead",
                tool_spec.node_id
            ));
        }

        // Raw node execution is intentionally limited to implementations that
        // opt into the pure-computation contract above. Effects use MCP tools
        // backed by ActionInvocationService instead.
        let result = node.execute(args).await?;

        let execution_time = execution_start.elapsed();

        println!(
            "🎯 FlowNode '{}' completed in {:?}",
            tool_spec.name, execution_time
        );
        Ok(result)
    }

    /// Execute an MCP tool
    async fn execute_mcp_tool(
        &self,
        tool_spec: &ToolSpec,
        args: Value,
        context: &ToolCallContext,
    ) -> Result<Value, String> {
        println!("⚡ Executing MCP tool: {}", tool_spec.name);

        let execution_start = std::time::Instant::now();
        let tool = {
            let tools = self.chat_state.mcp_tools.lock().await;
            tools
                .get_tool(&tool_spec.name)
                .ok_or_else(|| format!("MCP tool '{}' is not registered", tool_spec.name))?
        };
        let result = tool
            .call_with_context(args, context)
            .await
            .map_err(|error| format!("MCP tool '{}' failed: {error}", tool_spec.name))?;
        let result = serde_json::from_str(&result).unwrap_or(Value::String(result));
        let execution_time = execution_start.elapsed();

        println!(
            "🎯 MCP tool '{}' completed in {:?}",
            tool_spec.name, execution_time
        );
        Ok(result)
    }

    /// Execute a flow node as a tool
    async fn execute_flow_node_tool(
        &self,
        tool_spec: &ToolSpec,
        args: Value,
    ) -> Result<Value, String> {
        println!(
            "⚡ Executing flow node tool: {} (Node ID: {})",
            tool_spec.name, tool_spec.node_id
        );

        self.execute_system_tool(tool_spec, args).await
    }

    /// Get all system tools from the registry
    async fn get_all_system_tools(&self) -> Result<Vec<ToolSpec>, String> {
        let registry = self
            .flow_registry
            .registry
            .lock()
            .map_err(|e| e.to_string())?;
        let all_node_info = registry.get_all_node_info();

        let mut tools = Vec::new();
        for tool_info in &all_node_info {
            if let Some(system_node) = registry.get_node_by_name(&tool_info.name) {
                if system_node.ai_tool_execution_policy() != AiToolExecutionPolicy::PureComputation
                {
                    continue;
                }
                let actual_node_id = system_node.id();

                tools.push(ToolSpec {
                    name: actual_node_id.clone(),
                    description: format!("{} | {}", tool_info.name, system_node.description()),
                    parameters: system_node.parameters_schema(),
                    node_id: actual_node_id,
                    tool_type: ToolType::SystemTool,
                });
            }
        }

        Ok(tools)
    }

    /// Get all MCP tools from the global tool set
    async fn get_all_mcp_tools(&self) -> Result<Vec<ToolSpec>, String> {
        let tools_guard = self.chat_state.mcp_tools.lock().await;
        let mcp_tools = tools_guard.tools();

        let mut tools = Vec::new();
        for tool in mcp_tools {
            tools.push(ToolSpec {
                name: tool.name(),
                description: tool.description(),
                parameters: tool.parameters(),
                node_id: tool.name(),
                tool_type: ToolType::McpTool {
                    server_name: "default".to_string(),
                },
            });
        }

        Ok(tools)
    }

    /// Find a system tool by name
    async fn find_system_tool_by_name(&self, tool_name: &str) -> Result<ToolSpec, String> {
        let registry = self
            .flow_registry
            .registry
            .lock()
            .map_err(|e| e.to_string())?;

        // Try by exact ID first
        if let Some(system_node) = registry.get_node(tool_name).filter(|node| {
            node.ai_tool_execution_policy() == AiToolExecutionPolicy::PureComputation
        }) {
            return Ok(ToolSpec {
                name: tool_name.to_string(),
                description: system_node.description(),
                parameters: system_node.parameters_schema(),
                node_id: tool_name.to_string(),
                tool_type: ToolType::SystemTool,
            });
        }

        // Try by display name
        if let Some(system_node) = registry.get_node_by_name(tool_name).filter(|node| {
            node.ai_tool_execution_policy() == AiToolExecutionPolicy::PureComputation
        }) {
            let actual_id = system_node.id();
            return Ok(ToolSpec {
                name: actual_id.clone(),
                description: system_node.description(),
                parameters: system_node.parameters_schema(),
                node_id: actual_id,
                tool_type: ToolType::SystemTool,
            });
        }

        Err(format!("System tool '{}' not found", tool_name))
    }

    /// Find an MCP tool by name
    async fn find_mcp_tool_by_name(&self, tool_name: &str) -> Result<ToolSpec, String> {
        let tools_guard = self.chat_state.mcp_tools.lock().await;
        if let Some(tool) = tools_guard.get_tool(tool_name) {
            return Ok(ToolSpec {
                name: tool.name(),
                description: tool.description(),
                parameters: tool.parameters(),
                node_id: tool.name(),
                tool_type: ToolType::McpTool {
                    server_name: "default".to_string(),
                },
            });
        }

        Err(format!("MCP tool '{}' not found", tool_name))
    }
}

#[cfg(test)]
mod result_contract_tests {
    use super::*;

    fn action(call_id: &str, value: i64) -> ToolAction {
        ToolAction {
            call_id: call_id.to_string(),
            tool_name: "repeat.tool".to_string(),
            args: serde_json::json!({"value": value}),
            target_node_id: "repeat.tool".to_string(),
        }
    }

    #[test]
    fn same_name_results_remain_ordered_and_failure_is_not_applied() {
        let first = action("call-a", 1);
        let second = action("call-b", 2);
        let results = vec![
            ToolExecutionResult::accepted(&first, serde_json::json!({"seen": 1})),
            ToolExecutionResult::failed(&second, "permission revoked".to_string()),
        ];

        assert_eq!(results[0].call_id, "call-a");
        assert_eq!(results[0].args, serde_json::json!({"value": 1}));
        assert!(results[0].succeeded);
        assert_eq!(results[0].status, ToolExecutionStatus::Applied);
        assert_eq!(results[1].call_id, "call-b");
        assert_eq!(results[1].args, serde_json::json!({"value": 2}));
        assert!(!results[1].succeeded);
        assert_eq!(results[1].status, ToolExecutionStatus::Failed);
        assert!(results[1].result.is_none());
        assert_eq!(results[1].error.as_deref(), Some("permission revoked"));
    }

    #[test]
    fn accepted_transport_receipt_is_not_reported_as_applied() {
        let action = action("call-accepted", 1);
        let result = ToolExecutionResult::accepted(
            &action,
            serde_json::json!({"status": "transport_accepted"}),
        );

        assert!(result.succeeded);
        assert_eq!(result.status, ToolExecutionStatus::TransportAccepted);
    }
}

impl Clone for UnifiedToolExecutor {
    fn clone(&self) -> Self {
        Self {
            flow_registry: self.flow_registry.clone(),
            chat_state: self.chat_state.clone(),
        }
    }
}
