//! Plugin error types
//!
//! This module defines error types for plugin operations.

use super::PluginId;
use std::fmt;
use thiserror::Error;

/// Plugin error types
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Plugin already exists
    #[error("Plugin already exists: {0}")]
    AlreadyExists(String),

    /// Plugin initialization failed
    #[error("Initialization error: {0}")]
    Initialization(String),

    /// Plugin shutdown failed
    #[error("Shutdown error: {0}")]
    Shutdown(String),

    /// Plugin execution failed
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Invalid plugin
    #[error("Invalid plugin: {0}")]
    InvalidPlugin(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Plugin already registered
    #[error("Plugin already registered: {0}")]
    AlreadyRegistered(String),

    /// Plugin validation error
    #[error("Plugin validation error: {0}")]
    Validation(String),

    /// Plugin already active
    #[error("Plugin already active: {0}")]
    AlreadyActive(PluginId),

    /// Plugin permission denied
    #[error("Plugin permission denied: {0}")]
    PermissionDenied(String),

    /// WebAssembly module error
    #[error("WebAssembly module error: {0}")]
    WasmError(String),

    /// Function not found in plugin
    #[error("Function not found in plugin: {0}")]
    FunctionNotFound(String),

    /// Invalid function signature
    #[error("Invalid function signature: {0}")]
    InvalidSignature(String),

    /// Load failed
    #[error("Load failed: {0}")]
    LoadFailed(String),

    /// Initialization failed
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Dependency error
    #[error("Dependency error: {0}")]
    DependencyError(String),
}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;
