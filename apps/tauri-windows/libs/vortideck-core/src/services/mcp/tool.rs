use crate::services::mcp::error::McpError;
use crate::services::mcp::model::{Content, ToolCall, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, ClientRequest, Request, ServerResult,
        Tool as McpTool,
    },
    service::{PeerRequestOptions, RunningService, ServerSink},
    RoleClient,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ToolCallContext {
    pub call_id: String,
    pub execution: crate::services::execution::ExecutionContext,
}

tokio::task_local! {
    static CURRENT_TOOL_EXECUTION_CONTEXT: crate::services::execution::ExecutionContext;
}

pub fn current_tool_execution_context() -> Option<crate::services::execution::ExecutionContext> {
    CURRENT_TOOL_EXECUTION_CONTEXT.try_with(Clone::clone).ok()
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn parameters(&self) -> Value;

    /// Optional output schema for tools that define their output structure
    /// Most tools can use the default implementation which returns None
    fn output_schema(&self) -> Option<Value> {
        None
    }

    async fn call(&self, args: Value) -> Result<String>;

    /// Context-aware entry used by workflow/assistant execution. Pure and
    /// remote MCP tools may use the default; VortiDeck effect tools override it
    /// to pass the real principal and invocation identity to the durable layer.
    async fn call_with_context(&self, args: Value, _context: &ToolCallContext) -> Result<String> {
        CURRENT_TOOL_EXECUTION_CONTEXT
            .scope(_context.execution.clone(), self.call(args))
            .await
    }
}
#[derive(Clone)]
pub struct McpToolAdapter {
    tool: McpTool,
    server: ServerSink,
}

impl McpToolAdapter {
    pub fn new(tool: McpTool, server: ServerSink) -> Self {
        Self { tool, server }
    }

    fn call_params(&self, args: Value) -> CallToolRequestParams {
        let mut params = CallToolRequestParams::new(self.tool.name.clone());
        if let Value::Object(map) = args {
            params = params.with_arguments(map);
        }
        params
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> String {
        self.tool.name.clone().to_string()
    }

    fn description(&self) -> String {
        self.tool
            .description
            .clone()
            .unwrap_or_default()
            .to_string()
    }

    fn parameters(&self) -> Value {
        serde_json::to_value(&self.tool.input_schema).unwrap_or(serde_json::json!({}))
    }

    fn output_schema(&self) -> Option<Value> {
        self.tool
            .output_schema
            .as_ref()
            .map(|schema| serde_json::to_value(&**schema).unwrap_or(serde_json::json!({})))
    }

    async fn call(&self, args: Value) -> Result<String> {
        let call_result = self.server.call_tool(self.call_params(args)).await?;
        let result = serde_json::to_string(&call_result).unwrap();

        Ok(result)
    }

    async fn call_with_context(&self, args: Value, context: &ToolCallContext) -> Result<String> {
        let Some(timeout_ms) = context.execution.timeout_ms else {
            return self.call(args).await;
        };
        let request = ClientRequest::CallToolRequest(Request::new(self.call_params(args)));
        let response = self
            .server
            .send_cancellable_request(
                request,
                PeerRequestOptions::with_timeout(std::time::Duration::from_millis(timeout_ms)),
            )
            .await?
            .await_response()
            .await?;
        match response {
            ServerResult::CallToolResult(result) => Ok(serde_json::to_string(&result)?),
            ServerResult::InputRequiredResult(_) => anyhow::bail!(
                "MCP tool '{}' requested interactive input during a bounded workflow call",
                self.tool.name
            ),
            ServerResult::CreateTaskResult(_) => anyhow::bail!(
                "MCP tool '{}' returned an asynchronous task that this workflow path does not support",
                self.tool.name
            ),
            _ => anyhow::bail!("MCP tool '{}' returned an unexpected response", self.tool.name),
        }
    }
}

#[derive(Clone)]
pub struct ToolSet {
    tools: Vec<Arc<dyn Tool>>,
}

impl std::fmt::Debug for ToolSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolSet")
            .field("tools", &format_args!("{} tools", self.tools.len()))
            .finish()
    }
}

