//! VortiDeck Core Library
//!
//! This is the main headless core library for VortiDeck, providing a platform-agnostic
//! implementation of all core functionality.
//!
//! 🚀 UNIVERSAL ARCHITECTURE: Zero platform dependencies
//! All platform adapters (Tauri, CLI, Server) are implemented in the adapter layer
use crate::services::flow::traits::FlowNodeProvider;
use std::sync::Arc;

pub mod adapters;
pub mod core;
pub mod db;
pub mod events;
pub mod features;
pub mod integrations;
pub mod plugins;
pub mod proxy;
pub mod services;
pub mod storage;

// Re-export integration types
pub use integrations::{
    builtin::obs::ObsIntegration,
    framework::{
        IntegrationDiscovery, IntegrationFrameworkError, IntegrationFrameworkResult,
        IntegrationLifecycleManager, IntegrationManager, IntegrationRegistry, IntegrationValidator,
    },
    interface::{
        EventCallback, IntegrationAction, IntegrationActionResult, IntegrationActionStatus,
        IntegrationId, IntegrationMetadata, IntegrationRequirements, IntegrationResult,
    },
    traits::{AppIntegration, IntegrationError},
    CapabilityDefinition, CapabilityKind, CapabilityOption, CapabilityOutput, CapabilityParameter,
    CapabilityPresentation, CapabilityWidget, DynamicOptionSource,
};

// Re-export plugin types
pub use plugins::{
    interface::{Plugin, PluginAction, PluginError, PluginEvent, PluginMetadata, PluginTrait},
    manager::PluginManager,
};

// Re-export feature types
pub use features::{
    Feature, FeatureId, FeatureRegistry, FeatureRequirement, FeatureValidation, PremiumFeature,
    ResetPeriod,
};

// Re-export storage types
pub use storage::{Deck, DeckStorage, StorageError};

// Re-export service types (formerly modules)
pub use services::{
    // Auth service
    auth::{AuthError, AuthModule, User, UserPlan},

    // Execution service
    execution::{
        ActionInvocationService, ExecutionContext, ExecutionResult, InvocationResult,
        InvocationStatus, UniversalAction, UniversalExecutionEngine,
    },

    // Flow service
    flow::{FlowAction, FlowState},

    // Macro service
    macros::{MacroEngine, RecordedMacro},

    // Marketplace service
    marketplace::{MarketplaceError, MarketplacePlugin, PluginMarketplace},

    // Theme service
    themes::{Theme, ThemeError, ThemeManager},

    // UI service
    ui::{ButtonConfig, ButtonError, ButtonManager},
    // UI service - continued from above line
};

// System service (Windows only)
#[cfg(target_os = "windows")]
pub use services::system::SystemControl;

// Re-export commonly used types
pub use core::{
    ConnectionCandidate, ModuleState, PeerInfo, PluginPermissions, PublicState,
    SerializablePeerInfo, SerializableServerInfo, ServerInfo, StateManager, UIState,
};

/// Main VortiDeck Core struct that orchestrates all components
#[derive(Clone)]
pub struct VortiDeckCore {
    state_manager: Arc<StateManager>,
    plugin_manager: Arc<plugins::PluginManager>,
    integration_manager: Arc<integrations::IntegrationManager>,
    macro_engine: Arc<services::macros::MacroEngine>,
    platform_context: Arc<core::PlatformContext>,
}

impl VortiDeckCore {
    pub fn new(platform_context: Arc<core::PlatformContext>) -> Self {
        let state_manager = Arc::new(StateManager::new());

        Self {
            state_manager: state_manager.clone(),
            plugin_manager: Arc::new(plugins::PluginManager::new_with_context(
                platform_context.clone(),
            )),
            integration_manager: Arc::new(integrations::IntegrationManager::new()),
            macro_engine: Arc::new(services::macros::MacroEngine::new(state_manager.clone())),
            platform_context,
        }
    }

    pub fn state_manager(&self) -> Arc<StateManager> {
        self.state_manager.clone()
    }

    pub fn plugin_manager(&self) -> Arc<plugins::PluginManager> {
        self.plugin_manager.clone()
    }

    pub fn integration_manager(&self) -> Arc<integrations::IntegrationManager> {
        self.integration_manager.clone()
    }

    pub fn get_macro_engine(&self) -> Arc<services::macros::MacroEngine> {
        self.macro_engine.clone()
    }

    pub fn get_state_manager(&self) -> Arc<StateManager> {
        self.state_manager.clone()
    }

    pub fn platform_context(&self) -> Arc<core::PlatformContext> {
        self.platform_context.clone()
    }

    /// Get default plugins directory (NEW ARCHITECTURE)
    pub fn get_default_plugins_directory() -> std::path::PathBuf {
        crate::db::db::get_vortideck_data_root().join("plugins")
    }

