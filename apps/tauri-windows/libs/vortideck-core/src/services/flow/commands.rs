// 🚀 UNIVERSAL ARCHITECTURE: Core Flow business logic without platform dependencies
// This module provides platform-agnostic flow functionality

use crate::services::flow::executor::{FlowDefinition, FlowExecutionState, FlowExecutor};
use crate::services::flow::flow_control::ToolNodeInfo;
use crate::services::flow::registry::FlowNodeRegistry;
use crate::services::flow::traits::FlowNode;

use chrono;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid;

// Core state types (platform-agnostic)
#[derive(Clone)]
pub struct FlowRegistryState {
    pub registry: Arc<std::sync::Mutex<FlowNodeRegistry>>,
}

// Global flow executor state (platform-agnostic)
#[derive(Clone)]
pub struct FlowExecutorState {
    pub executor: Arc<Mutex<Option<FlowExecutor>>>,
}

impl FlowRegistryState {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(std::sync::Mutex::new(FlowNodeRegistry::new())),
        }
    }

    pub fn get_all_node_info(&self) -> Vec<ToolNodeInfo> {
        self.registry.lock().unwrap().get_all_node_info()
    }

    pub fn get_node(&self, node_id: &str) -> Option<Arc<dyn FlowNode>> {
        self.registry.lock().unwrap().get_node(node_id)
    }
}

impl FlowExecutorState {
    pub fn new() -> Self {
        Self {
            executor: Arc::new(Mutex::new(None)),
        }
    }
}

// Core business logic functions (platform-agnostic)

/// Core logic for retrieving all flow nodes
pub async fn retrieve_all_flow_nodes_core(
    state: &FlowRegistryState,
) -> Result<Vec<ToolNodeInfo>, String> {
    Ok(state.get_all_node_info())
}

/// Core logic for executing a flow node
pub async fn execute_flow_node_core(
    node_id: String,
    parameters: Value,
    state: &FlowRegistryState,
) -> Result<Value, String> {
    match state.get_node(&node_id) {
        Some(node) => node.execute(parameters).await.map_err(|e| e.to_string()),
        _ => Err(format!("Node with id '{}' not found", node_id)),
    }
}

/// Core logic for retrieving system tools
pub async fn retrieve_system_tools_core(
    _state: &FlowRegistryState,
) -> Result<Vec<ToolNodeInfo>, String> {
    // Return system tools available for flow building
    Ok(vec![
        ToolNodeInfo {
            id: "system.volume.set".to_string(),
            name: "Set Volume".to_string(),
            description: "Set system volume level".to_string(),
            parameters: serde_json::json!({
                "volume": {
                    "type": "number",
                    "description": "Volume level (0-100)",
                    "minimum": 0,
                    "maximum": 100
                }
            }),
            type_name: "system".to_string(),
            color: "#4F46E5".to_string(),
            type_color: "#4F46E5".to_string(),
            outputs: vec![],
            category_display_name: Some("SYSTEM".to_string()),
            subcategory: Some("Audio".to_string()),
        },
        ToolNodeInfo {
            id: "system.keystroke".to_string(),
            name: "Send Keystroke".to_string(),
            description: "Send keyboard input to the system".to_string(),
            parameters: serde_json::json!({
                "key": {
                    "type": "string",
                    "description": "Key to press"
                }
            }),
            type_name: "macro".to_string(),
            color: "#059669".to_string(),
            type_color: "#059669".to_string(),
            outputs: vec![],
            category_display_name: Some("MACRO".to_string()),
            subcategory: Some("Input".to_string()),
        },
    ])
}

/// Core logic for retrieving all tools
pub async fn retrieve_all_tools_core(
    state: &FlowRegistryState,
) -> Result<Vec<ToolNodeInfo>, String> {
    let mut all_tools = retrieve_all_flow_nodes_core(state).await?;
    let system_tools = retrieve_system_tools_core(state).await?;
    all_tools.extend(system_tools);
    Ok(all_tools)
}

// ✅ UNUSED FUNCTIONS REMOVED:
// - convert_frontend_flow_core: Frontend creates FlowDefinition directly
// - is_trigger_node: Not used in current flow execution pipeline

// ✅ CLEAN ARCHITECTURE: Tauri commands moved to adapter layer!
// Core library should contain only pure business logic without platform dependencies
