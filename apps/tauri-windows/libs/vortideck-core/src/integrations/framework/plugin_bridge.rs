//! Plugin-Integration Bridge
//!
//! This module provides the bridge between the plugin system and integration system,
//! allowing plugins to provide integrations in a sandboxed manner.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::integrations::{
    framework::{IntegrationFrameworkResult, IntegrationRegistry},
    interface::{IntegrationAction, IntegrationActionResult, IntegrationId, IntegrationMetadata},
    traits::{AppIntegration, IntegrationError},
    CapabilityDefinition,
};
use crate::plugins::{Plugin, PluginError, PluginId};

/// Represents the source of an integration
#[derive(Debug, Clone)]
pub enum IntegrationProvider {
    /// Native integration built into VortiDeck core
    Native(IntegrationMetadata),
    /// Integration provided by a plugin (sandboxed)
    Plugin {
        integration_meta: IntegrationMetadata,
        plugin_id: PluginId,
        plugin_meta: crate::plugins::PluginMetadata,
    },
}

/// Plugin-provided integration wrapper that enforces sandboxing
pub struct PluginIntegration {
    plugin_id: PluginId,
    integration_id: IntegrationId,
    metadata: IntegrationMetadata,
    /// Reference to the actual plugin that provides this integration
    plugin_ref: Arc<RwLock<Box<dyn Plugin>>>,
}

impl PluginIntegration {
    pub fn new(
        plugin_id: PluginId,
        integration_id: IntegrationId,
        metadata: IntegrationMetadata,
        plugin_ref: Arc<RwLock<Box<dyn Plugin>>>,
    ) -> Self {
        Self {
            plugin_id,
            integration_id,
            metadata,
            plugin_ref,
        }
    }
}

#[async_trait]
impl AppIntegration for PluginIntegration {
    fn id(&self) -> &IntegrationId {
        &self.integration_id
    }

    fn name(&self) -> &str {
        &self.metadata.name
    }

    fn description(&self) -> &str {
        &self.metadata.description
    }

    fn get_metadata(&self) -> IntegrationMetadata {
        self.metadata.clone()
    }

    fn get_requirements(&self) -> crate::integrations::interface::IntegrationRequirements {
        // Plugin integrations have basic requirements by default
        crate::integrations::interface::IntegrationRequirements {
            min_version: Some("0.1.0".to_string()),
            max_version: None,
            dependencies: vec!["plugin-system".to_string()],
        }
    }

    async fn is_available(&self) -> bool {
        // Check if the plugin is still loaded and functional
        // For now, assume plugin is available if we can access its metadata
        self.plugin_ref.read().await.metadata().id == self.plugin_id
    }

    async fn execute_action(
        &self,
        action: IntegrationAction,
    ) -> Result<IntegrationActionResult, IntegrationError> {
        // Validate action first
        self.validate_action(&action).await?;

        Err(IntegrationError::UnsupportedAction(format!(
            "Plugin integration '{}:{}' has no sandbox execution adapter",
            self.plugin_id, action.name
        )))
    }

    async fn subscribe_to_events(
        &self,
        _callback: Box<dyn crate::integrations::interface::EventCallback>,
    ) -> crate::integrations::traits::IntegrationResult<()> {
        Err(IntegrationError::UnsupportedAction(format!(
            "Plugin integration '{}' has no event subscription adapter",
            self.integration_id
        )))
    }

    async fn get_capabilities(&self) -> Vec<CapabilityDefinition> {
        // This legacy bridge has no action executor. An empty catalog is
        // truthful and prevents authoring a flow that can only fail later.
        Vec::new()
    }
}

impl PluginIntegration {
    /// Validate an action for plugin execution
    async fn validate_action(&self, action: &IntegrationAction) -> Result<(), IntegrationError> {
        // Plugin integrations have additional validation requirements
        if action.parameters.is_empty() {
            return Err(IntegrationError::InvalidParameters(
                "Plugin integrations require parameters".to_string(),
            ));
        }

        // Ensure the action is safe for plugin execution
        if action.name.contains("system_") || action.name.contains("internal_") {
            return Err(IntegrationError::ValidationError(
                "Plugin integrations cannot execute system actions".to_string(),
            ));
        }

        Ok(())
    }
}

/// Enhanced Integration Registry that supports plugin-provided integrations
pub struct HybridIntegrationRegistry {
    /// Native integrations (full access)
    native_integrations: Arc<RwLock<HashMap<IntegrationId, Box<dyn AppIntegration>>>>,
    /// Plugin-provided integrations (sandboxed)
    plugin_integrations: Arc<RwLock<HashMap<IntegrationId, PluginIntegration>>>,
    /// Mapping of plugin IDs to their provided integrations
    plugin_to_integrations: Arc<RwLock<HashMap<PluginId, Vec<IntegrationId>>>>,
}

