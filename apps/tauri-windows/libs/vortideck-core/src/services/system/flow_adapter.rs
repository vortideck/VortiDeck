use super::flow_traits::{FlowNode, FlowNodeProvider};
#[cfg(target_os = "windows")]
use crate::services::system::system_control::{ActionParameter, SystemAction, SystemControl};

use crate::core::PlatformContext;
#[cfg(not(target_os = "windows"))]
use crate::core::{ActionParameter, SystemAction};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

/// Adapter that turns a SystemAction into a FlowNode
pub struct SystemActionNode {
    action: SystemAction,
    platform_context: Option<Arc<PlatformContext>>,
}

impl SystemActionNode {
    pub fn new(action: SystemAction, platform_context: Option<Arc<PlatformContext>>) -> Self {
        Self {
            action,
            platform_context,
        }
    }
}

#[async_trait]
impl FlowNode for SystemActionNode {
    fn id(&self) -> String {
        self.action.id.clone()
    }

    fn name(&self) -> String {
        self.action.name.clone()
    }

    fn description(&self) -> String {
        self.action.description.clone()
    }

    fn category(&self) -> String {
        if self.action.is_system {
            "SYSTEM".to_string()
        } else {
            // Determine category based on action type
            match self.action.action_type.as_str() {
                "audio" => "AUDIO".to_string(),
                "keyboard" | "mouse" => "MACRO".to_string(),
                "network" => "INTEGRATION".to_string(),
                "stream" | "obs" => "STREAM".to_string(),
                _ => "UTILITY".to_string(),
            }
        }
    }

    fn parameters_schema(&self) -> Value {
        // Convert parameters into properties format matching MCP tools
        let properties: serde_json::Map<String, Value> = self
            .action
            .parameters
            .iter()
            .map(|param| {
                let mut property = json!({
                    "type": param.parameter_type,
                    "description": param.description
                });

                // Add default value if present
                if let Some(default) = &param.default_value {
                    property["default"] = default.clone();
                }

                // Add min/max values for numeric parameters
                if param.parameter_type == "number" {
                    if let Some(min) = param.min_value {
                        property["minimum"] = json!(min);
                    }
                    if let Some(max) = param.max_value {
                        property["maximum"] = json!(max);
                    }
                }

                (param.name.clone(), property)
            })
            .collect();

        // Create final parameters object matching MCP format exactly
        json!({
            "type": "object",
            "properties": properties,
            "required": self.action.parameters.iter()
                .filter(|p| p.required)
                .map(|p| p.name.clone())
                .collect::<Vec<String>>()
        })
    }

    fn type_color(&self) -> String {
        match self.action.action_type.as_str() {
            "audio" => "#4CAF50".to_string(),              // Green
            "keyboard" | "mouse" => "#9C27B0".to_string(), // Purple
            "network" => "#2196F3".to_string(),            // Blue
            "stream" | "obs" => "#FF9800".to_string(),     // Orange
            _ => "#607D8B".to_string(),                    // Blue-gray
        }
    }

    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, String> {
        // Extract parameters from input
        let params = if let Some(params_obj) = input.as_object() {
            params_obj.clone()
        } else {
            serde_json::Map::new()
        };

        // Convert to serde_json::Value for the new interface
        let params_value = serde_json::Value::Object(params);

        // Platform-specific implementation
        #[cfg(target_os = "windows")]
        {
            if let Some(platform_context) = &self.platform_context {
                let system_control = SystemControl::with_platform_context(platform_context.clone());
                system_control
                    .execute_action(&self.action.id, params_value)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Err("Platform context is required for system actions on Windows".to_string())
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Platform context is required for system actions".to_string())
        }
    }
}

/// Provider that exposes all system actions as flow nodes
pub struct SystemActionProvider {
    platform_context: Option<Arc<PlatformContext>>,
}

impl SystemActionProvider {
    pub fn new(platform_context: Option<Arc<PlatformContext>>) -> Self {
        Self { platform_context }
    }

    pub async fn get_actions(&self) -> Vec<SystemAction> {
        #[cfg(target_os = "windows")]
        {
            if let Some(platform_context) = &self.platform_context {
                let system_control = SystemControl::with_platform_context(platform_context.clone());
                system_control.get_available_actions()
            } else {
                // Return default actions when no platform context is available
                let system_control = SystemControl::new();
                system_control.get_available_actions()
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Return empty actions on non-Windows platforms
            Vec::new()
        }
    }
}

impl FlowNodeProvider for SystemActionProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        #[cfg(target_os = "windows")]
        {
            let system_control = if let Some(platform_context) = &self.platform_context {
                SystemControl::with_platform_context(platform_context.clone())
            } else {
                SystemControl::new()
            };

            system_control
                .get_available_actions()
                .into_iter()
                .map(|action| {
                    Box::new(SystemActionNode::new(action, self.platform_context.clone()))
                        as Box<dyn FlowNode>
                })
                .collect()
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Return empty nodes on non-Windows platforms
            Vec::new()
        }
    }

    fn provider_name(&self) -> &'static str {
        "SystemActionProvider"
    }
}
