//! Plugin system for VortiDeck
//!
//! This module provides a dual-runtime plugin system:
//! - WASM plugins (in-process, sandboxed via Extism)
//! - Service plugins (external process, JSON-RPC communication)

pub mod api;
pub mod bundle;
pub mod contributions;
pub mod error;
pub mod flow_integration;
pub mod integrity;
pub mod interface;
pub mod manager;
pub mod permissions;
pub mod service_plugin;
pub mod signing;
pub mod traits;
pub mod validation;
pub mod watcher;
pub mod widget_commands;
pub mod widget_settings;

use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

pub use api::PluginApi;
pub use bundle::{
    canonical_package_id, EntryConfig, PluginBundle, PluginManifest as BundleManifest,
    PluginPermission, PluginRuntime, PluginSignature, ServiceEntry, WebManifest, WebResources,
};
pub use contributions::{
    capabilities_to_permissions, compile_manifest_permissions, manifest_consent_hash, Contribution,
    NetworkCapability, PackageDiscoveryError, PackageInfo, WidgetCapabilities,
};
pub use error::PluginError;
pub use flow_integration::{WasmFlowNodeProvider, WasmFlowNodeWrapper};
pub use interface::{
    Plugin as PluginInterface, PluginAction, PluginError as InterfacePluginError, PluginEvent,
    PluginMetadata, PluginTrait,
};
pub use manager::{FlowRegistryProvider, PluginManager};
pub use permissions::EffectivePluginScope;
pub use service_plugin::ServicePlugin;
pub use traits::AsyncPlugin;
pub use validation::{validate_plugin_file, PluginFileType, ValidationError, ValidationResult};
pub use watcher::{PluginChangeEvent, PluginWatcher};

// Re-export the main Plugin trait
pub use interface::Plugin;

// Tauri dependency removed - plugin system is now platform-agnostic
// use tauri::Manager;

/// Plugin ID type
pub type PluginId = String;

/// Plugin result type
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub enabled: bool,
    pub settings: HashMap<String, String>,
}