impl HybridIntegrationRegistry {
    pub fn new() -> Self {
        Self {
            native_integrations: Arc::new(RwLock::new(HashMap::new())),
            plugin_integrations: Arc::new(RwLock::new(HashMap::new())),
            plugin_to_integrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a native integration (core team, full access)
    pub async fn register_native_integration(
        &self,
        integration: Box<dyn AppIntegration>,
    ) -> IntegrationFrameworkResult<()> {
        let integration_id = integration.get_metadata().id.clone();
        let mut native = self.native_integrations.write().await;

        if native.contains_key(&integration_id) {
            return Err(
                crate::integrations::framework::IntegrationFrameworkError::Error(format!(
                    "Native integration {} already registered",
                    integration_id
                )),
            );
        }

        native.insert(integration_id, integration);
        println!("✅ Native integration registered successfully");
        Ok(())
    }

    /// Register a plugin-provided integration (community, sandboxed)
    pub async fn register_plugin_integration(
        &self,
        plugin_id: PluginId,
        integration_id: IntegrationId,
        metadata: IntegrationMetadata,
        plugin_ref: Arc<RwLock<Box<dyn Plugin>>>,
    ) -> IntegrationFrameworkResult<()> {
        let mut plugins = self.plugin_integrations.write().await;
        let mut mappings = self.plugin_to_integrations.write().await;

        if plugins.contains_key(&integration_id) {
            return Err(
                crate::integrations::framework::IntegrationFrameworkError::Error(format!(
                    "Plugin integration {} already registered",
                    integration_id
                )),
            );
        }

        let plugin_integration = PluginIntegration::new(
            plugin_id.clone(),
            integration_id.clone(),
            metadata,
            plugin_ref,
        );

        plugins.insert(integration_id.clone(), plugin_integration);

        // Update plugin mapping
        mappings
            .entry(plugin_id.clone())
            .or_insert_with(Vec::new)
            .push(integration_id.clone());

        println!(
            "✅ Plugin integration registered: {} from plugin {}",
            integration_id, plugin_id
        );
        Ok(())
    }

    /// Unregister all integrations from a specific plugin
    pub async fn unregister_plugin_integrations(
        &self,
        plugin_id: &PluginId,
    ) -> IntegrationFrameworkResult<()> {
        let mut plugins = self.plugin_integrations.write().await;
        let mut mappings = self.plugin_to_integrations.write().await;

        if let Some(integration_ids) = mappings.remove(plugin_id) {
            for integration_id in integration_ids {
                plugins.remove(&integration_id);
                println!(
                    "🗑️ Removed plugin integration: {} from plugin {}",
                    integration_id, plugin_id
                );
            }
        }

        Ok(())
    }

    /// List all integrations with their provider information
    pub async fn list_integrations_with_providers(&self) -> Vec<IntegrationProvider> {
        let mut providers = Vec::new();

        // Add native integrations
        {
            let native = self.native_integrations.read().await;
            for integration in native.values() {
                providers.push(IntegrationProvider::Native(integration.get_metadata()));
            }
        }

        // Add plugin integrations
        {
            let plugins = self.plugin_integrations.read().await;
            for integration in plugins.values() {
                // Get plugin metadata
                let plugin = integration.plugin_ref.read().await;
                let plugin_meta = plugin.metadata();

                providers.push(IntegrationProvider::Plugin {
                    integration_meta: integration.get_metadata(),
                    plugin_id: integration.plugin_id.clone(),
                    plugin_meta,
                });
            }
        }

        providers
    }

    /// Get integration metadata by ID
    pub async fn get_integration_metadata(
        &self,
        integration_id: &IntegrationId,
    ) -> Option<IntegrationMetadata> {
        // Check native first
        {
            let native = self.native_integrations.read().await;
            if let Some(integration) = native.get(integration_id) {
                return Some(integration.get_metadata());
            }
        }

        // Check plugin integrations
        {
            let plugins = self.plugin_integrations.read().await;
            if let Some(integration) = plugins.get(integration_id) {
                return Some(integration.get_metadata());
            }
        }

        None
    }

    /// Check if an integration exists
    pub async fn has_integration(&self, integration_id: &IntegrationId) -> bool {
        let native = self.native_integrations.read().await;
        if native.contains_key(integration_id) {
            return true;
        }

        let plugins = self.plugin_integrations.read().await;
        plugins.contains_key(integration_id)
    }

    /// Get statistics about registered integrations
    pub async fn get_statistics(&self) -> IntegrationStatistics {
        let native_count = self.native_integrations.read().await.len();
        let plugin_count = self.plugin_integrations.read().await.len();
        let plugin_providers = self.plugin_to_integrations.read().await.len();

        IntegrationStatistics {
            native_integrations: native_count,
            plugin_integrations: plugin_count,
            total_integrations: native_count + plugin_count,
            active_plugin_providers: plugin_providers,
        }
    }
}

/// Statistics about the integration registry
#[derive(Debug, Clone)]
pub struct IntegrationStatistics {
    pub native_integrations: usize,
    pub plugin_integrations: usize,
    pub total_integrations: usize,
    pub active_plugin_providers: usize,
}

/// Trait for plugins that can provide integrations
#[async_trait]
pub trait PluginIntegrationProvider: Send + Sync {
    /// Get the integrations this plugin provides
    async fn get_provided_integrations(&self) -> Vec<IntegrationMetadata>;

    /// Initialize an integration provided by this plugin
    async fn initialize_integration(
        &self,
        integration_id: &IntegrationId,
    ) -> Result<(), PluginError>;

    /// Shutdown an integration provided by this plugin
    async fn shutdown_integration(&self, integration_id: &IntegrationId)
        -> Result<(), PluginError>;
}
