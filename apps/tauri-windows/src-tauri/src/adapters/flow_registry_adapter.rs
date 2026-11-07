use std::sync::Arc;
use vortideck_core::plugins::manager::FlowRegistryProvider;
use vortideck_core::services::flow::commands::FlowRegistryState;
use vortideck_core::services::flow::traits::FlowNodeProvider;

/// Adapter that bridges the VortiDeckCore plugin manager with Tauri's flow registry
/// This enables headless architecture where plugins automatically register with the flow system
pub struct TauriFlowRegistryAdapter {
    registry_state: Arc<FlowRegistryState>,
}

impl TauriFlowRegistryAdapter {
    pub fn new(registry_state: Arc<FlowRegistryState>) -> Self {
        Self { registry_state }
    }
}

impl FlowRegistryProvider for TauriFlowRegistryAdapter {
    fn replace_plugin_providers(&self, providers: Vec<Box<dyn FlowNodeProvider>>) {
        let mut registry = self.registry_state.registry.lock().unwrap();
        registry.replace_providers(&["WasmPluginProvider", "ServicePlugin"], providers);
    }

    fn get_node_count(&self) -> usize {
        let registry = self.registry_state.registry.lock().unwrap();
        registry.get_node_count()
    }
}
