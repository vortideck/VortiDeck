use crate::services::flow::traits::{FlowNode, FlowNodeProvider};
use crate::services::mcp::tool::{Tool, ToolSet};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Adapter that turns an MCP Tool into a FlowNode
pub struct McpToolNode {
    tool: Arc<dyn Tool>,
}

impl McpToolNode {
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl FlowNode for McpToolNode {
    fn id(&self) -> String {
        format!("mcp.{}", self.tool.name())
    }

    fn name(&self) -> String {
        self.tool.name()
    }

    fn description(&self) -> String {
        self.tool.description()
    }

    fn category(&self) -> String {
        let name = self.name().to_lowercase();

        // Determine category based on tool name
        if name.contains("date") || name.contains("time") || name.contains("calculator") {
            "SYSTEM".to_string()
        } else if name.contains("search") || name.contains("translate") || name.contains("weather")
        {
            "INTEGRATION".to_string()
        } else if name.contains("audio") || name.contains("sound") || name.contains("voice") {
            "AUDIO".to_string()
        } else if name.contains("obs") || name.contains("stream") {
            "STREAM".to_string()
        } else if name.contains("keyboard") || name.contains("mouse") || name.contains("macro") {
            "MACRO".to_string()
        } else if name.contains("ai") || name.contains("gpt") || name.contains("generate") {
            "AI".to_string()
        } else {
            "UTILITY".to_string()
        }
    }

    fn parameters_schema(&self) -> Value {
        // MCP tools already provide parameters in the correct format
        self.tool.parameters()
    }

    fn type_color(&self) -> String {
        let category = self.category();
        match category.as_str() {
            "SYSTEM" => "#607D8B".to_string(),      // Blue gray
            "INTEGRATION" => "#2196F3".to_string(), // Blue
            "AUDIO" => "#4CAF50".to_string(),       // Green
            "STREAM" => "#FF9800".to_string(),      // Orange
            "MACRO" => "#9C27B0".to_string(),       // Purple
            "AI" => "#E91E63".to_string(),          // Pink
            "UTILITY" => "#00BCD4".to_string(),     // Cyan
            _ => "#607D8B".to_string(),             // Default
        }
    }

    fn output_schema(&self) -> Option<Value> {
        self.tool.output_schema()
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        use crate::services::flow::traits::parse_output_schema_to_outputs;

        // Use the standardized parser function
        // This ensures MCP tools and regular nodes parse outputs the same way
        if let Some(schema) = self.output_schema() {
            // Log when MCP tool has output schema
            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                println!(
                    "📋 MCP Tool '{}' has output schema with {} fields",
                    self.tool.name(),
                    properties.len()
                );
                for field_name in properties.keys() {
                    if field_name != "result" {
                        println!("  ✓ Output field: {}", field_name);
                    }
                }
            }

            parse_output_schema_to_outputs(&schema)
        } else {
            // No schema - use default
            parse_output_schema_to_outputs(&serde_json::json!({}))
        }
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        self.tool
            .call(params)
            .await
            .map(|result| serde_json::from_str(&result).unwrap_or(Value::Null))
            .map_err(|e| e.to_string())
    }
}

/// Provider that exposes all MCP tools as flow nodes
pub struct McpToolProvider {
    tool_set: ToolSet,
}

impl McpToolProvider {
    pub fn new(tool_set: ToolSet) -> Self {
        Self { tool_set }
    }
}

impl FlowNodeProvider for McpToolProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        self.tool_set
            .tools()
            .into_iter()
            .map(|tool| Box::new(McpToolNode::new(tool)) as Box<dyn FlowNode>)
            .collect()
    }

    fn provider_name(&self) -> &'static str {
        "McpToolProvider"
    }
}
