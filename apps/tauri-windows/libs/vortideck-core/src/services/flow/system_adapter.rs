use crate::services::flow::traits as flow_traits;
use crate::services::system::flow_traits as system_traits;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// This struct adapts a system_control::flow_traits::FlowNode to a flow_control::traits::FlowNode
pub struct SystemNodeAdapter {
    inner: Arc<dyn system_traits::FlowNode>,
}

impl SystemNodeAdapter {
    pub fn new(node: Arc<dyn system_traits::FlowNode>) -> Self {
        Self { inner: node }
    }
}

#[async_trait]
impl flow_traits::FlowNode for SystemNodeAdapter {
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

    fn parameters_schema(&self) -> Value {
        self.inner.parameters_schema()
    }

    fn type_color(&self) -> String {
        self.inner.type_color()
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        // Convert from system NodeOutput to flow_control NodeOutput
        self.inner
            .outputs_schema()
            .into_iter()
            .map(|output| crate::services::flow::traits::NodeOutput {
                id: output.id,
                name: output.name,
                output_type: output.output_type,
                description: output.description,
                ..Default::default()
            })
            .collect()
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        self.inner.execute(params).await
    }
}

// Adapter provider that adapts system_control::flow_traits::FlowNodeProvider to flow_control::traits::FlowNodeProvider
pub struct SystemProviderAdapter {
    inner: Box<dyn system_traits::FlowNodeProvider>,
}

impl SystemProviderAdapter {
    pub fn new(provider: Box<dyn system_traits::FlowNodeProvider>) -> Self {
        Self { inner: provider }
    }
}

impl flow_traits::FlowNodeProvider for SystemProviderAdapter {
    fn get_flow_nodes(&self) -> Vec<Box<dyn flow_traits::FlowNode>> {
        self.inner
            .get_flow_nodes()
            .into_iter()
            .map(|node| {
                let adapter = SystemNodeAdapter::new(Arc::from(node));
                Box::new(adapter) as Box<dyn flow_traits::FlowNode>
            })
            .collect()
    }
}
