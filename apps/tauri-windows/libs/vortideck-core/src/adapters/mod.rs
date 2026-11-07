//! Adapters module for VortiDeck
//!
//! This module implements the adapter system for VortiDeck, providing
//! platform-specific implementations of core functionality.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Adapter error type
#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("Adapter not found: {0}")]
    NotFound(String),

    #[error("Adapter initialization failed: {0}")]
    InitError(String),

    #[error("Adapter operation failed: {0}")]
    OperationError(String),
}

/// Result type for adapter operations
pub type AdapterResult<T> = Result<T, AdapterError>;

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Type of adapter
    pub adapter_type: String,
    /// Platform-specific configuration
    pub platform_config: serde_json::Value,
}

/// Basic message sender trait - implementations are provided in platform adapters
#[async_trait]
pub trait BasicMessageSender: Send + Sync {
    async fn send_message(&self, message: String) -> Result<(), String>;
}

// Platform-specific adapters are implemented in the platform layer (src-tauri)
// Core library should not contain any platform-specific code

/// Adapter manager for VortiDeck
pub struct AdapterManager {
    // Adapter components will be added here as we migrate them
}

impl AdapterManager {
    /// Create a new adapter manager instance
    pub fn new() -> Self {
        Self {}
    }

    /// Initialize the adapter manager
    pub async fn init(&mut self, _config: AdapterConfig) -> AdapterResult<()> {
        // Implementation will be added later
        Ok(())
    }
}
