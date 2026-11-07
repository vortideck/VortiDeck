use crate::integrations::{
    framework::{IntegrationFrameworkResult, IntegrationLifecycleManager},
    interface::{EventCallback, IntegrationId, IntegrationMetadata, IntegrationRequirements},
    traits::{AppIntegration, IntegrationError},
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing integration providers
pub struct IntegrationRegistry {
    integrations: Arc<RwLock<Vec<Box<dyn AppIntegration>>>>,
}

impl IntegrationRegistry {
    /// Create a new integration registry
    pub fn new() -> Self {
        Self {
            integrations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new integration provider
    pub async fn register_provider(
        &self,
        provider: Box<dyn AppIntegration>,
    ) -> Result<(), IntegrationError> {
        let mut integrations = self.integrations.write().await;
        integrations.push(provider);
        Ok(())
    }

    /// Unregister an integration provider
    pub async fn unregister_provider(&self, id: IntegrationId) -> Result<(), IntegrationError> {
        let mut integrations = self.integrations.write().await;
        integrations.retain(|i| i.get_metadata().id != id);
        Ok(())
    }

    /// Get an integration provider by ID
    pub async fn get_provider(&self, id: IntegrationId) -> bool {
        let integrations = self.integrations.read().await;
        integrations.iter().any(|i| i.get_metadata().id == id)
    }
}

// Ensure IntegrationRegistry is Send + Sync
unsafe impl Send for IntegrationRegistry {}
unsafe impl Sync for IntegrationRegistry {}

#[async_trait]
impl IntegrationLifecycleManager for IntegrationRegistry {
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
