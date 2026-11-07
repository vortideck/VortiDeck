use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::integrations::interface::IntegrationId;
use crate::integrations::traits::AppIntegration;

/// Manages the lifecycle of integrations
pub struct IntegrationLifecycleManager {
    active_integrations: Arc<RwLock<HashMap<IntegrationId, Box<dyn AppIntegration>>>>,
}

impl IntegrationLifecycleManager {
    pub fn new() -> Self {
        Self {
            active_integrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_integration(&self, id: IntegrationId, integration: Box<dyn AppIntegration>) {
        let mut integrations = self.active_integrations.write().await;
        integrations.insert(id, integration);
    }

    pub async fn stop_integration(&self, id: &IntegrationId) {
        let mut integrations = self.active_integrations.write().await;
        integrations.remove(id);
    }

    pub async fn is_integration_active(&self, id: &IntegrationId) -> bool {
        let integrations = self.active_integrations.read().await;
        integrations.contains_key(id)
    }
} 