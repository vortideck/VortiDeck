use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents an output connection point for a flow node  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    /// Unique identifier for this output
    pub id: String,
    /// Display name for this output
    pub name: String,
    /// Type of data this output produces
    pub output_type: String,
    /// Optional description of what this output represents
    pub description: Option<String>,
}

/// Common trait for all tools and actions that can be used in the flow builder
#[async_trait]
pub trait FlowNode: Send + Sync {
    /// Unique identifier for this node
    fn id(&self) -> String;

    /// Display name shown in the UI
    fn name(&self) -> String;

    /// Description explaining what this node does
    fn description(&self) -> String;

    /// Category for grouping in the UI sidebar (e.g., "SYSTEM", "AUDIO", etc.)
    fn category(&self) -> String;

    /// JSON schema for the input parameters (matching the expected frontend format)
    fn parameters_schema(&self) -> Value;

    /// Color for the node header in the UI
    fn type_color(&self) -> String;

    /// Get the schema for all outputs this node produces
    fn outputs_schema(&self) -> Vec<NodeOutput> {
        // Default to single result output
        vec![NodeOutput {
            id: "result".to_string(),
            name: "Result".to_string(),
            output_type: "string".to_string(),
            description: Some("Node execution result".to_string()),
        }]
    }

    /// Execute the node with the given parameters
    async fn execute(&self, params: Value) -> Result<Value, String>;
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
