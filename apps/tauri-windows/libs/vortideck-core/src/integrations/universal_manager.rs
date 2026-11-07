use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use crate::core::state_manager::StateManager;
use crate::integrations::{
    contracts::{
        ActionPreflight, ActionPreflightIssue, ActionPreflightStatus, ActionReceiptSemantics,
        ActionRepairKind, CapabilityDefinition, CapabilityOption,
    },
    interface::{IntegrationAction, IntegrationActionResult, IntegrationId, IntegrationMetadata},
    traits::{AppIntegration, IntegrationError},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    pub id: String,
    pub name: String,
    pub status: ConnectionStatus,
    pub last_updated: u64, // Unix timestamp
    pub metadata: Option<IntegrationMetadata>,
    pub actions_available: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

impl ConnectionStatus {
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionStatus::Connected)
    }
}

#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit(
        &self,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct UniversalIntegrationManager {
    state_manager: Arc<StateManager>,
    integrations: Arc<RwLock<HashMap<IntegrationId, Arc<dyn AppIntegration>>>>,
    status_cache: Arc<RwLock<HashMap<String, IntegrationStatus>>>,
    event_emitter: Option<Arc<dyn EventEmitter>>,
}

impl UniversalIntegrationManager {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            state_manager,
            integrations: Arc::new(RwLock::new(HashMap::new())),
            status_cache: Arc::new(RwLock::new(HashMap::new())),
            event_emitter: None,
        }
    }

    pub fn with_event_emitter(mut self, emitter: Arc<dyn EventEmitter>) -> Self {
        self.event_emitter = Some(emitter);
        self
    }

    async fn integration(
        &self,
        integration_id: &str,
    ) -> Result<Arc<dyn AppIntegration>, IntegrationError> {
        self.integrations
            .read()
            .await
            .get(&IntegrationId::new(integration_id))
            .cloned()
            .ok_or_else(|| {
                IntegrationError::NotFound(format!("Integration not found: {}", integration_id))
            })
    }

    pub async fn resolve_capability(
        &self,
        integration_id: &str,
        action_id: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<CapabilityDefinition, IntegrationError> {
        self.integration(integration_id)
            .await?
            .resolve_capability(action_id, parameters)
            .await
    }

    pub async fn preflight_action(
        &self,
        integration_id: &str,
        action_id: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> ActionPreflight {
        self.preflight_action_versioned(integration_id, action_id, parameters, None)
            .await
    }

    /// Preflight a persisted binding against the current provider contract.
    pub async fn preflight_action_versioned(
        &self,
        integration_id: &str,
        action_id: &str,
        parameters: &HashMap<String, serde_json::Value>,
        expected_capability_version: Option<u32>,
    ) -> ActionPreflight {
        let integration = match self.integration(integration_id).await {
            Ok(integration) => integration,
            Err(error) => {
                return ActionPreflight {
                    integration_id: integration_id.to_string(),
                    action_id: action_id.to_string(),
                    status: ActionPreflightStatus::MissingIntegration,
                    capability: None,
                    issues: vec![ActionPreflightIssue {
                        code: "integration_not_registered".to_string(),
                        message: error.to_string(),
                        recoverable: true,
                        repairs: vec![ActionRepairKind::OpenIntegrationSettings],
                    }],
                };
            }
        };

        let capability = match integration.resolve_capability(action_id, parameters).await {
            Ok(capability) => capability,
            Err(error) => {
                let (status, code, repairs) = match &error {
                    IntegrationError::NotFound(_) => (
                        ActionPreflightStatus::MissingTarget,
                        "target_not_found",
                        vec![ActionRepairKind::RebindAction],
                    ),
                    IntegrationError::PermissionDenied(_)
                    | IntegrationError::AuthenticationError(_) => (
                        ActionPreflightStatus::PermissionDenied,
                        "permission_denied",
                        vec![ActionRepairKind::OpenIntegrationSettings],
                    ),
                    _ => (
                        ActionPreflightStatus::InvalidAction,
                        "capability_invalid",
                        vec![ActionRepairKind::RebindAction],
                    ),
                };
                return ActionPreflight {
                    integration_id: integration_id.to_string(),
                    action_id: action_id.to_string(),
                    status,
                    capability: None,
                    issues: vec![ActionPreflightIssue {
                        code: code.to_string(),
                        message: error.to_string(),
                        recoverable: true,
                        repairs,
                    }],
                };
            }
        };

        if let Some(expected_version) = expected_capability_version {
            if expected_version != capability.version {
                return ActionPreflight {
                    integration_id: integration_id.to_string(),
                    action_id: action_id.to_string(),
                    status: ActionPreflightStatus::ContractDrift,
                    capability: Some(capability.clone()),
                    issues: vec![ActionPreflightIssue {
                        code: "capability_version_drift".to_string(),
                        message: format!(
                            "Saved contract version {} no longer matches provider version {}",
                            expected_version, capability.version
                        ),
                        recoverable: true,
                        repairs: vec![ActionRepairKind::RebindAction],
                    }],
                };
            }
        }

        if matches!(
            self.get_integration_status(integration_id)
                .await
                .map(|status| status.status),
            Some(ConnectionStatus::Connecting)
        ) {
            return ActionPreflight {
                integration_id: integration_id.to_string(),
                action_id: action_id.to_string(),
                status: ActionPreflightStatus::Syncing,
                capability: Some(capability),
                issues: vec![ActionPreflightIssue {
                    code: "integration_syncing".to_string(),
                    message: format!("Integration '{}' is still syncing", integration_id),
                    recoverable: true,
                    repairs: vec![ActionRepairKind::RetryPreflight],
                }],
            };
        }

        if !integration.is_available().await {
            return ActionPreflight {
                integration_id: integration_id.to_string(),
                action_id: action_id.to_string(),
                status: ActionPreflightStatus::Disconnected,
                capability: Some(capability),
                issues: vec![ActionPreflightIssue {
                    code: "integration_disconnected".to_string(),
                    message: format!("Integration '{}' is not connected", integration_id),
                    recoverable: true,
                    repairs: vec![ActionRepairKind::OpenIntegrationSettings],
                }],
            };
        }

        if capability.contract.receipt == ActionReceiptSemantics::TransportAccepted {
            return ActionPreflight {
                integration_id: integration_id.to_string(),
                action_id: action_id.to_string(),
                status: ActionPreflightStatus::ReceiptDegraded,
                capability: Some(capability),
                issues: vec![ActionPreflightIssue {
                    code: "transport_receipt_only".to_string(),
                    message: "Provider confirms transport acceptance, not final application"
                        .to_string(),
                    recoverable: false,
                    repairs: vec![ActionRepairKind::ReviewReceiptSemantics],
                }],
            };
        }

        ActionPreflight {
            integration_id: integration_id.to_string(),
            action_id: action_id.to_string(),
            status: ActionPreflightStatus::Ready,
            capability: Some(capability),
            issues: vec![],
        }
    }

    /// Register an integration with the universal manager
    pub async fn register_integration(
        &self,
        integration: Arc<dyn AppIntegration>,
    ) -> Result<(), IntegrationError> {
        self.register_integrations(vec![integration]).await
    }

    /// Prepare provider metadata concurrently, then publish the complete batch
    /// under one write barrier. Readers therefore see either the old catalog or
    /// the complete new catalog, never a provider-by-provider startup sequence.
    pub async fn register_integrations(
        &self,
        integrations: Vec<Arc<dyn AppIntegration>>,
    ) -> Result<(), IntegrationError> {
        let mut ids = HashSet::new();
        for integration in &integrations {
            let id = integration.id().as_str().to_string();
            if !ids.insert(id.clone()) {
                return Err(IntegrationError::AlreadyRegistered(id));
            }
        }

        let prepared =
            futures::future::join_all(integrations.into_iter().map(|integration| async move {
                let integration_id = integration.id().clone();
                let metadata = integration.get_metadata();
                let status = IntegrationStatus {
                    id: integration_id.as_str().to_string(),
                    name: metadata.name.clone(),
                    status: if integration.is_available().await {
                        ConnectionStatus::Connected
                    } else {
                        ConnectionStatus::Disconnected
                    },
                    last_updated: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    metadata: Some(metadata),
                    actions_available: integration.get_capabilities().await.len(),
                    error_message: None,
                };
                (integration_id, integration, status)
            }))
            .await;

        {
            let mut registered = self.integrations.write().await;
            let mut cache = self.status_cache.write().await;
            if let Some((id, _, _)) = prepared
                .iter()
                .find(|(id, _, _)| registered.contains_key(id))
            {
                return Err(IntegrationError::AlreadyRegistered(id.as_str().to_string()));
            }
            for (id, integration, status) in &prepared {
                registered.insert(id.clone(), integration.clone());
                cache.insert(id.as_str().to_string(), status.clone());
            }
        }

        for (integration_id, _, status) in prepared {
            self.state_manager
                .set_integration_status(integration_id.as_str().to_string(), &status.status)
                .await;
            if let Some(emitter) = &self.event_emitter {
                let _ = emitter
                    .emit(
                        "integration_registered",
                        json!({
                            "integration_id": integration_id.as_str(),
                            "status": status,
                        }),
                    )
                    .await;
            }
            println!(
                "✅ Registered integration: {} ({})",
                status.name,
                integration_id.as_str()
            );
        }
        Ok(())
    }

    /// Update integration status - Universal method for all integrations
    pub async fn update_integration_status(
        &self,
        integration_id: &str,
        status: ConnectionStatus,
    ) -> Result<(), IntegrationError> {
        let mut updated_status = None;

        // Update status cache
        {
            let mut cache = self.status_cache.write().await;
            if let Some(existing) = cache.get_mut(integration_id) {
                existing.status = status.clone();
                existing.last_updated = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Update error message if status is error
                if let ConnectionStatus::Error(ref msg) = status {
                    existing.error_message = Some(msg.clone());
                } else {
                    existing.error_message = None;
                }

                updated_status = Some(existing.clone());
            }
        }

        if let Some(status_info) = updated_status {
            // Update StateManager
            self.state_manager
                .set_integration_status(integration_id.to_string(), &status)
                .await;

            // Emit universal event
            if let Some(emitter) = &self.event_emitter {
                let _ = emitter.emit("integration_status_changed", json!({
                    "integration_id": integration_id,
                    "status": status_info,
                    "timestamp": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                })).await;
            }

            println!(
                "🔄 Updated integration status: {} -> {:?}",
                integration_id, status
            );
            Ok(())
        } else {
            Err(IntegrationError::NotFound(format!(
                "Integration not found: {}",
                integration_id
            )))
        }
    }

    /// Get status for a specific integration
    pub async fn get_integration_status(&self, integration_id: &str) -> Option<IntegrationStatus> {
        let cache = self.status_cache.read().await;
        cache.get(integration_id).cloned()
    }

    /// Get all integration statuses
    pub async fn get_all_integration_statuses(&self) -> HashMap<String, IntegrationStatus> {
        let cache = self.status_cache.read().await;
        cache.clone()
    }

    /// Execute action on a specific integration
    pub(crate) async fn execute_integration_action(
        &self,
        integration_id: &str,
        action: IntegrationAction,
    ) -> Result<IntegrationActionResult, IntegrationError> {
        let preflight = self
            .preflight_action(integration_id, &action.name, &action.parameters)
            .await;
        if !preflight.is_ready() {
            let message = preflight
                .issues
                .first()
                .map(|issue| issue.message.clone())
                .unwrap_or_else(|| "Action preflight failed".to_string());
            return match preflight.status {
                ActionPreflightStatus::Disconnected => Err(IntegrationError::NotConnected(message)),
                ActionPreflightStatus::MissingIntegration => {
                    Err(IntegrationError::NotFound(message))
                }
                _ => Err(IntegrationError::ValidationError(message)),
            };
        }

        let integration = self.integration(integration_id).await?;

        // Execute the action
        match integration.execute_action(action).await {
            Ok(result) => {
                // Update last activity timestamp
                self.update_integration_last_activity(integration_id).await;
                Ok(result)
            }
            Err(e) => {
                // Update status to error if action fails
                self.update_integration_status(
                    integration_id,
                    ConnectionStatus::Error(e.to_string()),
                )
                .await?;
                Err(e)
            }
        }
    }

    /// Update last activity timestamp for an integration
    async fn update_integration_last_activity(&self, integration_id: &str) {
        let mut cache = self.status_cache.write().await;
        if let Some(status) = cache.get_mut(integration_id) {
            status.last_updated = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
    }

    /// Get connected integrations
    pub async fn get_connected_integrations(&self) -> Vec<IntegrationStatus> {
        let cache = self.status_cache.read().await;
        cache
            .values()
            .filter(|status| status.status.is_connected())
            .cloned()
            .collect()
    }

    /// Check if a specific integration is connected
    pub async fn is_integration_connected(&self, integration_id: &str) -> bool {
        let cache = self.status_cache.read().await;
        cache
            .get(integration_id)
            .map(|status| status.status.is_connected())
            .unwrap_or(false)
    }

    /// Get available actions for an integration
    pub async fn get_integration_capabilities(
        &self,
        integration_id: &str,
    ) -> Result<Vec<CapabilityDefinition>, IntegrationError> {
        let integration = self.integration(integration_id).await?;

        Ok(integration.get_capabilities().await)
    }

    pub async fn resolve_capability_options(
        &self,
        integration_id: &str,
        capability_id: &str,
        parameter_id: &str,
        current_values: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<CapabilityOption>, IntegrationError> {
        let integration = self.integration(integration_id).await?;
        integration
            .resolve_capability_options(capability_id, parameter_id, current_values)
            .await
    }

    /// Poll all integrations for status updates
    pub async fn poll_all_integrations(&self) -> Result<(), IntegrationError> {
        let integrations = {
            let integrations = self.integrations.read().await;
            integrations.clone()
        };

        for (integration_id, integration) in integrations {
            let current_status = if integration.is_available().await {
                ConnectionStatus::Connected
            } else {
                ConnectionStatus::Disconnected
            };

            // Only update if status changed
            let needs_update = {
                let cache = self.status_cache.read().await;
                cache
                    .get(integration_id.as_str())
                    .map(|status| {
                        !matches!(
                            (&status.status, &current_status),
                            (ConnectionStatus::Connected, ConnectionStatus::Connected)
                                | (
                                    ConnectionStatus::Disconnected,
                                    ConnectionStatus::Disconnected
                                )
                        )
                    })
                    .unwrap_or(true)
            };

            if needs_update {
                let _ = self
                    .update_integration_status(integration_id.as_str(), current_status)
                    .await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::builtin::test::TestIntegration;

    #[tokio::test]
    async fn preflight_reports_an_unregistered_provider_without_dispatching() {
        let manager = UniversalIntegrationManager::new(Arc::new(StateManager::new()));
        let result = manager
            .preflight_action("missing", "do_work", &HashMap::new())
            .await;

        assert_eq!(result.status, ActionPreflightStatus::MissingIntegration);
        assert!(result.capability.is_none());
        assert_eq!(result.issues[0].code, "integration_not_registered");
    }

    #[tokio::test]
    async fn duplicate_batch_is_rejected_without_partial_publication() {
        let manager = UniversalIntegrationManager::new(Arc::new(StateManager::new()));
        let providers: Vec<Arc<dyn AppIntegration>> = vec![
            Arc::new(TestIntegration::new()),
            Arc::new(TestIntegration::new()),
        ];

        assert!(matches!(
            manager.register_integrations(providers).await,
            Err(IntegrationError::AlreadyRegistered(id)) if id == "test"
        ));
        assert!(manager.get_all_integration_statuses().await.is_empty());
        assert!(manager.get_integration_capabilities("test").await.is_err());
    }
}

/// Integration Status Monitor - Background service for polling integration statuses
pub struct IntegrationStatusMonitor {
    manager: Arc<UniversalIntegrationManager>,
    poll_interval: std::time::Duration,
}

impl IntegrationStatusMonitor {
    pub fn new(
        manager: Arc<UniversalIntegrationManager>,
        poll_interval: std::time::Duration,
    ) -> Self {
        Self {
            manager,
            poll_interval,
        }
    }

    pub async fn start_monitoring(&self) {
        let mut interval = tokio::time::interval(self.poll_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.manager.poll_all_integrations().await {
                eprintln!("❌ Error polling integrations: {}", e);
            }
        }
    }

    pub fn spawn_monitor(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.start_monitoring().await;
        })
    }
}
