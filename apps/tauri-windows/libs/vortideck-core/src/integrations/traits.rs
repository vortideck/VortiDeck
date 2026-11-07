use crate::integrations::interface::{
    IntegrationAction, IntegrationActionResult, IntegrationEvent, IntegrationId,
    IntegrationMetadata, IntegrationRequirements,
};
use crate::integrations::{CapabilityDefinition, CapabilityOption, CapabilityParameter};
use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

// Re-export EventCallback from interface
pub use crate::integrations::interface::EventCallback;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

/// Error type for integration operations
#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("Integration not found: {0}")]
    NotFound(String),

    #[error("Integration not initialized: {0}")]
    NotInitialized(String),

    #[error("Integration not connected: {0}")]
    NotConnected(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid integration: {0}")]
    InvalidIntegration(String),

    #[error("Invalid action: {0}")]
    InvalidAction(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Action failed: {0}")]
    ActionFailed(String),

    #[error("Integration already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Integration error: {0}")]
    Error(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Unsupported action: {0}")]
    UnsupportedAction(String),
}

/// Result type for integration operations
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// Trait for integration implementations
#[async_trait]
pub trait AppIntegration: Send + Sync {
    /// Get the integration's ID
    fn id(&self) -> &IntegrationId;

    /// Get the integration's name
    fn name(&self) -> &str;

    /// Get the integration's description
    fn description(&self) -> &str;

    /// Get the integration metadata
    fn get_metadata(&self) -> IntegrationMetadata;

    /// Get the integration requirements
    fn get_requirements(&self) -> IntegrationRequirements;

    /// Check if the integration is available
    async fn is_available(&self) -> bool;

    /// Execute an action
    async fn execute_action(
        &self,
        action: IntegrationAction,
    ) -> IntegrationResult<IntegrationActionResult>;

    /// Subscribe to events
    async fn subscribe_to_events(&self, callback: Box<dyn EventCallback>) -> IntegrationResult<()>;

    /// Publish the provider's canonical capability definitions. Discovery,
    /// forms, validation, and execution all consume this same schema.
    async fn get_capabilities(&self) -> Vec<CapabilityDefinition>;

    /// Resolve a capability against current provider state and input values.
    /// Providers with parameter-dependent resource policies may override this.
    async fn resolve_capability(
        &self,
        capability_id: &str,
        parameters: &std::collections::HashMap<String, Value>,
    ) -> IntegrationResult<CapabilityDefinition> {
        let capability = self
            .get_capabilities()
            .await
            .into_iter()
            .find(|capability| capability.capability_id == capability_id)
            .ok_or_else(|| {
                IntegrationError::UnsupportedAction(format!(
                    "Integration '{}' does not publish capability '{}'",
                    self.id().as_str(),
                    capability_id
                ))
            })?;
        capability
            .validate_parameters(parameters)
            .map_err(IntegrationError::ValidationError)?;
        Ok(capability)
    }

    /// Resolve live choices for one input. The default implementation reads
    /// the provider's latest capability snapshot; providers may override it
    /// for remote, paginated, or parameter-dependent catalogs.
    async fn resolve_capability_options(
        &self,
        capability_id: &str,
        parameter_id: &str,
        _current_values: &std::collections::HashMap<String, Value>,
    ) -> IntegrationResult<Vec<CapabilityOption>> {
        let capability = self
            .get_capabilities()
            .await
            .into_iter()
            .find(|capability| capability.capability_id == capability_id)
            .ok_or_else(|| IntegrationError::UnsupportedAction(capability_id.to_string()))?;
        find_parameter(&capability.inputs, parameter_id)
            .map(|parameter| parameter.options.clone())
            .ok_or_else(|| IntegrationError::InvalidParameter(parameter_id.to_string()))
    }
}

fn find_parameter<'a>(
    parameters: &'a [CapabilityParameter],
    parameter_id: &str,
) -> Option<&'a CapabilityParameter> {
    for parameter in parameters {
        if parameter.id == parameter_id {
            return Some(parameter);
        }
        for option in &parameter.options {
            if let Some(found) = find_parameter(&option.parameters, parameter_id) {
                return Some(found);
            }
        }
    }
    None
}

/// Trait for features that require premium access
pub trait FeatureRequirement {
    /// Get the premium features required
    fn get_requirements(&self) -> Vec<crate::features::PremiumFeature>;

    /// Get the context for the feature requirement
    fn feature_context(&self) -> String;
}
