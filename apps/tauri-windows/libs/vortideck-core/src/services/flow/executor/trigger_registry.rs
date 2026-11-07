//! Active workflow-trigger lifecycle and registry synchronization.

use serde_json::Value;

use super::{ActiveTriggerDefinition, EventEmitter, FlowDefinition, FlowExecutor};

impl FlowExecutor {
    // ============================================================================
    // TRIGGER LIFECYCLE METHODS
    // ============================================================================

    pub(super) fn active_trigger_definitions(
        flow: &FlowDefinition,
    ) -> Vec<ActiveTriggerDefinition> {
        flow.nodes
            .iter()
            .filter(|node| node.node_type == "stream-trigger")
            .map(|node| ActiveTriggerDefinition {
                flow_id: flow.id.clone(),
                node_id: node.id.clone(),
                node_type: node.node_type.clone(),
                platform: node
                    .inputs
                    .get("service")
                    .or_else(|| node.inputs.get("platform"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                event_type: node
                    .inputs
                    .get("event_type")
                    .or_else(|| node.inputs.get("eventType"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
            .collect()
    }

    pub(super) async fn synchronize_trigger_registry(
        &self,
        flow: &FlowDefinition,
    ) -> Result<(), String> {
        if flow.is_active {
            let definitions = Self::active_trigger_definitions(flow);
            self.storage
                .replace_active_triggers(&flow.id, &definitions)
                .await
        } else {
            self.storage.remove_active_triggers(&flow.id).await
        }
    }

    pub(super) async fn emit_trigger_registry_changed(&self, flow_id: &str) {
        let payload = crate::events::EventPayload::Runtime(
            crate::events::RuntimeEventPayload::TriggerRegistryChanged {
                flow_id: flow_id.to_string(),
            },
        );

        match &self.event_emitter {
            EventEmitter::Universal(platform_context) => {
                if let Err(error) = platform_context.event_bus.emit(payload) {
                    log::error!("Failed to announce trigger registry update: {}", error);
                }
            }
        }
    }

    /// Activate a flow and atomically replace its persisted routing entries.
    pub async fn activate_flow(&self, flow_id: &str) -> Result<(), String> {
        let mut flow = self.storage.load_flow(flow_id).await?;
        flow.is_active = true;
        let trigger_count = Self::active_trigger_definitions(&flow).len();
        self.save_flow(&flow).await?;
        log::info!(
            "Activated flow '{}' with {} persisted trigger route(s)",
            flow.name,
            trigger_count
        );
        Ok(())
    }

    /// Deactivate a flow, remove routing entries, and clear trigger state.
    pub async fn deactivate_flow(&self, flow_id: &str) -> Result<(), String> {
        let mut flow = self.storage.load_flow(flow_id).await?;
        flow.is_active = false;
        let trigger_nodes = Self::active_trigger_definitions(&flow);
        self.save_flow(&flow).await?;

        for trigger in &trigger_nodes {
            if let Err(error) = self
                .storage
                .delete_trigger_state(flow_id, &trigger.node_id)
                .await
            {
                log::warn!("Failed to clear trigger state: {}", error);
            }
        }

        log::info!(
            "Deactivated flow '{}' and removed {} trigger route(s)",
            flow.name,
            trigger_nodes.len()
        );
        Ok(())
    }
}
