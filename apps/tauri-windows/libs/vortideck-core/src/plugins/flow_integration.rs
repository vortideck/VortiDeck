//! Flow Integration for WASM Plugins
//!
//! This module bridges WASM plugins with VortiDeck's flow system,
//! allowing plugins to register as FlowNodes and execute within flows.

use crate::plugins::manager::WasmPluginWrapper;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// Use the existing flow traits from VortiDeckCore
pub use crate::services::flow::traits::{FlowNode, FlowNodeProvider, NodeOutput};

/// Implementation of FlowNode for WasmPluginWrapper
#[async_trait]
impl FlowNode for WasmPluginWrapper {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn category(&self) -> String {
        self.category.clone()
    }

    fn category_display_name(&self) -> Option<String> {
        self.category_display_name.clone()
    }

    fn subcategory(&self) -> Option<String> {
        self.subcategory.clone()
    }

    fn parameters_schema(&self) -> Value {
        self.parameters_schema.clone()
    }

    fn type_color(&self) -> String {
        self.type_color.clone()
    }

    fn outputs_schema(&self) -> Vec<NodeOutput> {
        use crate::services::flow::traits::parse_output_schema_to_outputs;

        // Use the same approach as MCP tools - parse output schema JSON
        if let Some(schema) = &self.output_schema {
            parse_output_schema_to_outputs(schema)
        } else {
            // No schema defined - return just the default "result" output
            vec![crate::services::flow::traits::NodeOutput {
                id: "result".to_string(),
                name: "Result".to_string(),
                output_type: "string".to_string(),
                description: Some("Plugin output".to_string()),
                is_dynamic_type: None,
            }]
        }
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Delegate to WasmPluginWrapper's execute method
        self.execute(params).await
    }
}

/// Wrapper to make WasmPluginWrapper work as FlowNode in trait objects
pub struct WasmFlowNodeWrapper {
    inner: Arc<WasmPluginWrapper>,
}

impl WasmFlowNodeWrapper {
    pub fn new(inner: Arc<WasmPluginWrapper>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl FlowNode for WasmFlowNodeWrapper {
    fn id(&self) -> String {
        self.inner.id()
    }
    fn name(&self) -> String {
        self.inner.name()
    }
    fn description(&self) -> String {
        self.inner.description()
    }
    fn category(&self) -> String {
        self.inner.category()
    }
    fn category_display_name(&self) -> Option<String> {
        self.inner.category_display_name()
    }
    fn subcategory(&self) -> Option<String> {
        self.inner.subcategory()
    }
    fn parameters_schema(&self) -> Value {
        self.inner.parameters_schema()
    }
    fn type_color(&self) -> String {
        self.inner.type_color()
    }
    fn outputs_schema(&self) -> Vec<NodeOutput> {
        self.inner.outputs_schema()
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        self.inner.execute(params).await
    }
}

/// FlowNodeProvider implementation for WASM plugins
pub struct WasmFlowNodeProvider {
    plugins: Vec<Arc<WasmPluginWrapper>>,
}

impl WasmFlowNodeProvider {
    pub fn new(plugins: Vec<Arc<WasmPluginWrapper>>) -> Self {
        Self { plugins }
    }
}

impl FlowNodeProvider for WasmFlowNodeProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        self.plugins
            .iter()
            .map(|plugin| Box::new(WasmFlowNodeWrapper::new(plugin.clone())) as Box<dyn FlowNode>)
            .collect()
    }

    fn provider_name(&self) -> &'static str {
        "WasmPluginProvider"
    }
}

/// Flow registry for managing flow nodes
pub struct FlowRegistry {
    providers: Vec<Box<dyn FlowNodeProvider>>,
}

impl FlowRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Box<dyn FlowNodeProvider>) {
        println!("🎯 Registering flow provider");
        self.providers.push(provider);
    }

    pub fn clear_all_providers(&mut self) {
        self.providers.clear();
        println!("🧹 Cleared all providers");
    }

    pub fn get_all_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        let mut all_nodes = Vec::new();
        for provider in &self.providers {
            all_nodes.extend(provider.get_flow_nodes());
        }
        all_nodes
    }

    pub fn get_node_count(&self) -> usize {
        self.providers
            .iter()
            .map(|p| p.get_flow_nodes().len())
            .sum()
    }
}
