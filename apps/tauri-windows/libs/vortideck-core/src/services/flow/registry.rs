use crate::services::flow::flow_control::ToolNodeInfo;
use crate::services::flow::traits::{FlowNode, FlowNodeProvider};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Registry for all flow nodes across different modules
pub struct FlowNodeRegistry {
    providers: Vec<Box<dyn FlowNodeProvider>>,
    nodes: HashMap<String, Arc<dyn FlowNode>>,
    /// Track which provider each node belongs to for hot reloading
    node_providers: HashMap<String, String>, // node_id -> provider_name
}

impl FlowNodeRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            nodes: HashMap::new(),
            node_providers: HashMap::new(),
        }
    }

    /// Register a new provider, which contributes flow nodes to the registry
    pub fn register_provider(&mut self, provider: Box<dyn FlowNodeProvider>) {
        // Use the provider's unique name for tracking
        let provider_name = provider.provider_name().to_string();

        // Add all nodes from this provider to our registry
        for node in provider.get_flow_nodes() {
            let id = node.id();
            if let Some(existing_provider) = self.node_providers.get(&id) {
                log::error!(
                    "Rejected duplicate flow node ID '{}' from provider '{}'; already owned by '{}'",
                    id,
                    provider_name,
                    existing_provider
                );
                continue;
            }
            self.nodes.insert(id.clone(), Arc::from(node));
            self.node_providers.insert(id, provider_name.clone());
        }

        // Keep the provider for potential reloading/updates
        self.providers.push(provider);
    }

    /// Get all registered flow nodes as ToolNodeInfo for the UI
    pub fn get_all_node_info(&self) -> Vec<ToolNodeInfo> {
        self.nodes
            .values()
            .map(|node| {
                let tool_info = ToolNodeInfo {
                    id: node.id(), // Include the actual backend node ID
                    name: node.name(),
                    description: node.description(),
                    parameters: node.parameters_schema(),
                    type_name: node.category().to_lowercase(),
                    color: "#3A3A3A".to_string(), // Default background color
                    type_color: node.type_color(),
                    outputs: node.outputs_schema(),
                    category_display_name: node.category_display_name(),
                    subcategory: node.subcategory(),
                };

                tool_info
            })
            .collect()
    }

    /// Get all registered node IDs for debugging
    pub fn get_all_node_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    /// Get the total number of registered nodes
    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get a specific node by its ID
    pub fn get_node(&self, id: &str) -> Option<Arc<dyn FlowNode>> {
        self.nodes.get(id).cloned()
    }

    /// Get a specific node by its display name
    pub fn get_node_by_name(&self, name: &str) -> Option<Arc<dyn FlowNode>> {
        self.nodes
            .values()
            .find(|node| node.name() == name)
            .cloned()
    }

    /// Execute a flow node with the given parameters
    pub async fn execute_node(&self, id: &str, params: Value) -> Result<Value, String> {
        match self.get_node(id) {
            Some(node) => node.execute(params).await,
            _ => Err(format!("Node with ID '{}' not found", id)),
        }
    }

    /// Clear nodes from a specific provider type (for hot reloading)
    pub fn clear_provider_nodes(&mut self, provider_type: &str) {
        let mut keys_to_remove = Vec::new();

        // Find all nodes belonging to this provider
        for (node_id, tracked_provider_type) in &self.node_providers {
            if tracked_provider_type.contains(provider_type)
                || (provider_type == "WasmPluginProvider" && node_id.starts_with("plugin."))
            {
                keys_to_remove.push(node_id.clone());
            }
        }

        // Remove nodes and their provider tracking
        for key in &keys_to_remove {
            self.nodes.remove(key);
            self.node_providers.remove(key);
        }

        // Remove providers from the providers list
        self.providers.retain(|provider| {
            let current_provider_name = provider.provider_name();
            current_provider_name != provider_type
        });
    }

    /// Replace provider families as one registry mutation. Stale nodes are
    /// removed before the current provider snapshot becomes visible.
    pub fn replace_providers(
        &mut self,
        provider_names: &[&str],
        providers: Vec<Box<dyn FlowNodeProvider>>,
    ) {
        for provider_name in provider_names {
            self.clear_provider_nodes(provider_name);
        }
        for provider in providers {
            self.register_provider(provider);
        }
    }
}

/// A singleton accessor for the global FlowNodeRegistry
pub struct FlowRegistry {
    registry: Arc<Mutex<FlowNodeRegistry>>,
}

impl FlowRegistry {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(FlowNodeRegistry::new())),
        }
    }

    pub fn get_registry(&self) -> Arc<Mutex<FlowNodeRegistry>> {
        self.registry.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    struct TestNode(&'static str);

    #[async_trait]
    impl FlowNode for TestNode {
        fn id(&self) -> String {
            self.0.to_string()
        }
        fn name(&self) -> String {
            self.0.to_string()
        }
        fn description(&self) -> String {
            "test node".to_string()
        }
        fn category(&self) -> String {
            "test".to_string()
        }
        fn parameters_schema(&self) -> Value {
            json!({ "type": "object" })
        }
        fn type_color(&self) -> String {
            "#000000".to_string()
        }
        async fn execute(&self, _params: Value) -> Result<Value, String> {
            Ok(Value::Null)
        }
    }

    struct TestProvider {
        name: &'static str,
        node_ids: Vec<&'static str>,
    }

    impl FlowNodeProvider for TestProvider {
        fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
            self.node_ids
                .iter()
                .map(|id| Box::new(TestNode(id)) as Box<dyn FlowNode>)
                .collect()
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }
    }

    #[test]
    fn replacing_provider_families_removes_stale_nodes_only() {
        let mut registry = FlowNodeRegistry::new();
        registry.register_provider(Box::new(TestProvider {
            name: "BuiltinProvider",
            node_ids: vec!["builtin.keep"],
        }));
        registry.register_provider(Box::new(TestProvider {
            name: "WasmPluginProvider",
            node_ids: vec!["plugin.old"],
        }));

        registry.replace_providers(
            &["WasmPluginProvider", "ServicePlugin"],
            vec![Box::new(TestProvider {
                name: "ServicePlugin",
                node_ids: vec!["plugin.current"],
            })],
        );

        assert!(registry.get_node("plugin.old").is_none());
        assert!(registry.get_node("plugin.current").is_some());
        assert!(registry.get_node("builtin.keep").is_some());
    }
}
