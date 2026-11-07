use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::integrations::{
    framework::{IntegrationFrameworkResult, IntegrationLifecycleManager},
    interface::{IntegrationId, IntegrationMetadata},
    traits::AppIntegration,
};

/// Manages the lifecycle of integrations
pub struct IntegrationManager {
    integrations: Arc<RwLock<Vec<Box<dyn AppIntegration>>>>,
}

impl IntegrationManager {
    /// Create a new integration manager
    pub fn new() -> Self {
        Self {
            integrations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all registered integrations
    pub async fn list_integrations(&self) -> Vec<IntegrationMetadata> {
        let integrations = self.integrations.read().await;
        integrations.iter().map(|i| i.get_metadata()).collect()
    }
}

#[async_trait]
impl IntegrationLifecycleManager for IntegrationManager {
    async fn register_integration(
        &self,
        integration: Box<dyn AppIntegration>,
    ) -> IntegrationFrameworkResult<()> {
        let mut integrations = self.integrations.write().await;
        integrations.push(integration);
        Ok(())
    }

    async fn unregister_integration(&self, id: IntegrationId) -> IntegrationFrameworkResult<()> {
        let mut integrations = self.integrations.write().await;
        integrations.retain(|i| i.get_metadata().id != id);
        Ok(())
    }

    async fn list_integrations(&self) -> Vec<IntegrationMetadata> {
        let integrations = self.integrations.read().await;
        integrations.iter().map(|i| i.get_metadata()).collect()
    }

    async fn get_integration(&self, id: IntegrationId) -> Option<Box<dyn AppIntegration>> {
        // We can't clone trait objects, so we return None for now
        // This method needs to be redesigned to work without cloning
        None
    }
}
