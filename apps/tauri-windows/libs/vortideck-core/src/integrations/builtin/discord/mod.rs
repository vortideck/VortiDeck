use crate::core::state_manager::StateManager;
use crate::integrations::{
    contracts::{
        ActionContract, ActionReceiptSemantics, ActionRecoverySemantics, CapabilityDefinition,
        CapabilityKind, CapabilityOption, CapabilityOutput, CapabilityParameter,
        CapabilityPresentation, CapabilityWidget, ContractValueType,
    },
    interface::{
        IntegrationAction, IntegrationActionResult, IntegrationActionStatus, IntegrationId,
        IntegrationMetadata, IntegrationRequirements, IntegrationResult,
    },
    traits::{AppIntegration, EventCallback, IntegrationError},
    ConnectionStatus,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::broadcast;

pub mod auth;
#[cfg(feature = "discord-inbound")]
pub mod gateway;
pub mod inbound;
pub mod webhooks;

#[derive(Clone)]
pub struct DiscordIntegration {
    id: IntegrationId,
    name: String,
    description: String,
    state_manager: Arc<StateManager>,
    webhook_url: Arc<tokio::sync::RwLock<Option<String>>>,
    connected: Arc<tokio::sync::RwLock<bool>>,
}

impl DiscordIntegration {
    const KEYRING_SERVICE: &'static str = "vortideck";

    fn load_secret(target: &str) -> Option<String> {
        keyring::Entry::new_with_target(target, Self::KEYRING_SERVICE, "default")
            .ok()?
            .get_secret()
            .ok()
            .and_then(|secret| String::from_utf8(secret).ok())
    }

    fn save_secret(target: &str, value: &str) -> Result<(), IntegrationError> {
        keyring::Entry::new_with_target(target, Self::KEYRING_SERVICE, "default")
            .map_err(|error| IntegrationError::ConfigurationError(error.to_string()))?
            .set_secret(value.as_bytes())
            .map_err(|error| IntegrationError::ConfigurationError(error.to_string()))
    }

    fn delete_secret(target: &str) {
        if let Ok(entry) = keyring::Entry::new_with_target(target, Self::KEYRING_SERVICE, "default")
        {
            let _ = entry.delete_credential();
        }
    }

    pub fn new(state_manager: Arc<StateManager>) -> Self {
        let webhook_url = Self::load_secret("discord_webhook");
        Self {
            id: IntegrationId::new("discord"),
            name: "Discord".to_string(),
            description: "Integration with Discord for notifications and webhooks".to_string(),
            state_manager,
            webhook_url: Arc::new(tokio::sync::RwLock::new(webhook_url)),
            connected: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    pub async fn initialize(&self) {
        let status = self.check_connection().await;
        *self.connected.write().await = status.is_connected();
        self.state_manager
            .set_integration_status("discord".to_string(), &status)
            .await;
    }

    pub async fn disconnect(&self) {
        Self::delete_secret("discord_webhook");
        *self.webhook_url.write().await = None;
        *self.connected.write().await = false;
        self.state_manager
            .set_integration_status("discord".to_string(), &ConnectionStatus::Disconnected)
            .await;
    }

    /// Configure Discord webhook URL
    pub async fn configure_webhook(&self, webhook_url: String) -> Result<(), IntegrationError> {
        // Validate webhook URL format
        if !webhook_url.starts_with("https://discord.com/api/webhooks/") {
            return Err(IntegrationError::ConfigurationError(
                "Invalid Discord webhook URL format".to_string(),
            ));
        }

        // Validate before replacing a working credential.
        match Self::test_webhook_url(&webhook_url).await {
            Ok(_) => {
                Self::save_secret("discord_webhook", &webhook_url)?;
                *self.webhook_url.write().await = Some(webhook_url);
                *self.connected.write().await = true;
                self.state_manager
                    .set_integration_status("discord".to_string(), &ConnectionStatus::Connected)
                    .await;
                println!("✅ Discord webhook configured and tested successfully");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Test webhook connectivity
    async fn test_webhook(&self) -> Result<(), IntegrationError> {
        let webhook_url = self.webhook_url.read().await.clone().ok_or_else(|| {
            IntegrationError::ConfigurationError("No webhook URL configured".to_string())
        })?;

        Self::test_webhook_url(&webhook_url).await
    }

    async fn test_webhook_url(webhook_url: &str) -> Result<(), IntegrationError> {
        let client = reqwest::Client::new();
        let response = client.get(webhook_url).send().await.map_err(|e| {
            IntegrationError::ConnectionError(format!("Failed to send test webhook: {}", e))
        })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(IntegrationError::ConnectionError(format!(
                "Webhook test failed with status: {}",
                response.status()
            )))
        }
    }

    /// Send a message to Discord via webhook
    /// Uses the API Client service for optimized connection pooling
    async fn send_webhook_message(
        &self,
        content: String,
        embed: Option<serde_json::Value>,
    ) -> Result<(), IntegrationError> {
        let webhook_url = self.webhook_url.read().await.clone().ok_or_else(|| {
            IntegrationError::ConfigurationError("No webhook URL configured".to_string())
        })?;

        let mut message = json!({
            "content": content
        });

        if let Some(embed_data) = embed {
            message["embeds"] = json!([embed_data]);
        }

        // Use the global API client with connection pooling (50-100ms faster than creating new client)
        use crate::services::api::post;

        let response = post(&webhook_url).json(message).send().await.map_err(|e| {
            IntegrationError::ExecutionError(format!("Failed to send webhook message: {}", e))
        })?;

        if response.status >= 200 && response.status < 300 {
            Ok(())
        } else {
            Err(IntegrationError::ExecutionError(format!(
                "Failed to send Discord message, status: {}",
                response.status
            )))
        }
    }

    /// Handle Discord action execution
    async fn handle_discord_action(&self, action: &IntegrationAction) -> IntegrationResult<()> {
        match action.name.as_str() {
            "send_message" => {
                let content = action
                    .parameters
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Message from VortiDeck")
                    .to_string();

                let embed = action.parameters.get("embed").cloned();

                self.send_webhook_message(content, embed).await
            }
            "send_notification" => {
                let title = action
                    .parameters
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("VortiDeck Notification");

                let description = action
                    .parameters
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Notification from VortiDeck");

                let color = action
                    .parameters
                    .get("color")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0x0099ff);

                let embed = json!({
                    "title": title,
                    "description": description,
                    "color": color,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "footer": {
                        "text": "VortiDeck",
                        "icon_url": "https://vortideck.com/favicon.ico"
                    }
                });

                self.send_webhook_message("".to_string(), Some(embed)).await
            }
            "send_alert" => {
                let message = action
                    .parameters
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Alert from VortiDeck");

                let severity = action
                    .parameters
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("info");

                let color = match severity {
                    "error" => 0xff0000,
                    "warning" => 0xffaa00,
                    "success" => 0x00ff00,
                    _ => 0x0099ff,
                };

                let embed = json!({
                    "title": format!("🚨 {} Alert", severity.to_uppercase()),
                    "description": message,
                    "color": color,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "footer": {
                        "text": "VortiDeck Alert System"
                    }
                });

                self.send_webhook_message("".to_string(), Some(embed)).await
            }
            _ => Err(IntegrationError::UnsupportedAction(format!(
                "Unsupported Discord action: {}",
                action.name
            ))),
        }
    }

    fn capability(action_id: &str) -> IntegrationResult<CapabilityDefinition> {
        let parameter =
            |id: &str,
             label: &str,
             description: &str,
             value_type: ContractValueType,
             required: bool,
             widget: CapabilityWidget,
             default_value: Option<Value>,
             options: Vec<CapabilityOption>| CapabilityParameter {
                id: id.to_string(),
                label: label.to_string(),
                description: description.to_string(),
                value_type,
                required,
                widget,
                default_value,
                options,
                option_source: None,
            };
        let (name, description, inputs) = match action_id {
            "send_message" => (
                "Send Message",
                "Send a message to a Discord webhook channel",
                vec![
                    parameter(
                        "content",
                        "Message",
                        "Message content",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Textarea,
                        None,
                        vec![],
                    ),
                    parameter(
                        "embed",
                        "Embed JSON",
                        "Optional Discord embed object",
                        ContractValueType::Object,
                        false,
                        CapabilityWidget::Json,
                        None,
                        vec![],
                    ),
                ],
            ),
            "send_notification" => (
                "Send Notification",
                "Send a formatted Discord notification",
                vec![
                    parameter(
                        "title",
                        "Title",
                        "Notification title",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Text,
                        None,
                        vec![],
                    ),
                    parameter(
                        "description",
                        "Description",
                        "Notification body",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Textarea,
                        None,
                        vec![],
                    ),
                    parameter(
                        "color",
                        "Color",
                        "Embed color",
                        ContractValueType::Integer,
                        false,
                        CapabilityWidget::Color,
                        Some(json!(0x0099ff)),
                        vec![],
                    ),
                ],
            ),
            "send_alert" => (
                "Send Alert",
                "Send a severity-styled Discord alert",
                vec![
                    parameter(
                        "message",
                        "Message",
                        "Alert message",
                        ContractValueType::String,
                        true,
                        CapabilityWidget::Textarea,
                        None,
                        vec![],
                    ),
                    parameter(
                        "severity",
                        "Severity",
                        "Alert severity",
                        ContractValueType::String,
                        false,
                        CapabilityWidget::Select,
                        Some(json!("info")),
                        ["info", "warning", "error", "success"]
                            .into_iter()
                            .map(|value| CapabilityOption {
                                label: value.to_string(),
                                value: json!(value),
                                parameters: vec![],
                            })
                            .collect(),
                    ),
                ],
            ),
            _ => {
                return Err(IntegrationError::UnsupportedAction(format!(
                    "Unsupported Discord action: {}",
                    action_id
                )))
            }
        };
        let version = 1;
        Ok(CapabilityDefinition {
            integration_id: "discord".to_string(),
            capability_id: action_id.to_string(),
            version,
            kind: CapabilityKind::Action,
            presentation: CapabilityPresentation {
                name: name.to_string(),
                description: description.to_string(),
                category: "Discord".to_string(),
                icon: Some("simple-icons:discord".to_string()),
                aliases: vec!["chat".to_string(), "community".to_string()],
            },
            inputs,
            outputs: vec![CapabilityOutput {
                id: "accepted".to_string(),
                label: "Accepted".to_string(),
                value_type: ContractValueType::Boolean,
            }],
            allow_additional_inputs: false,
            contract: ActionContract {
                integration_id: "discord".to_string(),
                action_id: action_id.to_string(),
                version,
                required_permission: crate::plugins::bundle::PluginPermission::TriggerAction(
                    format!("integration:discord:{}", action_id),
                ),
                timeout_ms: 15_000,
                recovery: ActionRecoverySemantics::FailClosed,
                resource_key: Some("discord:webhook".to_string()),
                receipt: ActionReceiptSemantics::TransportAccepted,
            },
        })
    }

    /// Check Discord connection status
    pub async fn check_connection(&self) -> ConnectionStatus {
        if self.webhook_url.read().await.is_some() {
            match self.test_webhook().await {
                Ok(_) => ConnectionStatus::Connected,
                Err(e) => ConnectionStatus::Error(e.to_string()),
            }
        } else {
            ConnectionStatus::Disconnected
        }
    }
}

#[async_trait::async_trait]
impl AppIntegration for DiscordIntegration {
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
            website: Some("https://discord.com".to_string()),
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
        *self.connected.read().await
    }

    async fn execute_action(
        &self,
        action: IntegrationAction,
    ) -> IntegrationResult<IntegrationActionResult> {
        if !self.is_available().await {
            return Err(IntegrationError::NotConnected(
                "Discord integration is not configured".to_string(),
            ));
        }

        let request_id = action
            .execution
            .as_ref()
            .and_then(|execution| execution.request_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        match self.handle_discord_action(&action).await {
            Ok(_) => {
                println!("✅ Discord action '{}' executed successfully", action.name);
                Ok(IntegrationActionResult {
                    status: IntegrationActionStatus::TransportAccepted,
                    request_id,
                    provider_receipt_id: None,
                    outputs: HashMap::from([("accepted".to_string(), json!(true))]),
                })
            }
            Err(e) => {
                println!("❌ Discord action '{}' failed: {}", action.name, e);

                // Update connection status if it's a connection error
                if matches!(
                    e,
                    IntegrationError::ConnectionError(_) | IntegrationError::AuthenticationError(_)
                ) {
                    *self.connected.write().await = false;
                    self.state_manager
                        .set_integration_status(
                            "discord".to_string(),
                            &ConnectionStatus::Error(e.to_string()),
                        )
                        .await;
                }

                Err(e)
            }
        }
    }

    async fn resolve_capability(
        &self,
        action_id: &str,
        parameters: &HashMap<String, Value>,
    ) -> IntegrationResult<CapabilityDefinition> {
        let capability = Self::capability(action_id)?;
        capability
            .validate_parameters(parameters)
            .map_err(IntegrationError::ValidationError)?;
        Ok(capability)
    }

    async fn get_capabilities(&self) -> Vec<CapabilityDefinition> {
        ["send_message", "send_notification", "send_alert"]
            .into_iter()
            .filter_map(|id| Self::capability(id).ok())
            .collect()
    }

    async fn subscribe_to_events(
        &self,
        _callback: Box<dyn EventCallback>,
    ) -> IntegrationResult<()> {
        Err(IntegrationError::NotInitialized(
            "Discord inbound commands are not configured; the current webhook connection is outbound-only"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_capabilities_share_identity_with_their_execution_contracts() {
        for id in ["send_message", "send_notification", "send_alert"] {
            let capability = DiscordIntegration::capability(id).unwrap();
            assert_eq!(capability.integration_id, "discord");
            assert_eq!(capability.capability_id, id);
            assert_eq!(
                capability.contract.integration_id,
                capability.integration_id
            );
            assert_eq!(capability.contract.action_id, capability.capability_id);
            assert!(!capability.outputs.is_empty());
        }
    }

    #[test]
    fn discord_message_requires_typed_content() {
        let capability = DiscordIntegration::capability("send_message").unwrap();
        assert!(capability.validate_parameters(&HashMap::new()).is_err());
        assert!(capability
            .validate_parameters(&HashMap::from([("content".into(), json!("Hello"))]))
            .is_ok());
        assert!(capability
            .validate_parameters(&HashMap::from([("content".into(), json!(42))]))
            .is_err());
    }
}
