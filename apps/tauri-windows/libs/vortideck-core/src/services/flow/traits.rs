use crate::services::flow::executor::RuntimeVariables;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

// ============================================================================
// HELPER FUNCTIONS (standalone to keep FlowNode dyn-compatible)
// ============================================================================

/// Create the standard "result" output that all nodes include
pub(crate) fn default_result_output() -> NodeOutput {
    NodeOutput {
        id: "result".to_owned(),
        name: "result".to_owned(),
        output_type: "object".to_owned(),
        description: Some("Complete node execution result".to_owned()),
        is_dynamic_type: None,
    }
}

/// Parse JSON Schema "properties" into NodeOutputs
/// Used by MCP tools to convert their output_schema into Vec<NodeOutput>
pub fn parse_output_schema_to_outputs(schema: &Value) -> Vec<NodeOutput> {
    // Pre-allocate capacity if we have properties
    let properties = match schema.get("properties").and_then(|v| v.as_object()) {
        Some(props) => props,
        None => {
            // No properties - return just the default result
            return vec![default_result_output()];
        }
    };

    // Reserve space: properties.len() + 1 for the "result" output
    let mut outputs = Vec::with_capacity(properties.len() + 1);

    for (field_name, field_schema) in properties {
        // Skip "result" field - we add it as fallback at the end
        if field_name == "result" {
            continue;
        }

        let field_type = field_schema
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("any")
            .to_owned();

        let description = field_schema
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_owned);

        outputs.push(NodeOutput {
            id: field_name.to_owned(),
            name: field_name.to_owned(),
            output_type: field_type,
            description,
            is_dynamic_type: None,
        });
    }

    // Always include "result" as catch-all for full response
    outputs.push(default_result_output());

    outputs
}

/// Convert Vec<NodeOutput> back to JSON Schema
/// Useful for exposing VortiDeck nodes as MCP tools in the future
pub fn outputs_to_json_schema(outputs: &[NodeOutput]) -> Value {
    // Pre-allocate capacity (estimate: outputs.len() - 1 for "result" field we skip)
    let capacity = outputs.len().saturating_sub(1);
    let mut properties = serde_json::Map::with_capacity(capacity);

    for output in outputs.iter().filter(|o| o.id != "result") {
        let mut field_schema = serde_json::Map::new();
        field_schema.insert("type".to_owned(), Value::String(output.output_type.clone()));

        if let Some(ref desc) = output.description {
            field_schema.insert("description".to_owned(), Value::String(desc.clone()));
        }

        properties.insert(output.id.clone(), Value::Object(field_schema));
    }

    serde_json::json!({
        "type": "object",
        "properties": properties
    })
}

// ============================================================================
// EXECUTION CONTEXT - Shared context passed to nodes during execution
// ============================================================================

/// Progress callback for nodes to report their progress
pub type ProgressCallback = Arc<dyn Fn(f64, String) + Send + Sync>;

/// Whether a registry node may be called directly as an AI planning tool.
/// Unknown and effectful nodes are denied by default; external effects must use
/// an authoritative invocation-backed tool facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiToolExecutionPolicy {
    DenyDirectExecution,
    PureComputation,
}

/// Context available to nodes during execution
/// This enables nodes to access runtime data without hardcoding in the executor
pub struct ExecutionContext<'a> {
    /// Flow ID being executed
    pub flow_id: &'a str,

    /// Current node ID being executed
    pub node_id: &'a str,

    /// Mutable access to runtime variables
    pub variables: &'a mut RuntimeVariables,

    /// Optional progress callback for long-running operations
    /// Nodes can call this to emit progress updates: callback(progress_percent, message)
    pub progress_callback: Option<&'a ProgressCallback>,
}

/// Definition of a node output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    pub id: String,
    pub name: String,
    pub output_type: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dynamic_type: Option<bool>, // If Some(true), this output's type adapts at runtime
}

impl Default for NodeOutput {
    fn default() -> Self {
        NodeOutput {
            id: String::new(),
            name: String::new(),
            output_type: "any".to_string(),
            description: None,
            is_dynamic_type: None,
        }
    }
}

/// Common trait for all tools and actions that can be used in the flow builder
#[async_trait]
pub trait FlowNode: Send + Sync {
    fn ai_tool_execution_policy(&self) -> AiToolExecutionPolicy {
        AiToolExecutionPolicy::DenyDirectExecution
    }

    /// Unique identifier for this node
    fn id(&self) -> String;

    /// Display name shown in the UI
    fn name(&self) -> String;

    /// Description explaining what this node does
    fn description(&self) -> String;

    /// Category for grouping in the UI sidebar (e.g., "SYSTEM", "AUDIO", etc.)
    fn category(&self) -> String;

    /// Custom display name for the category (optional)
    fn category_display_name(&self) -> Option<String> {
        None // Default implementation returns None
    }

