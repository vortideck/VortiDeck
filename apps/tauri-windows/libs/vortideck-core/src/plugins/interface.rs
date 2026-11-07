use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin bundle information (represents a .wasm file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginBundleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
}

/// Flow node metadata (what the flow system uses)
/// A plugin bundle can provide one or more flow nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNodeMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    /// Path to plugin icon (for UI display via asset protocol)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<std::path::PathBuf>,
    /// Plugin bundle this node belongs to (for multi-node plugins)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_bundle: Option<PluginBundleInfo>,
    /// Category for grouping in UI sidebar (e.g., "AI", "DATA", "SYSTEM")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// JSON schema for input parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_schema: Option<serde_json::Value>,
    /// JSON schema for outputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// Color for the node header in UI (hex color)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_color: Option<String>,
}

/// Legacy alias for backward compatibility
/// @deprecated Use FlowNodeMetadata instead
pub type PluginMetadata = FlowNodeMetadata;

/// Plugin action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAction {
    pub name: String,
    pub parameters: HashMap<String, String>,
}

/// Plugin event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    pub name: String,
    pub data: serde_json::Value,
}

/// Plugin trait
pub trait PluginTrait: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn execute(&self, action: PluginAction) -> Result<(), PluginError>;
}

/// Plugin error
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin error: {0}")]
    Error(String),
}

/// Plugin interface
#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn metadata(&self) -> PluginMetadata;
    async fn initialize(&self) -> Result<(), PluginError>;
    async fn shutdown(&self) -> Result<(), PluginError>;
}
