use async_trait::async_trait;
use thiserror::Error;

use crate::integrations::{
    interface::{IntegrationId, IntegrationMetadata, IntegrationRequirements, IntegrationResult},
    traits::{AppIntegration, IntegrationError},
};

#[derive(Debug, Error)]
pub enum IntegrationFrameworkError {
    #[error("Integration not found: {0}")]
    NotFound(String),
    #[error("Integration error: {0}")]
    Error(String),
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
}

pub type IntegrationFrameworkResult<T> = Result<T, IntegrationFrameworkError>;

pub mod discovery;
pub mod manager;
pub mod plugin_bridge;
pub mod registry;
pub mod validation;

pub use discovery::IntegrationDiscovery;
pub use manager::IntegrationManager;
pub use plugin_bridge::{
    HybridIntegrationRegistry, IntegrationProvider as IntegrationProviderEnum,
    IntegrationStatistics, PluginIntegration, PluginIntegrationProvider,
};
pub use registry::IntegrationRegistry;
pub use validation::IntegrationValidator;

#[async_trait]
pub trait IntegrationLifecycleManager: Send + Sync {
    async fn register_integration(
        &self,
        integration: Box<dyn AppIntegration>,
    ) -> IntegrationFrameworkResult<()>;
    async fn unregister_integration(&self, id: IntegrationId) -> IntegrationFrameworkResult<()>;
    async fn list_integrations(&self) -> Vec<IntegrationMetadata>;
    async fn get_integration(&self, id: IntegrationId) -> Option<Box<dyn AppIntegration>>;
}