impl ToolSet {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Creates a ToolSet pre-populated with native VortiDeck tools
    pub fn with_native_tools(
        bus: Option<std::sync::Arc<crate::events::bus::EventBus>>,
        invocation_service: Option<
            std::sync::Arc<crate::services::execution::ActionInvocationService>,
        >,
        flow_executor: Option<crate::services::mcp::tools::SharedFlowExecutor>,
    ) -> Self {
        let mut toolset = Self::new();
        crate::services::mcp::tools::register_vortideck_native_tools(
            &mut toolset,
            bus,
            invocation_service,
            flow_executor,
        );
        toolset
    }

    pub fn add_tool<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.push(Arc::new(tool));
    }

    pub fn add_tool_arc(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn remove_tool(&mut self, name: &str) {
        self.tools.retain(|t| t.name() != name);
    }

    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolSet {
    fn default() -> Self {
        Self::with_native_tools(None, None, None)
    }
}

pub async fn get_mcp_tools(server: ServerSink) -> Result<Vec<McpToolAdapter>> {
    let tools = server.list_all_tools().await?;
    Ok(tools
        .into_iter()
        .map(|tool| McpToolAdapter::new(tool, server.clone()))
        .collect())
}

pub trait IntoCallToolResult {
    fn into_call_tool_result(self) -> Result<ToolResult, McpError>;
}

impl<T> IntoCallToolResult for Result<T, McpError>
where
    T: serde::Serialize,
{
    fn into_call_tool_result(self) -> Result<ToolResult, McpError> {
        match self {
            Ok(response) => {
                let content = Content {
                    content_type: "application/json".to_string(),
                    body: serde_json::to_string(&response).unwrap_or_default(),
                };
                Ok(ToolResult {
                    success: true,
                    contents: vec![content],
                })
            }
            Err(error) => {
                let content = Content {
                    content_type: "application/json".to_string(),
                    body: serde_json::to_string(&error).unwrap_or_default(),
                };
                Ok(ToolResult {
                    success: false,
                    contents: vec![content],
                })
            }
        }
    }
}

// ... existing code ...

#[cfg(test)]
mod context_tests {
    use super::*;
    use crate::services::execution::{
        ButtonExecutionMode, ExecutionContext, RetryConfig, TriggerSource,
    };

    struct ContextProbe;

    #[async_trait]
    impl Tool for ContextProbe {
        fn name(&self) -> String {
            "context_probe".to_string()
        }
        fn description(&self) -> String {
            "fixture".to_string()
        }
        fn parameters(&self) -> Value {
            serde_json::json!({})
        }

        async fn call(&self, _args: Value) -> Result<String> {
            let context = current_tool_execution_context().expect("scoped execution context");
            Ok(serde_json::to_string(&context.trigger_source)?)
        }
    }

    #[tokio::test]
    async fn contextual_call_scopes_trusted_principal_without_json_spoofing() {
        let execution = ExecutionContext {
            trigger_source: TriggerSource::Flow {
                flow_id: "run-1".to_string(),
                node_id: "ai-1".to_string(),
            },
            user_id: None,
            session_id: "flow-ai:run-1:ai-1".to_string(),
            variables: std::collections::HashMap::new(),
            timeout_ms: Some(30_000),
            retry_config: RetryConfig {
                max_retries: 0,
                retry_delay_ms: 0,
                exponential_backoff: false,
            },
            execution_mode: ButtonExecutionMode::RunOnce,
            execution_config: Some(serde_json::json!({"invocationId": "call-1"})),
        };

        let result = ContextProbe
            .call_with_context(
                serde_json::json!({"attemptedPrincipal": "plugin:spoofed"}),
                &ToolCallContext {
                    call_id: "call-1".to_string(),
                    execution,
                },
            )
            .await
            .expect("context probe");

        assert!(result.contains("run-1"));
        assert!(result.contains("ai-1"));
        assert!(!result.contains("spoofed"));
        assert!(current_tool_execution_context().is_none());
    }
}