    /// Optional second-level discovery group. This is presentation metadata;
    /// it never participates in execution or node identity.
    fn subcategory(&self) -> Option<String> {
        None
    }

    /// JSON schema for the input parameters (matching the expected frontend format)
    fn parameters_schema(&self) -> Value;

    /// Color for the node header in the UI
    fn type_color(&self) -> String;

    /// Optional JSON Schema for outputs (primarily for MCP tools)
    ///
    /// **When to use**:
    /// - MCP tools that provide output_schema from the MCP server
    /// - External tools where schema is dynamically loaded
    ///
    /// **When NOT to use**:
    /// - Predefined nodes (override outputs_schema() directly instead - faster and cleaner)
    ///
    /// If both output_schema() and outputs_schema() are defined, outputs_schema() takes precedence.
    ///
    /// Example (MCP tools):
    /// ```ignore
    /// fn output_schema(&self) -> Option<Value> {
    ///     self.mcp_tool.output_schema()  // From MCP server
    /// }
    /// ```
    fn output_schema(&self) -> Option<Value> {
        None // Default: no schema
    }

    /// Define the actual outputs this node produces
    ///
    /// **For predefined nodes**: Override this directly with Vec<NodeOutput> (recommended)
    /// **For MCP tools**: Define output_schema() instead, this will auto-parse it
    ///
    /// Example (predefined node):
    /// ```ignore
    /// fn outputs_schema(&self) -> Vec<NodeOutput> {
    ///     vec![
    ///         NodeOutput { id: "status", output_type: "string", ... },
    ///         NodeOutput { id: "count", output_type: "number", ... },
    ///     ]
    /// }
    /// ```
    fn outputs_schema(&self) -> Vec<NodeOutput> {
        // If node defines output_schema, parse it using the standalone helper
        if let Some(schema) = self.output_schema() {
            return parse_output_schema_to_outputs(&schema);
        }

        // Otherwise use default single "result" output
        vec![default_result_output()]
    }

    /// Execute the node with the given parameters
    async fn execute(&self, params: Value) -> Result<Value, String>;

    /// Execute the node with access to runtime context
    ///
    /// Override this method when your node needs access to:
    /// - Runtime variables (get/set)
    /// - Flow ID or Node ID
    /// - Any future context data
    ///
    /// Default implementation delegates to execute() for backward compatibility.
    /// Most nodes don't need to override this - only override if you need context access.
    async fn execute_with_context(
        &self,
        params: Value,
        _ctx: &mut ExecutionContext<'_>,
    ) -> Result<Value, String> {
        // Default: delegate to regular execute (backward compatible)
        self.execute(params).await
    }
}

/// Trait for registering flow nodes from a specific module
pub trait FlowNodeProvider: Send + Sync {
    /// Get all flow nodes provided by this module
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>>;

    /// Get the unique name of this provider for tracking
    /// This is used to identify and clear provider nodes during hot reloading
    fn provider_name(&self) -> &'static str {
        "UnknownProvider"
    }
}

// ============================================================================
// TRIGGER NODE LIFECYCLE TRAIT
// ============================================================================

use crate::services::flow::executor::{NaturalSource, TriggerContext};

/// Extended trait for trigger nodes that need lifecycle management
/// Examples: webhooks, polling, WebSocket connections
#[async_trait]
pub trait FlowTriggerNode: FlowNode {
    /// Called when the flow is activated (trigger should start listening)
    ///
    /// Examples:
    /// - Webhook: Register webhook URL with external service
    /// - Polling: Start background polling task
    /// - Connection: Open WebSocket or SSE connection
    async fn on_activate(&self, context: &TriggerContext) -> Result<(), String>;

    /// Called when the flow is deactivated (trigger should stop and cleanup)
    ///
    /// Examples:
    /// - Webhook: Unregister webhook from external service
    /// - Polling: Stop background polling task
    /// - Connection: Close WebSocket or SSE connection
    async fn on_deactivate(&self, context: &TriggerContext) -> Result<(), String>;

    /// Get trigger type for logging and debugging
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Manual
    }

    /// Get natural source preference for this trigger
    ///
    /// This determines where events should ideally come from:
    /// - BackendOnly: Streaming events (Twitch, Kick, YouTube) - handled by backend server
    /// - LocalOnly: Personal integrations (Discord bot, GitHub, file watching) - handled locally
    /// - Any: Can be triggered from either source with de-duplication
    fn natural_source(&self) -> NaturalSource {
        NaturalSource::Any // Default: accept from any source with de-duplication
    }
}

/// Types of triggers available
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    /// Manual execution (button click, API call)
    Manual,

    /// Webhook endpoint that receives HTTP requests
    Webhook,

    /// Polling external API/service on interval
    Polling,

    /// Persistent connection (WebSocket, SSE)
    Connection,

    /// Scheduled execution (cron-like)
    Schedule,
}
