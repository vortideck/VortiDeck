use crate::integrations::{
    interface::{IntegrationRequirements, IntegrationResult},
    traits::{AppIntegration, IntegrationError},
    CapabilityDefinition, CapabilityParameter,
};

/// Validates integration actions and configurations
pub struct IntegrationValidator;

impl IntegrationValidator {
    /// Create a new integration validator
    pub fn new() -> Self {
        Self
    }

    /// Validate an integration action
    pub async fn validate_action(
        &self,
        action_name: &str,
        params: serde_json::Value,
    ) -> IntegrationResult<()> {
        // TODO: Implement action validation
        Ok(())
    }

    /// Validate integration configuration
    pub fn validate_configuration(&self, config: &serde_json::Value) -> IntegrationResult<()> {
        match config {
            serde_json::Value::Object(obj) => {
                // TODO: Add specific configuration validation rules
                Ok(())
            }
            _ => Err(IntegrationError::ValidationError(
                "Configuration must be an object".to_string(),
            )),
        }
    }

    /// Validate integration requirements
    pub fn validate_requirements(
        &self,
        _requirements: &IntegrationRequirements,
    ) -> IntegrationResult<()> {
        // TODO: Implement requirements validation
        Ok(())
    }

    /// Validate parameter definitions
    pub fn validate_parameters(parameters: &[CapabilityParameter]) -> IntegrationResult<()> {
        for param in parameters {
            if param.id.trim().is_empty() {
                return Err(IntegrationError::ValidationError(
                    "Capability parameter ID cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub async fn validate_integration(
        &self,
        integration: &dyn AppIntegration,
    ) -> IntegrationResult<()> {
        for capability in integration.get_capabilities().await {
            Self::validate_capability(&capability)?;
        }
        Ok(())
    }

    pub fn validate_capability(capability: &CapabilityDefinition) -> IntegrationResult<()> {
        if capability.integration_id.trim().is_empty() || capability.capability_id.trim().is_empty()
        {
            return Err(IntegrationError::ValidationError(format!(
                "Capability identity cannot be empty"
            )));
        }
        if capability.contract.integration_id != capability.integration_id
            || capability.contract.action_id != capability.capability_id
            || capability.contract.version != capability.version
        {
            return Err(IntegrationError::ValidationError(
                "Capability identity/version does not match its operational contract".to_string(),
            ));
        }
        Self::validate_parameters(&capability.inputs)
    }
}