    /// Scan plugins directory to see what plugins exist (separate from loading)
    pub fn scan_plugins_directory(
        &self,
        plugins_dir: &std::path::Path,
    ) -> Result<(usize, Vec<std::path::PathBuf>), String> {
        if !plugins_dir.exists() {
            println!("📁 Creating plugins directory: {:?}", plugins_dir);
            std::fs::create_dir_all(plugins_dir)
                .map_err(|e| format!("Failed to create plugins directory: {}", e))?;
            return Ok((0, Vec::new()));
        }

        let entries = std::fs::read_dir(plugins_dir)
            .map_err(|e| format!("Failed to read plugins directory: {}", e))?;

        let mut plugin_folders = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            // Only scan for plugin folders (directories with manifest.json)
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    plugin_folders.push(path);
                }
            }
            // Standalone .wasm and .vdext files are no longer supported
        }

        Ok((plugin_folders.len(), plugin_folders))
    }

    /// Classify a discovered package without executing it. Widget-only packages
    /// must remain discoverable even when the account's runtime-plugin limit is
    /// exhausted; limits govern executable runtimes, not manifest visibility.
    pub fn plugin_folder_has_runtime(plugin_folder: &std::path::Path) -> Result<bool, String> {
        let manifest_path = plugin_folder.join("manifest.json");
        let manifest_json = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read {}: {}", manifest_path.display(), e))?;
        let manifest: plugins::BundleManifest = serde_json::from_str(&manifest_json)
            .map_err(|e| format!("Failed to parse {}: {}", manifest_path.display(), e))?;
        let wasm_path = plugin_folder.join(manifest.get_wasm_path());
        Ok(wasm_path.exists() || manifest.get_service_config().is_some())
    }

    /// Get plugin count for validation and frontend display
    pub async fn get_plugin_count(&self) -> usize {
        self.plugin_manager.get_plugin_count().await
    }

    /// Get all plugin metadata for frontend display
    pub async fn get_plugin_metadata(&self) -> Vec<plugins::PluginMetadata> {
        self.plugin_manager.get_all_plugin_metadata().await
    }

    /// Initialize with plugin watcher for hot reload
    pub async fn start_plugin_watcher(
        &self,
    ) -> Result<plugins::PluginWatcher, Box<dyn std::error::Error + Send + Sync>> {
        let plugins_dir = Self::get_default_plugins_directory();
        println!(
            "🔥 VortiDeckCore: Starting plugin watcher for directory: {:?}",
            plugins_dir
        );

        // For now, create a simple watcher - later we can integrate with notify
        let watcher = plugins::PluginWatcher::new(plugins_dir)?;
        println!("✅ VortiDeckCore: Plugin watcher started successfully");

        Ok(watcher)
    }

    /// Get WASM flow provider for flow registry integration (SIMPLIFIED)
    pub async fn get_wasm_flow_provider(&self) -> plugins::WasmFlowNodeProvider {
        // Get all WASM plugins from the plugin manager
        let wasm_plugins = self.plugin_manager.get_wasm_plugins().await;

        // Create a flow provider with these plugins (same as old working system)
        plugins::WasmFlowNodeProvider::new(wasm_plugins)
    }

    /// Enable or disable a plugin
    pub async fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        self.plugin_manager
            .set_plugin_enabled(plugin_id, enabled)
            .await
    }

    /// Check if a plugin is enabled
    pub async fn is_plugin_enabled(&self, plugin_id: &str) -> Result<bool, String> {
        self.plugin_manager.is_plugin_enabled(plugin_id).await
    }
}

#[async_trait::async_trait]
impl IntegrationLifecycleManager for VortiDeckCore {
    async fn register_integration(
        &self,
        integration: Box<dyn AppIntegration>,
    ) -> IntegrationFrameworkResult<()> {
        self.integration_manager
            .register_integration(integration)
            .await
    }

    async fn unregister_integration(&self, id: IntegrationId) -> IntegrationFrameworkResult<()> {
        self.integration_manager.unregister_integration(id).await
    }

    async fn get_integration(&self, id: IntegrationId) -> Option<Box<dyn AppIntegration>> {
        self.integration_manager.get_integration(id).await
    }

    async fn list_integrations(&self) -> Vec<IntegrationMetadata> {
        self.integration_manager.list_integrations().await
    }
}

/// Error type for VortiDeck Core operations
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Integration error: {0}")]
    Integration(#[from] crate::integrations::traits::IntegrationError),
    #[error("Feature error: {0}")]
    Feature(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for VortiDeck Core operations
pub type CoreResult<T> = std::result::Result<T, CoreError>;
