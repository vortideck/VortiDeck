use crate::integrations::{
    interface::{
        IntegrationAction, IntegrationActionResult, IntegrationActionStatus, IntegrationId,
        IntegrationMetadata, IntegrationRequirements, IntegrationResult,
    },
    traits::{AppIntegration, EventCallback, IntegrationError},
    ActionContract, ActionReceiptSemantics, ActionRecoverySemantics, CapabilityDefinition,
    CapabilityKind, CapabilityPresentation,
};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct TestIntegration {
    id: IntegrationId,
    name: String,
    description: String,
}

impl TestIntegration {
    pub fn new() -> Self {
        Self {
            id: IntegrationId::new("test"),
            name: "Test Integration".to_string(),
            description: "A test integration for development purposes".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AppIntegration for TestIntegration {
    fn id(&self) -> &IntegrationId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn get_metadata(&self) -> IntegrationMetadata {
        IntegrationMetadata {
            id: self.id.clone(),
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            description: self.description.clone(),
            author: "VortiDeck".to_string(),
            website: None,
        }
    }

    fn get_requirements(&self) -> IntegrationRequirements {
        IntegrationRequirements {
            min_version: None,
            max_version: None,
            dependencies: vec![],
        }
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn execute_action(
        &self,
        action: IntegrationAction,
    ) -> IntegrationResult<IntegrationActionResult> {
        match action.name.as_str() {
            "test_action" => Ok(IntegrationActionResult {
                status: IntegrationActionStatus::TransportAccepted,
                request_id: action
                    .execution
                    .and_then(|execution| execution.request_id)
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                provider_receipt_id: None,
                outputs: HashMap::new(),
            }),
            _ => Err(IntegrationError::InvalidAction(format!(
                "Unknown action: {}",
                action.name
            ))),
        }
    }

    async fn get_capabilities(&self) -> Vec<CapabilityDefinition> {
        vec![CapabilityDefinition {
            integration_id: "test".into(),
            capability_id: "test_action".into(),
            version: 1,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: "Test Action".into(),
                description: "A test action".into(),
                category: "Test".into(),
                icon: None,
                aliases: vec![],
            },
            inputs: vec![],
            outputs: vec![],
            allow_additional_inputs: false,
            contract: ActionContract {
                integration_id: "test".into(),
                action_id: "test_action".into(),
                version: 1,
                required_permission: crate::plugins::bundle::PluginPermission::TriggerAction(
                    "integration:test:test_action".into(),
                ),
                timeout_ms: 1_000,
                recovery: ActionRecoverySemantics::FailClosed,
                resource_key: Some("test:action".into()),
                receipt: ActionReceiptSemantics::TransportAccepted,
            },
        }]
    }

    async fn subscribe_to_events(
        &self,
        _callback: Box<dyn EventCallback>,
    ) -> IntegrationResult<()> {
        Ok(())
    }
}
