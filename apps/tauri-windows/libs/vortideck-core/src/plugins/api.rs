//! Plugin API interface
//!
//! This module defines the interface for plugin system.

use crate::plugins::{PluginError, PluginResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Plugin API types for future implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub plugin_id: String,
    pub permissions: HashMap<String, bool>,
}

// Placeholder for future plugin API implementation
pub struct PluginApi;

impl PluginApi {
    pub fn new() -> Self {
        Self
    }

    pub async fn load_plugin(&self, _path: &str) -> PluginResult<Box<dyn crate::plugins::Plugin>> {
        // Future implementation will load plugins using a plugin system
        Err(PluginError::LoadFailed(
            "Plugin loading not yet implemented".to_string(),
        ))
    }
}

/// HTTP request structure for plugin HTTP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    /// Request method (GET, POST, etc.)
    pub method: String,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: Vec<(String, String)>,
    /// Request body
    pub body: Option<Vec<u8>>,
}

/// HTTP response structure for plugin HTTP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// Response status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: Vec<u8>,
}
